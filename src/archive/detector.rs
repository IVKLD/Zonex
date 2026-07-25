use super::zip_engine::ZipEngine;
use super::ArchiveEngine;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFormat {
    Zip,
    Tar,
    TarGz,
    TarZst,
}

pub struct FormatDetector;

impl FormatDetector {
    /// Detects archive format using magic byte inspection, tail EOCD search, extension, or trial
    pub fn detect(path: &Path) -> Option<DetectedFormat> {
        tracing::debug!("Detecting format for: {}", path.display());

        // 1. Try content magic & tail EOCD inspection
        if let Ok(format) = Self::detect_by_content(path) {
            tracing::info!("Detected format {:?} by content signature for {}", format, path.display());
            return Some(format);
        }

        // 2. Try extension check
        if let Some(format) = Self::detect_by_extension(path) {
            tracing::info!("Detected format {:?} by file extension for {}", format, path.display());
            return Some(format);
        }

        // 3. Engine trial fallback
        if ZipEngine.list_entries(path).is_ok() {
            tracing::info!("Detected Zip format via engine trial for {}", path.display());
            return Some(DetectedFormat::Zip);
        }

        tracing::warn!("Could not detect format for file: {}", path.display());
        None
    }

    fn detect_by_content(path: &Path) -> std::io::Result<DetectedFormat> {
        let mut file = File::open(path)?;
        let file_len = file.metadata()?.len();

        let mut header = [0u8; 512];
        let bytes_read = file.read(&mut header)?;

        if bytes_read >= 2 {
            if header[0] == 0x1F && header[1] == 0x8B {
                return Ok(DetectedFormat::TarGz);
            }

            if bytes_read >= 4 && header[0..4] == [0x28, 0xB5, 0x2F, 0xFD] {
                return Ok(DetectedFormat::TarZst);
            }

            if bytes_read >= 4
                && (header[0..4] == [0x50, 0x4B, 0x03, 0x04]
                    || header[0..4] == [0x50, 0x4B, 0x05, 0x06]
                    || header[0..4] == [0x50, 0x4B, 0x07, 0x08])
            {
                return Ok(DetectedFormat::Zip);
            }
        }

        if bytes_read >= 262 && &header[257..262] == b"ustar" {
            return Ok(DetectedFormat::Tar);
        }

        if file_len >= 22 {
            let search_len = std::cmp::min(file_len, 65536) as usize;
            let _ = file.seek(SeekFrom::End(-(search_len as i64)));
            let mut tail_buf = vec![0u8; search_len];
            if file.read_exact(&mut tail_buf).is_ok() {
                for i in (0..search_len.saturating_sub(3)).rev() {
                    if tail_buf[i..i + 4] == [0x50, 0x4B, 0x05, 0x06] {
                        return Ok(DetectedFormat::Zip);
                    }
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Unknown format signature",
        ))
    }

    fn detect_by_extension(path: &Path) -> Option<DetectedFormat> {
        let name = path.to_string_lossy().to_lowercase();
        if name.ends_with(".zip")
            || name.ends_with(".exe")
            || name.ends_with(".jar")
            || name.ends_with(".apk")
            || name.ends_with(".aar")
            || name.ends_with(".war")
        {
            Some(DetectedFormat::Zip)
        } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            Some(DetectedFormat::TarGz)
        } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
            Some(DetectedFormat::TarZst)
        } else if name.ends_with(".tar") {
            Some(DetectedFormat::Tar)
        } else {
            None
        }
    }
}
