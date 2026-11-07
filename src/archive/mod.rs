pub mod detector;
pub mod entry;
pub mod libarchive_engine;
pub mod manager;
pub mod tar_engine;
pub mod zip_engine;

#[cfg(test)]
mod tests;

pub use detector::{detect, DetectedFormat};
pub use entry::ArchiveEntry;
pub use manager::ArchiveManager;

use anyhow::Result;
use std::path::{Path, PathBuf};

pub const BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct IntegrityReport {
    pub total_files: usize,
    pub ok_files: usize,
    pub failed_files: Vec<(String, String)>,
}

pub trait ArchiveEngine: Send + Sync {
    fn list_entries(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>>;

    fn list_entries_with_password(
        &self,
        archive_path: &Path,
        _password: Option<&str>,
    ) -> Result<Vec<ArchiveEntry>> {
        self.list_entries(archive_path)
    }

    fn extract_entries(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        self.extract_entries_with_password(archive_path, entry_paths, dest_dir, None)
    }

    fn extract_entries_with_password(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
        password: Option<&str>,
    ) -> Result<Vec<PathBuf>>;

    fn create_archive(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()>;

    fn add_entries(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()>;

    fn delete_entries(&self, archive_path: &Path, entry_paths: &[PathBuf]) -> Result<()>;

    fn verify_integrity(
        &self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<IntegrityReport>;

    fn encrypt_archive(&self, archive_path: &Path, password: &str) -> Result<()> {
        let _ = (archive_path, password);
        anyhow::bail!("Encryption not supported for this archive engine")
    }
}
