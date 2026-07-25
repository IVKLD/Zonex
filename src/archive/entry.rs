use std::path::PathBuf;

/// Represents a single file or directory entry inside an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub path: PathBuf,
    pub size: u64,
    pub compressed_size: Option<u64>,
    pub is_dir: bool,
    pub modified: Option<u64>,
    pub mode: Option<u32>,
}

impl ArchiveEntry {
    pub fn new(path: impl Into<PathBuf>, size: u64, is_dir: bool) -> Self {
        Self {
            path: path.into(),
            size,
            compressed_size: None,
            is_dir,
            modified: None,
            mode: None,
        }
    }

    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.to_string_lossy().to_string())
    }

    pub fn formatted_size(&self) -> String {
        if self.is_dir {
            return "<DIR>".to_string();
        }
        format_bytes(self.size)
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
