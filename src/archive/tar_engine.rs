use super::{ArchiveEngine, ArchiveEntry};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use tar::{Archive, Builder};
use walkdir::WalkDir;

const BUFFER_SIZE: usize = 128 * 1024; // 128 KB buffer

pub enum TarCompression {
    None,
    Gzip,
    Zstd,
}

pub struct TarEngine {
    pub compression: TarCompression,
}

impl TarEngine {
    pub fn new(compression: TarCompression) -> Self {
        Self { compression }
    }

    fn open_reader<'a>(&self, file: File) -> Result<Box<dyn Read + 'a>> {
        let buffered = BufReader::with_capacity(BUFFER_SIZE, file);
        match self.compression {
            TarCompression::None => Ok(Box::new(buffered)),
            TarCompression::Gzip => Ok(Box::new(GzDecoder::new(buffered))),
            TarCompression::Zstd => Ok(Box::new(zstd::Decoder::new(buffered)?)),
        }
    }
}

impl ArchiveEngine for TarEngine {
    fn list_entries(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>> {
        let file = File::open(archive_path).context("Failed to open tar file")?;
        let reader = self.open_reader(file)?;
        let mut archive = Archive::new(reader);

        let mut entries = Vec::new();
        for entry in archive.entries()? {
            let entry = entry?;
            let path = entry.path()?.to_path_buf();
            let size = entry.size();
            let is_dir = entry.header().entry_type().is_dir();
            entries.push(ArchiveEntry::new(path, size, is_dir));
        }

        Ok(entries)
    }

    fn extract_entries(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        let file = File::open(archive_path)?;
        let reader = self.open_reader(file)?;
        let mut archive = Archive::new(reader);
        let mut extracted = Vec::new();

        fs::create_dir_all(dest_dir)?;

        for entry_res in archive.entries()? {
            let mut entry = entry_res?;
            let raw_path = entry.path()?.to_path_buf();

            let safe_path = raw_path
                .components()
                .filter(|c| matches!(c, Component::Normal(_)))
                .collect::<PathBuf>();

            if safe_path.as_os_str().is_empty() {
                continue;
            }

            let should_extract = entry_paths.is_empty()
                || entry_paths
                    .iter()
                    .any(|p| safe_path.starts_with(p) || &safe_path == p);

            if should_extract {
                let out_path = dest_dir.join(&safe_path);
                entry.unpack_in(dest_dir)?;
                extracted.push(out_path);
            }
        }

        Ok(extracted)
    }

    fn create_archive(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        let file = File::create(archive_path)?;
        let buffered_file = BufWriter::with_capacity(BUFFER_SIZE, file);

        let writer: Box<dyn Write> = match self.compression {
            TarCompression::None => Box::new(buffered_file),
            TarCompression::Gzip => Box::new(GzEncoder::new(buffered_file, Compression::fast())),
            TarCompression::Zstd => Box::new(zstd::Encoder::new(buffered_file, 1)?.auto_finish()),
        };

        let mut builder = Builder::new(writer);

        for src in source_paths {
            if src.is_dir() {
                for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();
                    let name = path.strip_prefix(src.parent().unwrap_or(src))?;
                    if path.is_dir() {
                        builder.append_dir(name, path)?;
                    } else {
                        builder.append_path_with_name(path, name)?;
                    }
                }
            } else if src.is_file() {
                let name = src.file_name().unwrap_or(src.as_os_str());
                builder.append_path_with_name(src, name)?;
            }
        }

        builder.into_inner()?.flush()?;
        Ok(())
    }

    fn add_entries(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let temp_extracted = temp_dir.path().join("existing");

        if archive_path.exists() {
            let _ = self.extract_entries(archive_path, &[], &temp_extracted);
        }

        for src in source_paths {
            if let Some(name) = src.file_name() {
                let target_in_temp = temp_extracted.join(name);
                if target_in_temp.exists() {
                    if target_in_temp.is_dir() {
                        let _ = fs::remove_dir_all(&target_in_temp);
                    } else {
                        let _ = fs::remove_file(&target_in_temp);
                    }
                }
            }
        }

        let mut all_sources = Vec::new();
        if temp_extracted.exists() {
            for entry in fs::read_dir(&temp_extracted)? {
                all_sources.push(entry?.path());
            }
        }
        all_sources.extend(source_paths.iter().cloned());

        self.create_archive(archive_path, &all_sources)?;
        Ok(())
    }

    fn extract_entries_with_password(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
        _password: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        self.extract_entries(archive_path, entry_paths, dest_dir)
    }

    fn delete_entries(&self, archive_path: &Path, entry_paths: &[PathBuf]) -> Result<()> {
        if entry_paths.is_empty() || !archive_path.exists() {
            return Ok(());
        }

        let temp_dir = tempfile::tempdir()?;
        let temp_extracted = temp_dir.path().join("existing");

        let _ = self.extract_entries(archive_path, &[], &temp_extracted);

        let delete_targets: Vec<String> = entry_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        for target in delete_targets {
            let target_path = temp_extracted.join(&target);
            if target_path.exists() {
                if target_path.is_dir() {
                    let _ = fs::remove_dir_all(&target_path);
                } else {
                    let _ = fs::remove_file(&target_path);
                }
            }
        }

        let mut remaining_sources = Vec::new();
        if temp_extracted.exists() {
            for entry in fs::read_dir(&temp_extracted)? {
                remaining_sources.push(entry?.path());
            }
        }

        self.create_archive(archive_path, &remaining_sources)?;
        Ok(())
    }

    fn verify_integrity(
        &self,
        archive_path: &Path,
        _password: Option<&str>,
    ) -> Result<super::IntegrityReport> {
        let file = File::open(archive_path)?;
        let reader = self.open_reader(file)?;
        let mut archive = Archive::new(reader);

        let mut total_files = 0;
        let mut ok_files = 0;
        let mut failed_files = Vec::new();
        let mut sink = std::io::sink();

        for entry_res in archive.entries()? {
            total_files += 1;
            match entry_res {
                Ok(mut entry) => {
                    let path_str = entry
                        .path()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| format!("Entry #{}", total_files));
                    if std::io::copy(&mut entry, &mut sink).is_ok() {
                        ok_files += 1;
                    } else {
                        failed_files.push((path_str, "Corruption or read failure".to_string()));
                    }
                }
                Err(err) => {
                    failed_files.push((format!("Entry #{}", total_files), format!("Corrupt header: {:?}", err)));
                }
            }
        }

        Ok(super::IntegrityReport {
            total_files,
            ok_files,
            failed_files,
        })
    }
}
