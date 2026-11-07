use crate::archive::ArchiveEntry;
use anyhow::{Context, Result};
use compress_tools::list_archive_entries;
use std::fs::File;
use std::path::Path;

pub fn list_entries(archive_path: &Path) -> Result<Vec<ArchiveEntry>> {
    let mut file = File::open(archive_path).context("Failed to open archive for libarchive")?;
    let file_list =
        list_archive_entries(&mut file).context("Failed to list files with libarchive")?;

    let entries = file_list
        .into_iter()
        .map(|entry| {
            let name = entry.path;
            let is_dir = name.ends_with('/') || name.ends_with('\\');
            ArchiveEntry::new(name, entry.size, is_dir)
        })
        .collect();

    Ok(entries)
}
