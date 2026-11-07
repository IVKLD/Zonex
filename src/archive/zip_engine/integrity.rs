use super::ZipEngine;
use crate::archive::IntegrityReport;
use anyhow::Result;
use crc32fast::Hasher;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use zip::ZipArchive;

struct HashWriter<'a>(&'a mut Hasher);

impl Write for HashWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ZipEngine {
    pub(super) fn verify(archive_path: &Path, password: Option<&str>) -> Result<IntegrityReport> {
        let file = File::open(archive_path)?;
        let mut zip = ZipArchive::new(file)?;
        let total_files = zip.len();
        let mut ok_files = 0;
        let mut failed_files = Vec::new();

        for i in 0..total_files {
            let zip_res = match password {
                Some(pwd) => zip.by_index_decrypt(i, pwd.as_bytes()),
                None => zip.by_index(i),
            };

            match zip_res {
                Ok(mut zip_file) => {
                    let expected_crc = zip_file.crc32();
                    let mut hasher = Hasher::new();
                    match std::io::copy(&mut zip_file, &mut HashWriter(&mut hasher)) {
                        Err(e) => {
                            failed_files
                                .push((zip_file.name().to_string(), format!("Read error: {e}")));
                        }
                        Ok(_) => {
                            let computed_crc = hasher.finalize();
                            if computed_crc != expected_crc {
                                failed_files.push((
                                    zip_file.name().to_string(),
                                    format!(
                                        "CRC mismatch: expected {expected_crc:08x}, got {computed_crc:08x}"
                                    ),
                                ));
                            } else {
                                ok_files += 1;
                            }
                        }
                    }
                }
                Err(err) => {
                    failed_files.push((format!("Entry #{i}"), format!("Read error: {err:?}")));
                }
            }
        }

        Ok(IntegrityReport {
            total_files,
            ok_files,
            failed_files,
        })
    }
}
