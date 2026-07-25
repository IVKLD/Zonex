use super::{ArchiveEngine, ArchiveEntry};
use anyhow::{anyhow, Context, Result};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::unstable::write::FileOptionsExt;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

const BUFFER_SIZE: usize = 128 * 1024; // 128 KB buffer for optimal streaming I/O

fn compute_target_out_path(dest_dir: &Path, entry_paths: &[PathBuf], safe_relative_path: &Path) -> PathBuf {
    if entry_paths.is_empty() {
        return dest_dir.join(safe_relative_path);
    }

    for p in entry_paths {
        if safe_relative_path == p {
            if let Some(file_name) = p.file_name() {
                return dest_dir.join(file_name);
            }
        } else if safe_relative_path.starts_with(p) {
            if let (Some(parent_name), Ok(rel_sub)) = (p.file_name(), safe_relative_path.strip_prefix(p)) {
                return dest_dir.join(parent_name).join(rel_sub);
            }
        }
    }

    dest_dir.join(safe_relative_path)
}

pub struct ZipEngine;

impl ZipEngine {
    fn find_next_zip_header(&self, file: &mut File, start_pos: u64, file_len: u64) -> Result<u64> {
        use std::io::SeekFrom;

        let mut pos = start_pos;
        let mut buf = [0u8; 65536];

        while pos + 4 <= file_len {
            file.seek(SeekFrom::Start(pos))?;
            let bytes_read = match file.read(&mut buf) {
                Ok(n) if n >= 4 => n,
                _ => break,
            };

            for i in 0..bytes_read.saturating_sub(3) {
                if (buf[i] == 0x50 && buf[i + 1] == 0x4B)
                    && ((buf[i + 2] == 0x03 && buf[i + 3] == 0x04)
                        || (buf[i + 2] == 0x01 && buf[i + 3] == 0x02))
                {
                    return Ok(pos + i as u64);
                }
            }

            if bytes_read <= 3 {
                break;
            }
            pos += (bytes_read - 3) as u64;
        }

        Ok(file_len)
    }

    /// Resilient local header scanner that reads zip entries from offset 0
    /// using seeking without allocating the entire archive in RAM (0 OOM risk).
    fn scan_local_headers(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>> {
        use std::io::SeekFrom;

        let mut file = File::open(archive_path)?;
        let file_len = file.metadata()?.len();
        let mut entries = Vec::new();
        let mut pos: u64 = 0;

        let mut header_buf = [0u8; 30];

        while pos + 30 <= file_len {
            file.seek(SeekFrom::Start(pos))?;
            if file.read_exact(&mut header_buf).is_err() {
                break;
            }

            if header_buf[0..4] == [0x50, 0x4B, 0x03, 0x04] {
                let flags = u16::from_le_bytes([header_buf[6], header_buf[7]]);
                let has_data_descriptor = (flags & 0x0008) != 0;

                let comp_size = u32::from_le_bytes([
                    header_buf[18],
                    header_buf[19],
                    header_buf[20],
                    header_buf[21],
                ]) as u64;
                let uncomp_size = u32::from_le_bytes([
                    header_buf[22],
                    header_buf[23],
                    header_buf[24],
                    header_buf[25],
                ]) as u64;
                let name_len =
                    u16::from_le_bytes([header_buf[26], header_buf[27]]) as usize;
                let extra_len =
                    u16::from_le_bytes([header_buf[28], header_buf[29]]) as usize;

                if pos + 30 + (name_len as u64) > file_len {
                    break;
                }

                let mut name_bytes = vec![0u8; name_len];
                if file.read_exact(&mut name_bytes).is_ok() {
                    let name = String::from_utf8_lossy(&name_bytes).to_string();
                    if !name.is_empty() {
                        let is_dir = name.ends_with('/') || name.ends_with('\\');
                        let mut entry = ArchiveEntry::new(name, uncomp_size, is_dir);
                        entry.compressed_size = Some(comp_size);
                        entries.push(entry);
                    }
                }

                let data_start = pos + 30 + (name_len as u64) + (extra_len as u64);
                if comp_size > 0 && !has_data_descriptor {
                    pos = data_start.saturating_add(comp_size);
                } else {
                    pos = self.find_next_zip_header(&mut file, data_start, file_len)?;
                }
            } else if header_buf[0..4] == [0x50, 0x4B, 0x01, 0x02] {
                let mut cd_buf = [0u8; 46];
                file.seek(SeekFrom::Start(pos))?;
                if file.read_exact(&mut cd_buf).is_ok() {
                    let comp_size = u32::from_le_bytes([cd_buf[20], cd_buf[21], cd_buf[22], cd_buf[23]]) as u64;
                    let uncomp_size = u32::from_le_bytes([cd_buf[24], cd_buf[25], cd_buf[26], cd_buf[27]]) as u64;
                    let name_len = u16::from_le_bytes([cd_buf[28], cd_buf[29]]) as usize;
                    let extra_len = u16::from_le_bytes([cd_buf[30], cd_buf[31]]) as usize;
                    let comment_len = u16::from_le_bytes([cd_buf[32], cd_buf[33]]) as usize;

                    let mut name_bytes = vec![0u8; name_len];
                    if file.read_exact(&mut name_bytes).is_ok() {
                        let name = String::from_utf8_lossy(&name_bytes).to_string();
                        if !name.is_empty() && !entries.iter().any(|e| e.path.to_string_lossy() == name) {
                            let is_dir = name.ends_with('/') || name.ends_with('\\');
                            let mut entry = ArchiveEntry::new(name, uncomp_size, is_dir);
                            entry.compressed_size = Some(comp_size);
                            entries.push(entry);
                        }
                    }
                    pos += 46 + name_len as u64 + extra_len as u64 + comment_len as u64;
                } else {
                    pos += 1;
                }
            } else {
                pos = self.find_next_zip_header(&mut file, pos + 1, file_len)?;
            }
        }

        if entries.is_empty() {
            return Err(anyhow!("No local zip headers found"));
        }

        Ok(entries)
    }

    pub fn extract_local_headers(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        use std::io::SeekFrom;

        let mut file = File::open(archive_path)?;
        let file_len = file.metadata()?.len();
        let mut extracted = Vec::new();
        let mut pos: u64 = 0;
        let mut header_buf = [0u8; 30];

        while pos + 30 <= file_len {
            file.seek(SeekFrom::Start(pos))?;
            if file.read_exact(&mut header_buf).is_err() {
                break;
            }

            if header_buf[0..4] == [0x50, 0x4B, 0x03, 0x04] {
                let flags = u16::from_le_bytes([header_buf[6], header_buf[7]]);
                let has_data_descriptor = (flags & 0x0008) != 0;

                let comp_size = u32::from_le_bytes([
                    header_buf[18],
                    header_buf[19],
                    header_buf[20],
                    header_buf[21],
                ]) as u64;
                let uncomp_size = u32::from_le_bytes([
                    header_buf[22],
                    header_buf[23],
                    header_buf[24],
                    header_buf[25],
                ]) as u64;
                let name_len =
                    u16::from_le_bytes([header_buf[26], header_buf[27]]) as usize;
                let extra_len =
                    u16::from_le_bytes([header_buf[28], header_buf[29]]) as usize;

                if pos + 30 + (name_len as u64) > file_len {
                    break;
                }

                let mut name_bytes = vec![0u8; name_len];
                if file.read_exact(&mut name_bytes).is_ok() {
                    let raw_name = String::from_utf8_lossy(&name_bytes).to_string();
                    let safe_path = PathBuf::from(&raw_name);

                    let should_extract = entry_paths.is_empty()
                        || entry_paths
                            .iter()
                            .any(|p| safe_path.starts_with(p) || &safe_path == p);

                    if should_extract {
                        let out_path = compute_target_out_path(dest_dir, entry_paths, &safe_path);
                        let is_dir = raw_name.ends_with('/') || raw_name.ends_with('\\');

                        if is_dir {
                            let _ = fs::create_dir_all(&out_path);
                        } else {
                            if let Some(parent) = out_path.parent() {
                                let _ = fs::create_dir_all(parent);
                            }

                            let data_start = pos + 30 + (name_len as u64) + (extra_len as u64);
                            file.seek(SeekFrom::Start(data_start))?;

                            let mut entry_reader = Read::by_ref(&mut file).take(uncomp_size);
                            if let Ok(out_file) = File::create(&out_path) {
                                let mut buffered_out = BufWriter::with_capacity(BUFFER_SIZE, out_file);
                                let _ = std::io::copy(&mut entry_reader, &mut buffered_out);
                                let _ = buffered_out.flush();
                            }
                        }
                        extracted.push(out_path);
                    }
                }

                let data_start = pos + 30 + (name_len as u64) + (extra_len as u64);
                if comp_size > 0 && !has_data_descriptor {
                    pos = data_start.saturating_add(comp_size);
                } else {
                    pos = self.find_next_zip_header(&mut file, data_start, file_len)?;
                }
            } else if header_buf[0..4] == [0x50, 0x4B, 0x01, 0x02] {
                let mut cd_buf = [0u8; 46];
                file.seek(SeekFrom::Start(pos))?;
                if file.read_exact(&mut cd_buf).is_ok() {
                    let uncomp_size = u32::from_le_bytes([cd_buf[24], cd_buf[25], cd_buf[26], cd_buf[27]]) as u64;
                    let name_len = u16::from_le_bytes([cd_buf[28], cd_buf[29]]) as usize;
                    let extra_len = u16::from_le_bytes([cd_buf[30], cd_buf[31]]) as usize;
                    let comment_len = u16::from_le_bytes([cd_buf[32], cd_buf[33]]) as usize;
                    let local_header_offset = u32::from_le_bytes([cd_buf[42], cd_buf[43], cd_buf[44], cd_buf[45]]) as u64;

                    let mut name_bytes = vec![0u8; name_len];
                    if file.read_exact(&mut name_bytes).is_ok() {
                        let raw_name = String::from_utf8_lossy(&name_bytes).to_string();
                        let safe_path = PathBuf::from(&raw_name);

                        let should_extract = entry_paths.is_empty()
                            || entry_paths
                                .iter()
                                .any(|p| safe_path.starts_with(p) || &safe_path == p);

                        if should_extract {
                            let out_path = compute_target_out_path(dest_dir, entry_paths, &safe_path);
                            let is_dir = raw_name.ends_with('/') || raw_name.ends_with('\\');

                            if is_dir {
                                let _ = fs::create_dir_all(&out_path);
                            } else {
                                if let Some(parent) = out_path.parent() {
                                    let _ = fs::create_dir_all(parent);
                                }

                                let mut local_hdr = [0u8; 30];
                                file.seek(SeekFrom::Start(local_header_offset))?;
                                if file.read_exact(&mut local_hdr).is_ok() && local_hdr[0..4] == [0x50, 0x4B, 0x03, 0x04] {
                                    let l_name_len = u16::from_le_bytes([local_hdr[26], local_hdr[27]]) as u64;
                                    let l_extra_len = u16::from_le_bytes([local_hdr[28], local_hdr[29]]) as u64;
                                    let data_start = local_header_offset + 30 + l_name_len + l_extra_len;
                                    file.seek(SeekFrom::Start(data_start))?;

                                    let mut entry_reader = Read::by_ref(&mut file).take(uncomp_size);
                                    if let Ok(out_file) = File::create(&out_path) {
                                        let mut buffered_out = BufWriter::with_capacity(BUFFER_SIZE, out_file);
                                        let _ = std::io::copy(&mut entry_reader, &mut buffered_out);
                                        let _ = buffered_out.flush();
                                    }
                                }
                            }
                            if !extracted.contains(&out_path) {
                                extracted.push(out_path);
                            }
                        }
                    }
                    pos += 46 + name_len as u64 + extra_len as u64 + comment_len as u64;
                } else {
                    pos += 1;
                }
            } else {
                pos = self.find_next_zip_header(&mut file, pos + 1, file_len)?;
            }
        }

        if extracted.is_empty() {
            // Fallback for extraction via libarchive / bsdtar if local header scan produced 0
            use crate::archive::libarchive_engine::LibarchiveEngine;
            return LibarchiveEngine.extract_entries(archive_path, entry_paths, dest_dir);
        }

        Ok(extracted)
    }
}

impl ArchiveEngine for ZipEngine {
    fn list_entries(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>> {
        self.list_entries_with_password(archive_path, None)
    }

    fn list_entries_with_password(
        &self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<Vec<ArchiveEntry>> {
        let file = File::open(archive_path).context("Failed to open zip file")?;

        match ZipArchive::new(file) {
            Ok(mut zip) => {
                let mut entries = Vec::new();
                let mut is_encrypted_archive = false;

                for i in 0..zip.len() {
                    let file_res = match password {
                        Some(pwd) => zip.by_index_decrypt(i, pwd.as_bytes()),
                        None => zip.by_index(i),
                    };

                    let file = match file_res {
                        Ok(f) => f,
                        Err(err) => {
                            tracing::warn!("Failed to read zip index {} (password error?): {:?}", i, err);
                            return Err(anyhow!("Archive is encrypted or invalid password"));
                        }
                    };

                    if file.encrypted() {
                        is_encrypted_archive = true;
                    }

                    let is_dir = file.is_dir();
                    let entry_name = file
                        .enclosed_name()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| file.name().to_string());

                    let mut entry = ArchiveEntry::new(entry_name, file.size(), is_dir);
                    entry.compressed_size = Some(file.compressed_size());
                    entries.push(entry);
                }

                if is_encrypted_archive && password.is_none() {
                    return Err(anyhow!("Archive is encrypted: Password required"));
                }

                Ok(entries)
            }
            Err(err) => {
                tracing::warn!(
                    "ZipArchive::new failed for {} ({:?}). Falling back to local header stream scanner.",
                    archive_path.display(),
                    err
                );
                self.scan_local_headers(archive_path)
            }
        }
    }

    fn extract_entries_with_password(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
        password: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        let file = File::open(archive_path)?;
        fs::create_dir_all(dest_dir)?;

        if let Ok(mut zip) = ZipArchive::new(file) {
            let mut extracted = Vec::new();
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
                    None => continue,
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

        // Resilient fallback for extraction of ZIPs with missing EOCD / local headers
        self.extract_local_headers(archive_path, entry_paths, dest_dir)
    }

    fn create_archive(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        let file = File::create(archive_path)?;
        let buffered_file = BufWriter::with_capacity(BUFFER_SIZE, file);
        let mut zip = ZipWriter::new(buffered_file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for src in source_paths {
            if src.is_dir() {
                for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();
                    let name = path.strip_prefix(src.parent().unwrap_or(src))?;
                    if path.is_file() {
                        zip.start_file(name.to_string_lossy(), options)?;
                        let f = File::open(path)?;
                        let mut buffered_in = BufReader::with_capacity(BUFFER_SIZE, f);
                        std::io::copy(&mut buffered_in, &mut zip)?;
                    } else if !name.as_os_str().is_empty() {
                        zip.add_directory(name.to_string_lossy(), options)?;
                    }
                }
            } else if src.is_file() {
                let name = src.file_name().ok_or_else(|| anyhow!("Invalid path"))?;
                zip.start_file(name.to_string_lossy(), options)?;
                let f = File::open(src)?;
                let mut buffered_in = BufReader::with_capacity(BUFFER_SIZE, f);
                std::io::copy(&mut buffered_in, &mut zip)?;
            }
        }

        zip.finish()?;
        Ok(())
    }

    fn add_entries(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        if !archive_path.exists() {
            return self.create_archive(archive_path, source_paths);
        }

        let temp_archive_path = archive_path.with_extension("tmp_zip");

        {
            let src_file = File::open(archive_path)?;
            let mut zip_in = match ZipArchive::new(src_file) {
                Ok(zip) => zip,
                Err(err) => {
                    tracing::warn!(
                        "ZipArchive::new failed for {} ({:?}). Cannot append to invalid ZIP file directly.",
                        archive_path.display(),
                        err
                    );
                    return Err(anyhow!("Cannot append entries to invalid ZIP file: {:?}", err));
                }
            };

            let out_file = File::create(&temp_archive_path)?;
            let out_buf = BufWriter::with_capacity(BUFFER_SIZE, out_file);
            let mut zip_out = ZipWriter::new(out_buf);

            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            let new_names: Vec<String> = source_paths
                .iter()
                .filter_map(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .collect();

            // 1. Raw copy existing entries (0% CPU load, 0 decompression/re-compression)
            for i in 0..zip_in.len() {
                let zip_file = match zip_in.by_index(i) {
                    Ok(file) => file,
                    Err(_) => continue,
                };

                let safe_name = zip_file
                    .enclosed_name()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| zip_file.name().to_string());

                let is_overwritten = new_names
                    .iter()
                    .any(|name| safe_name == *name || safe_name.starts_with(&format!("{}/", name)));

                if !is_overwritten {
                    let _ = zip_out.raw_copy_file(zip_file);
                }
            }

            // 2. Compress ONLY the new source files being added
            for src in source_paths {
                if src.is_dir() {
                    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
                        let path = entry.path();
                        let name = path.strip_prefix(src.parent().unwrap_or(src))?;
                        if path.is_file() {
                            zip_out.start_file(name.to_string_lossy(), options)?;
                            let f = File::open(path)?;
                            let mut buffered_in = BufReader::with_capacity(BUFFER_SIZE, f);
                            std::io::copy(&mut buffered_in, &mut zip_out)?;
                        } else if !name.as_os_str().is_empty() {
                            zip_out.add_directory(name.to_string_lossy(), options)?;
                        }
                    }
                } else if src.is_file() {
                    let name = src.file_name().ok_or_else(|| anyhow!("Invalid path"))?;
                    zip_out.start_file(name.to_string_lossy(), options)?;
                    let f = File::open(src)?;
                    let mut buffered_in = BufReader::with_capacity(BUFFER_SIZE, f);
                    std::io::copy(&mut buffered_in, &mut zip_out)?;
                }
            }

            zip_out.finish()?;
        }

        fs::rename(&temp_archive_path, archive_path)?;
        Ok(())
    }

    fn delete_entries(&self, archive_path: &Path, entry_paths: &[PathBuf]) -> Result<()> {
        if entry_paths.is_empty() {
            return Ok(());
        }

        let temp_archive_path = archive_path.with_extension("tmp_del_zip");

        {
            let src_file = File::open(archive_path)?;
            let mut zip_in = ZipArchive::new(src_file)?;

            let out_file = File::create(&temp_archive_path)?;
            let out_buf = BufWriter::with_capacity(BUFFER_SIZE, out_file);
            let mut zip_out = ZipWriter::new(out_buf);

            let delete_targets: Vec<String> = entry_paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            for i in 0..zip_in.len() {
                let zip_file = match zip_in.by_index(i) {
                    Ok(file) => file,
                    Err(_) => continue,
                };

                let safe_name = zip_file
                    .enclosed_name()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| zip_file.name().to_string());

                let should_delete = delete_targets.iter().any(|target| {
                    let t = target.trim_end_matches('/');
                    safe_name == *target
                        || safe_name == t
                        || safe_name.starts_with(&format!("{}/", t))
                });

                if !should_delete {
                    let _ = zip_out.raw_copy_file(zip_file);
                }
            }

            zip_out.finish()?;
        }

        fs::rename(&temp_archive_path, archive_path)?;
        Ok(())
    }

    fn verify_integrity(
        &self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<super::IntegrityReport> {
        let file = File::open(archive_path)?;
        let mut zip = ZipArchive::new(file)?;
        let total_files = zip.len();
        let mut ok_files = 0;
        let mut failed_files = Vec::new();
        let mut buf = [0u8; 64 * 1024];

        for i in 0..total_files {
            let zip_res = match password {
                Some(pwd) => zip.by_index_decrypt(i, pwd.as_bytes()),
                None => zip.by_index(i),
            };

            match zip_res {
                Ok(mut zip_file) => {
                    let read_err = false;
                    while let Ok(n) = zip_file.read(&mut buf) {
                        if n == 0 {
                            break;
                        }
                    }
                    if read_err {
                        failed_files.push((zip_file.name().to_string(), "CRC / Read mismatch".to_string()));
                    } else {
                        ok_files += 1;
                    }
                }
                Err(err) => {
                    failed_files.push((format!("Entry #{}", i), format!("Read error: {:?}", err)));
                }
            }
        }

        Ok(super::IntegrityReport {
            total_files,
            ok_files,
            failed_files,
        })
    }

    fn encrypt_archive(&self, archive_path: &Path, password: &str) -> Result<()> {
        if password.is_empty() {
            return Ok(());
        }

        let temp_archive_path = archive_path.with_extension("tmp_enc_zip");

        {
            let src_file = File::open(archive_path)?;
            let mut zip_in = ZipArchive::new(src_file)?;

            let out_file = File::create(&temp_archive_path)?;
            let out_buf = BufWriter::with_capacity(BUFFER_SIZE, out_file);
            let mut zip_out = ZipWriter::new(out_buf);

            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .with_deprecated_encryption(password.as_bytes());

            let mut buf = vec![0u8; BUFFER_SIZE];

            for i in 0..zip_in.len() {
                let mut zip_file = match zip_in.by_index(i) {
                    Ok(file) => file,
                    Err(_) => continue,
                };

                let safe_name = zip_file
                    .enclosed_name()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| zip_file.name().to_string());

                if zip_file.is_dir() {
                    zip_out.add_directory(safe_name, options)?;
                } else {
                    zip_out.start_file(safe_name, options)?;
                    while let Ok(n) = zip_file.read(&mut buf) {
                        if n == 0 {
                            break;
                        }
                        zip_out.write_all(&buf[..n])?;
                    }
                }
            }

            zip_out.finish()?;
        }

        fs::rename(&temp_archive_path, archive_path)?;
        Ok(())
    }
}
