use std::path::{Component, Path, PathBuf};

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
            .unwrap_or(self.path.as_os_str())
            .to_string_lossy()
            .into_owned()
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

pub fn sanitize_archive_path(path: &Path) -> Option<PathBuf> {
    let clean: PathBuf = path
        .components()
        .filter_map(|comp| match comp {
            Component::Normal(c) => Some(c),
            _ => None,
        })
        .collect();
    (!clean.as_os_str().is_empty()).then_some(clean)
}

pub fn safe_join_dest(dest_dir: &Path, relative_path: &Path) -> Option<PathBuf> {
    let sanitized = sanitize_archive_path(relative_path)?;
    let target = dest_dir.join(sanitized);
    target.starts_with(dest_dir).then_some(target)
}

pub fn compute_target_out_path(
    dest_dir: &Path,
    entry_paths: &[PathBuf],
    safe_relative_path: &Path,
) -> PathBuf {
    let clean_path = match sanitize_archive_path(safe_relative_path) {
        Some(p) => p,
        None => return dest_dir.join("extracted_file"),
    };

    for p in entry_paths {
        if let Some(clean_p) = sanitize_archive_path(p) {
            if clean_path == clean_p {
                if let Some(file_name) = clean_p.file_name() {
                    return dest_dir.join(file_name);
                }
            } else if clean_path.starts_with(&clean_p) {
                if let (Some(parent_name), Ok(rel_sub)) =
                    (clean_p.file_name(), clean_path.strip_prefix(&clean_p))
                {
                    return dest_dir.join(parent_name).join(rel_sub);
                }
            }
        }
    }

    let target = dest_dir.join(&clean_path);
    if target.starts_with(dest_dir) {
        target
    } else {
        dest_dir.join("extracted_file")
    }
}
