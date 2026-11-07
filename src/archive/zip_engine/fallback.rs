use super::records::find_next_zip_header;
use super::ZipEngine;
use crate::archive::entry::{compute_target_out_path, sanitize_archive_path};
use crate::archive::libarchive_engine::LibarchiveEngine;
use crate::archive::{ArchiveEngine, BUFFER_SIZE};
use anyhow::Result;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

impl ZipEngine {
    pub(super) fn extract_fallback(
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        let mut file = File::open(archive_path)?;
        let file_len = file.metadata()?.len();
        let mut extracted = Vec::new();
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
                let comp_size = u32::from_le_bytes(header_buf[18..22].try_into().unwrap()) as u64;
                let uncomp_size = u32::from_le_bytes(header_buf[22..26].try_into().unwrap()) as u64;
                let name_len = u16::from_le_bytes(header_buf[26..28].try_into().unwrap()) as usize;
                let extra_len = u16::from_le_bytes(header_buf[28..30].try_into().unwrap()) as usize;

                if pos + 30 + (name_len as u64) > file_len {
                    break;
                }

                let mut name_bytes = vec![0u8; name_len];
                if file.read_exact(&mut name_bytes).is_ok() {
                    let raw_name = String::from_utf8_lossy(&name_bytes).to_string();
                    if let Some(safe_path) = sanitize_archive_path(Path::new(&raw_name)) {
                        let should_extract = entry_paths.is_empty()
                            || entry_paths
                                .iter()
                                .any(|p| safe_path.starts_with(p) || &safe_path == p);

                        if should_extract {
                            let out_path =
                                compute_target_out_path(dest_dir, entry_paths, &safe_path);
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
                                    let mut buffered_out =
                                        BufWriter::with_capacity(BUFFER_SIZE, out_file);
                                    let _ = std::io::copy(&mut entry_reader, &mut buffered_out);
                                    let _ = buffered_out.flush();
                                }
                            }
                            extracted.push(out_path);
                        }
                    }
                }

                pos += 30 + (name_len as u64) + (extra_len as u64) + comp_size;
            } else if header_buf[0..4] == CENTRAL_DIR_MAGIC {
                let mut cd_buf = [0u8; 46];
                file.seek(SeekFrom::Start(pos))?;
                if file.read_exact(&mut cd_buf).is_ok() {
                    let name_len = u16::from_le_bytes(cd_buf[28..30].try_into().unwrap()) as usize;
                    let extra_len = u16::from_le_bytes(cd_buf[30..32].try_into().unwrap()) as usize;
                    let comment_len =
                        u16::from_le_bytes(cd_buf[32..34].try_into().unwrap()) as usize;

                    let mut name_bytes = vec![0u8; name_len];
                    if file.read_exact(&mut name_bytes).is_ok() {
                        let raw_name = String::from_utf8_lossy(&name_bytes).to_string();
                        if let Some(safe_path) = sanitize_archive_path(Path::new(&raw_name)) {
                            let should_extract = entry_paths.is_empty()
                                || entry_paths
                                    .iter()
                                    .any(|p| safe_path.starts_with(p) || &safe_path == p);

                            if should_extract && !extracted.iter().any(|p| p.ends_with(&safe_path))
                            {
                                let out_path =
                                    compute_target_out_path(dest_dir, entry_paths, &safe_path);
                                if raw_name.ends_with('/') || raw_name.ends_with('\\') {
                                    let _ = fs::create_dir_all(&out_path);
                                }
                            }
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

        if extracted.is_empty() {
            return LibarchiveEngine.extract_entries(archive_path, entry_paths, dest_dir);
        }

        Ok(extracted)
    }
}
