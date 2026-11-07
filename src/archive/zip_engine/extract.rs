use super::ZipEngine;
use crate::archive::entry::{compute_target_out_path, sanitize_archive_path};
use crate::archive::{ArchiveEntry, BUFFER_SIZE};
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

impl ZipEngine {
    pub(super) fn list(archive_path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>> {
        let file = File::open(archive_path).context("Failed to open zip file")?;

        match ZipArchive::new(file) {
            Ok(mut zip) => {
                let mut entries = Vec::with_capacity(zip.len());
                let mut is_encrypted_archive = false;

                for i in 0..zip.len() {
                    let file_res = match password {
                        Some(pwd) => zip.by_index_decrypt(i, pwd.as_bytes()),
                        None => zip.by_index_raw(i),
                    };

                    match file_res {
                        Ok(zip_file) => {
                            let raw_name = zip_file.name().to_string();
                            let is_dir = zip_file.is_dir()
                                || raw_name.ends_with('/')
                                || raw_name.ends_with('\\');

                            let mut entry = ArchiveEntry::new(raw_name, zip_file.size(), is_dir);
                            entry.compressed_size = Some(zip_file.compressed_size());

                            if zip_file.encrypted() {
                                is_encrypted_archive = true;
                            }

                            entries.push(entry);
                        }
                        Err(zip::result::ZipError::UnsupportedArchive(msg)) => {
                            if msg.to_lowercase().contains("password")
                                || msg.to_lowercase().contains("encrypt")
                            {
                                is_encrypted_archive = true;
                            }
                        }
                        Err(e) => {
                            let err_str = e.to_string().to_lowercase();
                            if err_str.contains("password") || err_str.contains("encrypt") {
                                is_encrypted_archive = true;
                            }
                        }
                    }
                }

                if is_encrypted_archive && password.is_none() {
                    anyhow::bail!("Archive is encrypted. Password required.");
                }

                if entries.is_empty() {
                    return Self::scan_headers(archive_path);
                }

                Ok(entries)
            }
            Err(_) => Self::scan_headers(archive_path),
        }
    }

    pub(super) fn extract(
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
        password: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        let file = File::open(archive_path)?;
        fs::create_dir_all(dest_dir)?;

        if let Ok(mut zip) = ZipArchive::new(file) {
            let mut extracted = Vec::with_capacity(zip.len());
            for i in 0..zip.len() {
                let zip_file_res = match password {
                    Some(pwd) => zip.by_index_decrypt(i, pwd.as_bytes()),
                    None => zip.by_index(i),
                };

                let mut zip_file = match zip_file_res {
                    Ok(file) => file,
                    Err(_) => continue,
                };

                let safe_relative_path = match zip_file.enclosed_name() {
                    Some(path) => path.to_path_buf(),
                    None => match sanitize_archive_path(Path::new(zip_file.name())) {
                        Some(p) => p,
                        None => continue,
                    },
                };

                let should_extract = entry_paths.is_empty()
                    || entry_paths
                        .iter()
                        .any(|p| safe_relative_path.starts_with(p) || &safe_relative_path == p);

                if !should_extract {
                    continue;
                }

                let out_path = compute_target_out_path(dest_dir, entry_paths, &safe_relative_path);

                if zip_file.is_dir() {
                    let _ = fs::create_dir_all(&out_path);
                } else {
                    if let Some(parent) = out_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    if let Ok(out_file) = File::create(&out_path) {
                        let mut buffered_out = BufWriter::with_capacity(BUFFER_SIZE, out_file);
                        let _ = std::io::copy(&mut zip_file, &mut buffered_out);
                        let _ = buffered_out.flush();
                    }
                }

                extracted.push(out_path);
            }
            return Ok(extracted);
        }

        Self::extract_fallback(archive_path, entry_paths, dest_dir)
    }
}
