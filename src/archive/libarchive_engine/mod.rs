mod extract;
mod listing;
mod ops;

use super::{ArchiveEngine, ArchiveEntry, IntegrityReport};
use anyhow::Result;
use extract::extract_entries_with_password;
use listing::list_entries;
use ops::{add_entries, create_archive, delete_entries, verify_integrity};
use std::path::{Path, PathBuf};

pub struct LibarchiveEngine;

impl ArchiveEngine for LibarchiveEngine {
    fn list_entries(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>> {
        list_entries(archive_path)
    }

    fn extract_entries(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        extract_entries_with_password(archive_path, entry_paths, dest_dir, None)
    }

    fn extract_entries_with_password(
        &self,
        archive_path: &Path,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
        password: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        extract_entries_with_password(archive_path, entry_paths, dest_dir, password)
    }

    fn create_archive(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        create_archive(archive_path, source_paths)
    }

    fn add_entries(&self, archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
        add_entries(archive_path, source_paths)
    }

    fn delete_entries(&self, archive_path: &Path, entry_paths: &[PathBuf]) -> Result<()> {
        delete_entries(archive_path, entry_paths)
    }

    fn verify_integrity(
        &self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<IntegrityReport> {
        verify_integrity(archive_path, password)
    }
}
