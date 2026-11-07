mod read;
mod write;

use super::{ArchiveEngine, ArchiveEntry, IntegrityReport};
use anyhow::Result;
use read::{extract_entries, list_entries, verify_integrity};
use std::path::{Path, PathBuf};
use write::{add_entries, create_archive, create_folder, delete_entries};

pub enum TarCompression {
    None,
    Gzip,
    Zstd,
}

pub struct TarEngine {
    pub compression: TarCompression,
}

impl TarEngine {
    pub fn new(compression: TarCompression) -> Self {
        Self { compression }
    }
}

impl ArchiveEngine for TarEngine {
    fn list_entries(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>> {
        list_entries(&self.compression, archive_path)
    }

    fn extract_entries(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        extract_entries(&self.compression, archive_path, entry_paths, dest_dir)
    }

    fn extract_entries_with_password(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
        _password: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        extract_entries(&self.compression, archive_path, entry_paths, dest_dir)
    }

    fn create_archive(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        create_archive(&self.compression, archive_path, source_paths)
    }

    fn add_entries(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        add_entries(&self.compression, archive_path, source_paths, Path::new(""))
    }

    fn add_entries_to_folder(
        &self,
        archive_path: &Path,
        source_paths: &[PathBuf],
        target_folder: &Path,
    ) -> Result<()> {
        add_entries(&self.compression, archive_path, source_paths, target_folder)
    }

    fn create_folder(&self, archive_path: &Path, folder_path: &Path) -> Result<()> {
        create_folder(&self.compression, archive_path, folder_path)
    }

    fn delete_entries(&self, archive_path: &Path, entry_paths: &[PathBuf]) -> Result<()> {
        delete_entries(&self.compression, archive_path, entry_paths)
    }

    fn verify_integrity(
        &self,
        archive_path: &Path,
        _password: Option<&str>,
    ) -> Result<IntegrityReport> {
        verify_integrity(&self.compression, archive_path)
    }
}
