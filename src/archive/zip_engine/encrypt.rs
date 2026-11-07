use super::ZipEngine;
use crate::archive::EncryptionMethod;
use anyhow::Result;
use std::fs::File;
use std::path::Path;
use tempfile::NamedTempFile;
use zip::unstable::write::FileOptionsExt;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

impl ZipEngine {
    pub(super) fn encrypt(
        archive_path: &Path,
        password: &str,
        method: EncryptionMethod,
    ) -> Result<()> {
        if password.is_empty() {
            return Ok(());
        }

        let src_file = File::open(archive_path)?;
        let mut zip_in = ZipArchive::new(src_file)?;

        let parent = archive_path.parent().unwrap_or(Path::new("."));
        let temp_file = NamedTempFile::new_in(parent)?;
        let mut zip_out = ZipWriter::new(temp_file);

        let options = match method {
            EncryptionMethod::Aes256 => SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .with_aes_encryption(zip::AesMode::Aes256, password),
            EncryptionMethod::Aes192 => SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .with_aes_encryption(zip::AesMode::Aes192, password),
            EncryptionMethod::Aes128 => SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .with_aes_encryption(zip::AesMode::Aes128, password),
            EncryptionMethod::ZipCrypto => SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .with_deprecated_encryption(password.as_bytes())?,
        };

        for i in 0..zip_in.len() {
            let mut zip_file = match zip_in.by_index(i) {
                Ok(file) => file,
                Err(_) => continue,
            };

            let safe_name = zip_file
                .enclosed_name()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| zip_file.name().replace('\\', "/"));

            if zip_file.is_dir() {
                zip_out.add_directory(&safe_name, options)?;
            } else {
                zip_out.start_file(&safe_name, options)?;
                std::io::copy(&mut zip_file, &mut zip_out)?;
            }
        }

        let temp_file = zip_out.finish()?;
        temp_file.persist(archive_path)?;
        Ok(())
    }
}
