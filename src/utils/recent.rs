use std::fs;
use std::path::{Path, PathBuf};

fn recent_file_path() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?
        .join("zonex");
    Some(dir.join("recent.txt"))
}

pub fn get_recent() -> Vec<PathBuf> {
    let Some(path) = recent_file_path() else {
        return Vec::new();
    };
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };

    content
        .lines()
        .map(|l| PathBuf::from(l.trim()))
        .filter(|p| !p.as_os_str().is_empty() && p.exists())
        .collect()
}

pub fn add_recent(file_path: &Path) {
    let Some(path) = recent_file_path() else {
        return;
    };
    let mut list = get_recent();
    list.retain(|p| p != file_path);
    list.insert(0, file_path.to_path_buf());
    list.truncate(8);

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = list
        .iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = fs::write(path, content);
}
