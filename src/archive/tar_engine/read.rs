use super::TarCompression;
use crate::archive::entry::{compute_target_out_path, sanitize_archive_path};
use crate::archive::{ArchiveEntry, IntegrityReport, BUFFER_SIZE};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use tar::Archive;

pub fn open_reader<'a>(compression: &TarCompression, file: File) -> Result<Box<dyn Read + 'a>> {
    let buffered = BufReader::with_capacity(BUFFER_SIZE, file);
    match compression {
        TarCompression::None => Ok(Box::new(buffered)),
        TarCompression::Gzip => Ok(Box::new(GzDecoder::new(buffered))),
        TarCompression::Zstd => Ok(Box::new(zstd::Decoder::new(buffered)?)),
    }
}

fn is_unsafe_link<R: Read>(entry: &tar::Entry<R>) -> bool {
    let entry_type = entry.header().entry_type();
    if !entry_type.is_symlink() && !entry_type.is_hard_link() {
        return false;
    }

    match entry.link_name() {
        Ok(Some(target)) => {
            target.is_absolute()
                || target
                    .components()
                    .any(|c| matches!(c, Component::ParentDir))
        }
        _ => false,
    }
}

pub fn list_entries(
    compression: &TarCompression,
    archive_path: &Path,
) -> Result<Vec<ArchiveEntry>> {
    let file = File::open(archive_path).context("Failed to open tar file")?;
    let reader = open_reader(compression, file)?;
    let mut archive = Archive::new(reader);

    archive
        .entries()?
        .map(|entry| {
            let entry = entry?;
            let path = entry.path()?.to_path_buf();
            let size = entry.size();
            let is_dir = entry.header().entry_type().is_dir();
            Ok(ArchiveEntry::new(path, size, is_dir))
        })
        .collect()
}

pub fn extract_entries(
    compression: &TarCompression,
    archive_path: &Path,
    entry_paths: &[PathBuf],
    dest_dir: &Path,
) -> Result<Vec<PathBuf>> {
    let file = File::open(archive_path)?;
    let reader = open_reader(compression, file)?;
    let mut archive = Archive::new(reader);
    let mut extracted = Vec::new();

    fs::create_dir_all(dest_dir)?;

    for entry_res in archive.entries()? {
        let mut entry = entry_res?;
        let raw_path = entry.path()?.to_path_buf();

        let safe_path = match sanitize_archive_path(&raw_path) {
            Some(p) => p,
            None => continue,
        };

        if is_unsafe_link(&entry) {
            tracing::warn!(
                "Skipping unsafe link in TAR archive: {}",
                safe_path.display()
            );
            continue;
        }

        let should_extract =
            entry_paths.is_empty() || entry_paths.iter().any(|p| safe_path.starts_with(p));

        if should_extract {
            let out_path = compute_target_out_path(dest_dir, entry_paths, &safe_path);
            if entry_paths.is_empty() {
                entry.unpack_in(dest_dir)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                entry.unpack(&out_path)?;
            }
            extracted.push(out_path);
        }
    }

    Ok(extracted)
}

pub fn verify_integrity(
    compression: &TarCompression,
    archive_path: &Path,
) -> Result<IntegrityReport> {
    let file = File::open(archive_path)?;
    let reader = open_reader(compression, file)?;
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
                failed_files.push((
                    format!("Entry #{}", total_files),
                    format!("Corrupt header: {:?}", err),
                ));
            }
        }
    }

    Ok(IntegrityReport {
        total_files,
        ok_files,
        failed_files,
    })
}
