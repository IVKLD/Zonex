use super::ZipEngine;
use anyhow::Result;
use std::fs::File;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use zip::{ZipArchive, ZipWriter};

impl ZipEngine {
    pub(super) fn delete(archive_path: &Path, entry_paths: &[PathBuf]) -> Result<()> {
        if entry_paths.is_empty() {
            return Ok(());
        }

        let src_file = File::open(archive_path)?;
        let mut zip_in = ZipArchive::new(src_file)?;

        let parent = archive_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_file = NamedTempFile::new_in(parent)?;
        let mut zip_out = ZipWriter::new(temp_file);

        let delete_targets: Vec<String> = entry_paths
            .iter()
            .map(|p| {
                let s = p.to_string_lossy().replace('\\', "/");
                s.trim_end_matches('/').to_string()
            })
            .collect();

        for i in 0..zip_in.len() {
            let zip_file = match zip_in.by_index(i) {
                Ok(file) => file,
                Err(_) => continue,
            };

            let safe_name = zip_file
                .enclosed_name()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| zip_file.name().replace('\\', "/"));

            let should_delete = delete_targets.iter().any(|t| {
                safe_name == *t
                    || safe_name
                        .strip_prefix(t.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
            });

            if !should_delete {
                let _ = zip_out.raw_copy_file(zip_file);
            }
        }

        let temp_file = zip_out.finish()?;
        temp_file.persist(archive_path)?;
        Ok(())
    }
}
