use std::fs;
use std::path::{Path, PathBuf};

pub struct RecentManager;

impl RecentManager {
    fn config_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        let dir = PathBuf::from(home).join(".config/zonex");
        let _ = fs::create_dir_all(&dir);
        Some(dir.join("recent.txt"))
    }

    pub fn get_recent() -> Vec<PathBuf> {
        if let Some(path) = Self::config_path() {
            if let Ok(content) = fs::read_to_string(path) {
                return content
                    .lines()
                    .map(|l| PathBuf::from(l.trim()))
                    .filter(|p| !p.as_os_str().is_empty() && p.exists())
                    .collect();
            }
        }
        Vec::new()
    }

    pub fn add_recent(file_path: &Path) {
        let mut list = Self::get_recent();
        let canonical = file_path.to_path_buf();
        list.retain(|p| p != &canonical);
        list.insert(0, canonical);
        list.truncate(8);

        if let Some(path) = Self::config_path() {
            let content = list
                .iter()
                .map(|p| p.to_string_lossy())
                .collect::<Vec<_>>()
                .join("\n");
            let _ = fs::write(path, content);
        }
    }
}
