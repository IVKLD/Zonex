use super::ZipEngine;
use crate::archive::BUFFER_SIZE;
use anyhow::{anyhow, Result};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Seek, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

fn write_file_entry<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    path: &Path,
    name: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    zip.start_file(name, options)?;
    let f = File::open(path)?;
    let mut buffered_in = BufReader::with_capacity(BUFFER_SIZE, f);
    std::io::copy(&mut buffered_in, zip)?;
    Ok(())
}

fn write_sources<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    source_paths: &[PathBuf],
    options: SimpleFileOptions,
) -> Result<()> {
    for src in source_paths {
        if src.is_dir() {
            for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = path.strip_prefix(src.parent().unwrap_or(src))?;
                let name_str = name.to_string_lossy().replace('\\', "/");
                if path.is_file() {
                    write_file_entry(zip, path, &name_str, options)?;
                } else if !name.as_os_str().is_empty() {
                    zip.add_directory(&name_str, options)?;
                }
            }
        } else if src.is_file() {
            let name = src.file_name().ok_or_else(|| anyhow!("Invalid path"))?;
            let name_str = name.to_string_lossy().replace('\\', "/");
            write_file_entry(zip, src, &name_str, options)?;
        }
    }
    Ok(())
}

impl ZipEngine {
    pub(super) fn create(archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        if let Some(parent) = archive_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(archive_path)?;
        let buffered_file = BufWriter::with_capacity(BUFFER_SIZE, file);
        let mut zip = ZipWriter::new(buffered_file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        write_sources(&mut zip, source_paths, options)?;
        zip.finish()?;
        Ok(())
    }

    pub(super) fn append(archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        if !archive_path.exists() {
            return Self::create(archive_path, source_paths);
        }

        let src_file = File::open(archive_path)?;
        let mut zip_in = match ZipArchive::new(src_file) {
            Ok(zip) => zip,
            Err(err) => {
                tracing::warn!(
                    "ZipArchive::new failed for {} ({:?}). Cannot append to invalid ZIP file directly.",
                    archive_path.display(),
                    err
                );
                return Err(anyhow!(
                    "Cannot append entries to invalid ZIP file: {:?}",
                    err
                ));
            }
        };

        let parent = archive_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_file = NamedTempFile::new_in(parent)?;
        let mut zip_out = ZipWriter::new(temp_file);

        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let new_names: Vec<String> = source_paths
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().replace('\\', "/"))
            .collect();

        for i in 0..zip_in.len() {
            let zip_file = match zip_in.by_index(i) {
                Ok(file) => file,
                Err(_) => continue,
            };

            let safe_name = zip_file
                .enclosed_name()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| zip_file.name().replace('\\', "/"));

            let is_overwritten = new_names.iter().any(|name| {
                safe_name == *name
                    || safe_name
                        .strip_prefix(name.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
            });

            if !is_overwritten {
                let _ = zip_out.raw_copy_file(zip_file);
            }
        }

        write_sources(&mut zip_out, source_paths, options)?;
        let temp_file = zip_out.finish()?;
        temp_file.persist(archive_path)?;
        Ok(())
    }
}
