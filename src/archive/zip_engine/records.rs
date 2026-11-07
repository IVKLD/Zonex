use super::ZipEngine;
use crate::archive::ArchiveEntry;
use anyhow::{anyhow, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub(super) fn find_next_zip_header(file: &mut File, start_pos: u64, file_len: u64) -> Result<u64> {
    let mut pos = start_pos;
    let mut buf = [0u8; 65536];

    while pos + 4 <= file_len {
        file.seek(SeekFrom::Start(pos))?;
        let bytes_read = match file.read(&mut buf) {
            Ok(n) if n >= 4 => n,
            _ => break,
        };

        if let Some(i) = buf[..bytes_read].windows(4).position(|w| {
            w[0..2] == [0x50, 0x4B] && (w[2..4] == [0x03, 0x04] || w[2..4] == [0x01, 0x02])
        }) {
            return Ok(pos + i as u64);
        }

        if bytes_read <= 3 {
            break;
        }
        pos += (bytes_read - 3) as u64;
    }

    Ok(file_len)
}

impl ZipEngine {
    pub(super) fn scan_headers(archive_path: &Path) -> Result<Vec<ArchiveEntry>> {
        let mut file = File::open(archive_path)?;
        let file_len = file.metadata()?.len();
        let mut entries = Vec::new();
        let mut pos: u64 = 0;
        let mut header_buf = [0u8; 30];

        const LOCAL_HEADER_MAGIC: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
        const CENTRAL_DIR_MAGIC: [u8; 4] = [0x50, 0x4B, 0x01, 0x02];

        while pos + 30 <= file_len {
            file.seek(SeekFrom::Start(pos))?;
            if file.read_exact(&mut header_buf).is_err() {
                break;
            }

            if header_buf[0..4] == LOCAL_HEADER_MAGIC {
                let flags = u16::from_le_bytes(header_buf[6..8].try_into().unwrap());
                let has_data_descriptor = (flags & 0x0008) != 0;

                let comp_size = u32::from_le_bytes(header_buf[18..22].try_into().unwrap()) as u64;
                let uncomp_size = u32::from_le_bytes(header_buf[22..26].try_into().unwrap()) as u64;
                let name_len = u16::from_le_bytes(header_buf[26..28].try_into().unwrap()) as usize;
                let extra_len = u16::from_le_bytes(header_buf[28..30].try_into().unwrap()) as usize;

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
                    pos = find_next_zip_header(&mut file, data_start, file_len)?;
                }
            } else if header_buf[0..4] == CENTRAL_DIR_MAGIC {
                let mut cd_buf = [0u8; 46];
                file.seek(SeekFrom::Start(pos))?;
                if file.read_exact(&mut cd_buf).is_ok() {
                    let comp_size = u32::from_le_bytes(cd_buf[20..24].try_into().unwrap()) as u64;
                    let uncomp_size = u32::from_le_bytes(cd_buf[24..28].try_into().unwrap()) as u64;
                    let name_len = u16::from_le_bytes(cd_buf[28..30].try_into().unwrap()) as usize;
                    let extra_len = u16::from_le_bytes(cd_buf[30..32].try_into().unwrap()) as usize;
                    let comment_len =
                        u16::from_le_bytes(cd_buf[32..34].try_into().unwrap()) as usize;

                    let mut name_bytes = vec![0u8; name_len];
                    if file.read_exact(&mut name_bytes).is_ok() {
                        let name = String::from_utf8_lossy(&name_bytes).to_string();
                        if !name.is_empty() && !entries.iter().any(|e| e.path == Path::new(&name)) {
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
                pos = find_next_zip_header(&mut file, pos + 1, file_len)?;
            }
        }

        if entries.is_empty() {
            return Err(anyhow!("No local zip headers found"));
        }

        Ok(entries)
    }
}
