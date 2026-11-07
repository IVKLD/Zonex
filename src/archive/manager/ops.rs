use super::engine_resolver::get_engine_for_path;
use super::ArchiveManager;
use crate::archive::{ArchiveEngine, IntegrityReport};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

impl ArchiveManager {
    pub(super) fn active_engine(&self) -> Result<(Box<dyn ArchiveEngine>, &Path)> {
        let current = self
            .current_path
            .as_deref()
            .ok_or_else(|| anyhow!("No archive is currently open"))?;
        let engine_type = self
            .active_engine_type
            .ok_or_else(|| anyhow!("No active engine available"))?;
        Ok((engine_type.create_engine(), current))
    }

    pub fn create_archive(&mut self, archive_path: &Path, files: &[PathBuf]) -> Result<()> {
        tracing::info!(
            "Creating archive {} with {} source file(s)",
            archive_path.display(),
            files.len()
        );
        let engine = get_engine_for_path(archive_path)?;
        engine.create_archive(archive_path, files)?;
        self.open_archive(archive_path)?;
        Ok(())
    }

    pub fn delete_selected_entries(&mut self, entry_paths: &[PathBuf]) -> Result<()> {
        let saved_folder = self.nav.current_folder.clone();
        let (engine, current) = self.active_engine()?;
        tracing::info!(
            "Deleting {} selected entry(ies) from {}",
            entry_paths.len(),
            current.display()
        );

        let path = current.to_path_buf();
        engine.delete_entries(&path, entry_paths)?;
        self.open_archive(&path)?;

        if !saved_folder.as_os_str().is_empty()
            && !self
                .all_entries
                .iter()
                .any(|e| e.path.starts_with(&saved_folder))
        {
            let mut parent = saved_folder;
            parent.pop();
            self.nav.current_folder = parent;
        } else {
            self.nav.current_folder = saved_folder;
        }

        Ok(())
    }

    pub fn verify_integrity(&self, password: Option<&str>) -> Result<IntegrityReport> {
        let (engine, current) = self.active_engine()?;
        engine.verify_integrity(current, password)
    }

    pub fn add_files(&mut self, files: &[PathBuf]) -> Result<()> {
        let current_folder = self.nav.current_folder.clone();
        let (engine, current) = self.active_engine()?;
        tracing::info!(
            "Adding {} file(s) to open archive {} into folder '{}'",
            files.len(),
            current.display(),
            current_folder.display()
        );
        let path = current.to_path_buf();
        engine.add_entries_to_folder(&path, files, &current_folder)?;
        self.open_archive(&path)?;
        self.nav.current_folder = current_folder;
        Ok(())
    }

    pub fn create_folder(&mut self, folder_name: &str) -> Result<()> {
        let current_folder = self.nav.current_folder.clone();
        let target_path = if current_folder.as_os_str().is_empty() {
            PathBuf::from(folder_name)
        } else {
            current_folder.join(folder_name)
        };
        let (engine, current) = self.active_engine()?;
        tracing::info!(
            "Creating folder '{}' in open archive {}",
            target_path.display(),
            current.display()
        );
        let path = current.to_path_buf();
        engine.create_folder(&path, &target_path)?;
        self.open_archive(&path)?;
        self.nav.current_folder = current_folder;
        Ok(())
    }

    pub fn encrypt_archive(&mut self, password: &str) -> Result<()> {
        self.encrypt_archive_with_method(password, crate::archive::EncryptionMethod::default())
    }

    pub fn encrypt_archive_with_method(
        &mut self,
        password: &str,
        method: crate::archive::EncryptionMethod,
    ) -> Result<()> {
        let saved_folder = self.nav.current_folder.clone();
        let (engine, current) = self.active_engine()?;
        tracing::info!(
            "Encrypting open archive {} with method {:?}",
            current.display(),
            method
        );

        let path = current.to_path_buf();
        engine.encrypt_archive_with_method(&path, password, method)?;
        self.open_archive_with_password(&path, Some(password))?;
        self.nav.current_folder = saved_folder;
        Ok(())
    }
}
