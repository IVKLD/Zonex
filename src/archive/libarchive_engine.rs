use super::{ArchiveEngine, ArchiveEntry};
use anyhow::{Context, Result};
use compress_tools::{list_archive_files, uncompress_archive, Ownership};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct LibarchiveEngine;

impl ArchiveEngine for LibarchiveEngine {
    /// Перечисляет файлы архива через libarchive.
    /// Для zip-файлов дополнительно получает размеры через ZipArchive
    /// (compress_tools::list_archive_files возвращает только имена).
    fn list_entries(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>> {
        let mut file = File::open(archive_path).context("Failed to open archive for libarchive")?;
        let file_list = list_archive_files(&mut file).context("Failed to list files with libarchive")?;

        // Пробуем получить размеры через ZipArchive (для zip-форматов)
        // Это быстро — ZipArchive читает только Central Directory, без распаковки
        let zip_sizes: Option<std::collections::HashMap<String, u64>> = {
            let zip_file = File::open(archive_path).ok();
            zip_file.and_then(|f| {
                zip::ZipArchive::new(f).ok().map(|mut zip| {
                    let mut map = std::collections::HashMap::new();
                    for i in 0..zip.len() {
                        if let Ok(entry) = zip.by_index(i) {
                            map.insert(entry.name().to_string(), entry.size());
                        }
                    }
                    map
                })
            })
        };

        // Для tar-форматов пробуем tar crate
        let tar_sizes: Option<std::collections::HashMap<String, u64>> = if zip_sizes.is_none() {
            try_get_tar_sizes(archive_path)
        } else {
            None
        };

        let mut entries = Vec::new();
        for name in file_list {
            let is_dir = name.ends_with('/') || name.ends_with('\\');

            // Пытаемся получить размер из любого доступного источника
            let size = if let Some(ref sizes) = zip_sizes {
                sizes.get(&name).copied().unwrap_or(0)
            } else if let Some(ref sizes) = tar_sizes {
                sizes.get(name.trim_end_matches('/')).copied().unwrap_or(0)
            } else {
                0
            };

            let entry = ArchiveEntry::new(name, size, is_dir);
            entries.push(entry);
        }

        Ok(entries)
    }

    fn extract_entries(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        fs::create_dir_all(dest_dir)?;

        // Try bsdtar for fast selective extraction of exact entries
        if !entry_paths.is_empty() {
            let mut cmd = Command::new("bsdtar");
            cmd.arg("-xf")
                .arg(archive_path)
                .arg("-C")
                .arg(dest_dir);

            for path in entry_paths {
                cmd.arg(path);
            }

            if let Ok(status) = cmd.status() {
                if status.success() || status.code().unwrap_or(0) == 1 {
                    let mut extracted = Vec::new();
                    for path in entry_paths {
                        let out = dest_dir.join(path);
                        if out.exists() {
                            extracted.push(out);
                        }
                    }
                    if !extracted.is_empty() {
                        return Ok(extracted);
                    }
                }
            }
        }

        // Fallback to ZipEngine if bsdtar failed or was not available
        use crate::archive::zip_engine::ZipEngine;
        if let Ok(extracted) = ZipEngine.extract_entries(archive_path, entry_paths, dest_dir) {
            if !extracted.is_empty() {
                return Ok(extracted);
            }
        }

        // Final fallback: uncompress_archive only if entry_paths was empty
        if entry_paths.is_empty() {
            let mut file = File::open(archive_path).context("Failed to open archive")?;
            uncompress_archive(&mut file, dest_dir, Ownership::Ignore)?;

            let mut extracted = Vec::new();
            if let Ok(dir_entries) = fs::read_dir(dest_dir) {
                for entry in dir_entries.filter_map(|e| e.ok()) {
                    extracted.push(entry.path());
                }
            }
            return Ok(extracted);
        }

        anyhow::bail!("Failed to extract selective entries via Libarchive")
    }

    fn create_archive(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        use crate::archive::zip_engine::ZipEngine;
        ZipEngine.create_archive(archive_path, source_paths)
    }

    fn add_entries(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        use crate::archive::zip_engine::ZipEngine;
        ZipEngine.add_entries(archive_path, source_paths)
    }
}

/// Пробует получить размеры файлов из TAR-архива (tar, tar.gz, tar.zst, tar.bz2...).
/// Возвращает None если формат не TAR или произошла ошибка.
fn try_get_tar_sizes(archive_path: &Path) -> Option<std::collections::HashMap<String, u64>> {
    use std::io::BufReader;

    let try_plain = || -> Option<std::collections::HashMap<String, u64>> {
        let mut archive = tar::Archive::new(BufReader::new(File::open(archive_path).ok()?));
        let mut map = std::collections::HashMap::new();
        for entry in archive.entries().ok()? {
            let entry = entry.ok()?;
            let path = entry.path().ok()?.to_string_lossy().to_string();
            map.insert(path, entry.size());
        }
        Some(map)
    };

    if let Some(m) = try_plain() {
        return Some(m);
    }

    // Пробуем gzip
    let try_gz = || -> Option<std::collections::HashMap<String, u64>> {
        let file = File::open(archive_path).ok()?;
        let gz = flate2::read::GzDecoder::new(BufReader::new(file));
        let mut archive = tar::Archive::new(gz);
        let mut map = std::collections::HashMap::new();
        for entry in archive.entries().ok()? {
            let entry = entry.ok()?;
            let path = entry.path().ok()?.to_string_lossy().to_string();
            map.insert(path, entry.size());
        }
        Some(map)
    };

    if let Some(m) = try_gz() {
        return Some(m);
    }

    // Пробуем zstd
    let try_zst = || -> Option<std::collections::HashMap<String, u64>> {
        let file = File::open(archive_path).ok()?;
        let zst = zstd::Decoder::new(BufReader::new(file)).ok()?;
        let mut archive = tar::Archive::new(zst);
        let mut map = std::collections::HashMap::new();
        for entry in archive.entries().ok()? {
            let entry = entry.ok()?;
            let path = entry.path().ok()?.to_string_lossy().to_string();
            map.insert(path, entry.size());
        }
        Some(map)
    };

    try_zst()
}
