mod delete;
mod encrypt;
mod extract;
mod fallback;
mod integrity;
mod records;
mod writer;

use super::{ArchiveEngine, ArchiveEntry, IntegrityReport};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct ZipEngine;

impl ArchiveEngine for ZipEngine {
    fn list_entries(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>> {
        Self::list(archive_path, None)
    }

    fn list_entries_with_password(
        &self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<Vec<ArchiveEntry>> {
        Self::list(archive_path, password)
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
    ) -> Result<Vec<PathBuf>> {
        Self::extract(archive_path, entry_paths, dest_dir, password)
    }

    fn create_archive(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        Self::create(archive_path, source_paths)
    }

    fn add_entries(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        Self::append(archive_path, source_paths)
    }

    fn delete_entries(&self, archive_path: &Path, entry_paths: &[PathBuf]) -> Result<()> {
        Self::delete(archive_path, entry_paths)
    }

    fn verify_integrity(
        &self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<IntegrityReport> {
        Self::verify(archive_path, password)
    }

    fn encrypt_archive(&self, archive_path: &Path, password: &str) -> Result<()> {
        Self::encrypt(archive_path, password, crate::archive::EncryptionMethod::default())
    }

    fn encrypt_archive_with_method(
        &self,
        archive_path: &Path,
        password: &str,
        method: crate::archive::EncryptionMethod,
    ) -> Result<()> {
        Self::encrypt(archive_path, password, method)
    }
}
