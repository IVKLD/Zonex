use super::engine_resolver::{candidate_engine_types, EngineType};
use crate::archive::ArchiveEntry;
use crate::fuse::FuseMountManager;
use anyhow::{anyhow, Result};
use std::path::Path;

pub fn open_archive_with_password(
    path: &Path,
    password: Option<&str>,
) -> Result<(Vec<ArchiveEntry>, EngineType, Option<FuseMountManager>)> {
    tracing::info!(
        "Attempting to open archive: {} (password: {})",
        path.display(),
        password.is_some()
    );

    let candidates = candidate_engine_types(path);
    let mut last_err = anyhow!("Failed to open archive: Unsupported format or corrupted file");

    for engine_type in candidates {
        let engine = engine_type.create_engine();
        match engine.list_entries_with_password(path, password) {
            Ok(entries) => {
                tracing::info!(
                    "Successfully opened archive {} using engine {:?}. Total entries: {}",
                    path.display(),
                    engine_type,
                    entries.len()
                );

                let fuse_mount = match FuseMountManager::mount(path.to_path_buf(), &entries) {
                    Ok(mount) => {
                        tracing::info!(
                            "FUSE: virtual FS ready at '{}' — DnD will use zero-latency streaming",
                            mount.mount_path().display()
                        );
                        Some(mount)
                    }
                    Err(e) => {
                        tracing::warn!(
                            "FUSE mount unavailable (libfuse3 not installed?): {}. DnD will fallback to tmpfs extraction.",
                            e
                        );
                        None
                    }
                };

                return Ok((entries, engine_type, fuse_mount));
            }
            Err(err) => {
                let err_msg = err.to_string();
                let err_lower = err_msg.to_lowercase();
                if err_lower.contains("password")
                    || err_lower.contains("encrypted")
                    || err_lower.contains("passphrase")
                    || err_lower.contains("bad decrypt")
                {
                    return Err(err);
                }
                tracing::warn!(
                    "Engine {:?} failed for {}: {:?}",
                    engine_type,
                    path.display(),
                    err
                );
                last_err = err;
            }
        }
    }

    tracing::error!(
        "All engine attempts failed for {}: {:?}",
        path.display(),
        last_err
    );
    Err(last_err)
}
