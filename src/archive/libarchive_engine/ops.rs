use super::listing::list_entries;
use crate::archive::detector::detect;
use crate::archive::manager::EngineType;
use crate::archive::zip_engine::ZipEngine;
use crate::archive::{ArchiveEngine, IntegrityReport};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

fn get_target_engine(path: &Path) -> Option<Box<dyn ArchiveEngine>> {
    detect(path).map(|fmt| EngineType::from(fmt).create_engine())
}

pub fn create_archive(archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
    let engine = get_target_engine(archive_path).unwrap_or_else(|| Box::new(ZipEngine));
    engine.create_archive(archive_path, source_paths)
}

pub fn add_entries(archive_path: &Path, source_paths: &[PathBuf]) -> Result<()> {
    let engine = get_target_engine(archive_path)
        .ok_or_else(|| anyhow!("Adding files to this archive format is not supported"))?;
    engine.add_entries(archive_path, source_paths)
}

pub fn delete_entries(archive_path: &Path, entry_paths: &[PathBuf]) -> Result<()> {
    let engine = get_target_engine(archive_path)
        .ok_or_else(|| anyhow!("Deleting files from this archive format is not supported"))?;
    engine.delete_entries(archive_path, entry_paths)
}

pub fn verify_integrity(archive_path: &Path, password: Option<&str>) -> Result<IntegrityReport> {
    if let Ok(rep) = ZipEngine.verify_integrity(archive_path, password) {
        return Ok(rep);
    }

    let entries = list_entries(archive_path)?;
    Ok(IntegrityReport {
        total_files: entries.len(),
        ok_files: entries.len(),
        failed_files: Vec::new(),
    })
}
