use crate::archive::{ArchiveEntry, ArchiveManager};
use std::path::{Path, PathBuf};

pub fn is_self_drop(mgr: &ArchiveManager, paths: &[PathBuf]) -> bool {
    paths.iter().any(|p| {
        Some(p.as_path()) == mgr.current_path()
            || mgr.fuse_mount_path().is_some_and(|m| p.starts_with(m))
    })
}

pub fn execute_drop_action(
    mgr: &mut ArchiveManager,
    paths: &[PathBuf],
) -> (Vec<ArchiveEntry>, PathBuf, String, bool) {
    if mgr.current_path().is_some() {
        match mgr.add_files(paths) {
            Ok(()) => (
                mgr.visible_entries(),
                mgr.current_folder().to_path_buf(),
                format!("Added {} item(s) to open archive", paths.len()),
                true,
            ),
            Err(err) => {
                tracing::error!("Error adding files to archive: {:?}", err);
                (
                    Vec::new(),
                    PathBuf::new(),
                    format!("Error adding files: {}", err),
                    false,
                )
            }
        }
    } else {
        let default_archive = paths[0]
            .parent()
            .unwrap_or(Path::new("."))
            .join("archive.zip");
        match mgr.create_archive(&default_archive, paths) {
            Ok(()) => (
                mgr.visible_entries(),
                mgr.current_folder().to_path_buf(),
                format!("Created archive: {}", default_archive.display()),
                true,
            ),
            Err(err) => {
                tracing::error!("Error creating archive {:?}: {:?}", default_archive, err);
                (
                    Vec::new(),
                    PathBuf::new(),
                    format!("Error creating archive: {}", err),
                    false,
                )
            }
        }
    }
}
