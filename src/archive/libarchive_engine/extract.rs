use crate::archive::entry::compute_target_out_path;
use crate::archive::zip_engine::ZipEngine;
use crate::archive::ArchiveEngine;
use anyhow::{Context, Result};
use compress_tools::{uncompress_archive, Ownership};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn extract_entries(
    archive_path: &Path,
    entry_paths: &[PathBuf],
    dest_dir: &Path,
) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dest_dir)?;

    if entry_paths.is_empty() {
        let mut file = File::open(archive_path).context("Failed to open archive")?;
        uncompress_archive(&mut file, dest_dir, Ownership::Ignore)?;

        let extracted = fs::read_dir(dest_dir)?
            .filter_map(|e| e.ok().map(|entry| entry.path()))
            .collect();
        return Ok(extracted);
    }

    let mut cmd = Command::new("bsdtar");
    cmd.arg("-xf")
        .arg(archive_path)
        .arg("-C")
        .arg(dest_dir)
        .args(entry_paths);

    if let Ok(status) = cmd.status() {
        if status.success() || status.code().unwrap_or(0) == 1 {
            let extracted: Vec<PathBuf> = entry_paths
                .iter()
                .map(|path| dest_dir.join(path))
                .filter(|out| out.exists())
                .collect();
            if !extracted.is_empty() {
                return Ok(extracted);
            }
        }
    }

    if let Ok(extracted) = ZipEngine.extract_entries(archive_path, entry_paths, dest_dir) {
        if !extracted.is_empty() {
            return Ok(extracted);
        }
    }

    let mut extracted = Vec::new();
    for path in entry_paths {
        let path_str = path.to_string_lossy();
        let out = compute_target_out_path(dest_dir, entry_paths, path);
        if let Some(parent) = out.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(source) = File::open(archive_path) {
            if let Ok(mut out_file) = File::create(&out) {
                if compress_tools::uncompress_archive_file(source, &mut out_file, &path_str).is_ok()
                {
                    extracted.push(out);
                }
            }
        }
    }
    if !extracted.is_empty() {
        return Ok(extracted);
    }

    anyhow::bail!("Failed to extract selective entries via Libarchive")
}

pub fn extract_entries_with_password(
    archive_path: &Path,
    entry_paths: &[PathBuf],
    dest_dir: &Path,
    password: Option<&str>,
) -> Result<Vec<PathBuf>> {
    if let Ok(res) =
        ZipEngine.extract_entries_with_password(archive_path, entry_paths, dest_dir, password)
    {
        if !res.is_empty() {
            return Ok(res);
        }
    }
    extract_entries(archive_path, entry_paths, dest_dir)
}
