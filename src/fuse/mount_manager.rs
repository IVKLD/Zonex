use super::archive_fs::ArchiveFuseFs;
use crate::archive::ArchiveEntry;
use fuser::MountOption;
use std::path::{Path, PathBuf};

pub struct FuseMountManager {
    mount_path: PathBuf,
    _session: Option<fuser::BackgroundSession>,
}

impl FuseMountManager {
    pub fn mount(archive_path: PathBuf, entries: &[ArchiveEntry]) -> anyhow::Result<Self> {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .unwrap_or_else(std::env::temp_dir);

        let archive_stem = archive_path.file_stem().map_or_else(
            || "archive".to_string(),
            |s| s.to_string_lossy().to_string(),
        );

        let mount_name = format!("{}_{}", archive_stem, std::process::id());
        let fuse_root = runtime_dir.join("zonex_fuse");
        let mount_path = fuse_root.join(&mount_name);

        std::fs::create_dir_all(&mount_path).map_err(|e| {
            anyhow::anyhow!("Cannot create FUSE mountpoint {:?}: {}", mount_path, e)
        })?;

        tracing::info!(
            "FUSE: mounting archive '{}' at '{}' ({} entries)",
            archive_path.display(),
            mount_path.display(),
            entries.len()
        );

        let fs = ArchiveFuseFs::new(archive_path, entries);

        let mut config = fuser::Config::default();
        config.mount_options.push(MountOption::RO);
        config
            .mount_options
            .push(MountOption::FSName(format!("zonex:{}", archive_stem)));

        let session = fuser::spawn_mount(fs, &mount_path, &config)
            .map_err(|e| anyhow::anyhow!("FUSE mount failed: {}. Is libfuse3 installed?", e))?;

        tracing::info!(
            "FUSE: session started successfully at '{}'",
            mount_path.display()
        );

        Ok(Self {
            mount_path,
            _session: Some(session),
        })
    }

    pub fn mount_path(&self) -> &Path {
        &self.mount_path
    }
}

impl Drop for FuseMountManager {
    fn drop(&mut self) {
        tracing::info!("FUSE: unmounting '{}'", self.mount_path.display());
        if let Some(session) = self._session.take() {
            drop(session);
        }
        let _ = std::fs::remove_dir(&self.mount_path);
    }
}

impl std::fmt::Debug for FuseMountManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuseMountManager")
            .field("mount_path", &self.mount_path)
            .finish()
    }
}
