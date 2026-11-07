use super::ArchiveManager;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

impl ArchiveManager {
    pub fn extract_selected(
        &self,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        self.extract_selected_with_password(entry_paths, dest_dir, None)
    }

    pub fn extract_selected_with_password(
        &self,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
        password: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        let (engine, current) = self.active_engine()?;
        tracing::info!(
            "Extracting selected entries from {} to {} (password: {})",
            current.display(),
            dest_dir.display(),
            password.is_some()
        );

        let extracted =
            engine.extract_entries_with_password(current, entry_paths, dest_dir, password)?;
        if !extracted.is_empty() {
            Ok(extracted)
        } else {
            Err(anyhow!("Failed to extract selected entries from archive"))
        }
    }
}
