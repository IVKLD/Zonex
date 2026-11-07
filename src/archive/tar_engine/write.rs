use super::read::extract_entries;
use super::TarCompression;
use crate::archive::BUFFER_SIZE;
use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use tar::Builder;
use walkdir::WalkDir;

fn remove_path(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

fn collect_dir_paths(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|entry| entry.path()))
        .collect()
}

pub fn create_archive(
    compression: &TarCompression,
    archive_path: &Path,
    source_paths: &[PathBuf],
) -> Result<()> {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(archive_path)?;
    let buffered_file = BufWriter::with_capacity(BUFFER_SIZE, file);

    let writer: Box<dyn Write> = match compression {
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

pub fn add_entries(
    compression: &TarCompression,
    archive_path: &Path,
    source_paths: &[PathBuf],
) -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let temp_extracted = temp_dir.path().join("existing");

    if archive_path.exists() {
        let _ = extract_entries(compression, archive_path, &[], &temp_extracted);
    }

    for src in source_paths {
        if let Some(name) = src.file_name() {
            remove_path(&temp_extracted.join(name));
        }
    }

    let mut all_sources = collect_dir_paths(&temp_extracted);
    all_sources.extend_from_slice(source_paths);

    create_archive(compression, archive_path, &all_sources)?;
    Ok(())
}

pub fn delete_entries(
    compression: &TarCompression,
    archive_path: &Path,
    entry_paths: &[PathBuf],
) -> Result<()> {
    if entry_paths.is_empty() || !archive_path.exists() {
        return Ok(());
    }

    let temp_dir = tempfile::tempdir()?;
    let temp_extracted = temp_dir.path().join("existing");

    let _ = extract_entries(compression, archive_path, &[], &temp_extracted);

    for target in entry_paths {
        let clean = target.to_string_lossy();
        let target_path = temp_extracted.join(clean.trim_end_matches('/'));
        remove_path(&target_path);
    }

    let remaining_sources = collect_dir_paths(&temp_extracted);
    create_archive(compression, archive_path, &remaining_sources)?;
    Ok(())
}
