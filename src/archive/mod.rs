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

/// Common Interface for Archive Engines (Zip, Tar, TarGz, Libarchive, etc.)
pub trait ArchiveEngine: Send + Sync {
    /// Returns the list of all entries inside the archive
    fn list_entries(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>>;

    /// Extracts specific entries or all files to target directory
    fn extract_entries(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>>;

    /// Creates a new archive containing specified files/directories
    fn create_archive(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()>;

    /// Adds files/directories to an existing archive
    fn add_entries(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()>;
}
