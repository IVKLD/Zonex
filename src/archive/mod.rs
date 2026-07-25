pub mod detector;
pub mod entry;
pub mod libarchive_engine;
pub mod manager;
pub mod tar_engine;
pub mod zip_engine;

#[cfg(test)]
mod tests;

pub use entry::ArchiveEntry;
pub use manager::ArchiveManager;

use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct IntegrityReport {
    pub total_files: usize,
    pub ok_files: usize,
    pub failed_files: Vec<(String, String)>,
}

/// Common Interface for Archive Engines (Zip, Tar, TarGz, Libarchive, etc.)
pub trait ArchiveEngine: Send + Sync {
    /// Returns the list of all entries inside the archive
    fn list_entries(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>>;

    /// Returns the list of all entries inside the archive with password support
    fn list_entries_with_password(
        &self,
        archive_path: &Path,
        _password: Option<&str>,
    ) -> Result<Vec<ArchiveEntry>> {
        self.list_entries(archive_path)
    }

    /// Extracts specific entries or all files to target directory
    fn extract_entries(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        self.extract_entries_with_password(archive_path, entry_paths, dest_dir, None)
    }

    /// Extracts specific entries or all files to target directory with password support
    fn extract_entries_with_password(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
        password: Option<&str>,
    ) -> Result<Vec<PathBuf>>;

    /// Creates a new archive containing specified files/directories
    fn create_archive(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()>;

    /// Adds files/directories to an existing archive
    fn add_entries(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()>;

    /// Removes specified files/directories from an existing archive
    fn delete_entries(&self, archive_path: &Path, entry_paths: &[PathBuf]) -> Result<()>;

    /// Verifies checksums and readability of all entries in the archive
    fn verify_integrity(&self, archive_path: &Path, password: Option<&str>) -> Result<IntegrityReport>;

    /// Encrypts/protects all entries in the archive with a password
    fn encrypt_archive(&self, archive_path: &Path, password: &str) -> Result<()> {
        let _ = (archive_path, password);
        anyhow::bail!("Encryption not supported for this archive engine")
    }
}

