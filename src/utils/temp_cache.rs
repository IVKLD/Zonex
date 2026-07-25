use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// RAM-backed tmpfs VFS staging manager for Zero-Latency Drag-and-Drop
pub struct TempCache {
    mount_dir: PathBuf,
}

impl TempCache {
    pub fn new() -> Result<Self> {
        let user_run_dir = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/run/user/1000"));

        let base_dir = if user_run_dir.exists() {
            user_run_dir.join("zonex_mount_ram")
        } else {
            PathBuf::from("/tmp/zonex_mount_ram")
        };

        let stage_dir = base_dir.join("stage");
        fs::create_dir_all(&stage_dir)?;

        Ok(Self {
            mount_dir: base_dir,
        })
    }

    /// Returns persistent RAM-backed staging directory
    pub fn stage_dir(&self) -> PathBuf {
        let stage = self.mount_dir.join("stage");
        let _ = fs::create_dir_all(&stage);
        stage
    }

    /// Clears staging directory cache
    pub fn clear_stage(&self) {
        let stage = self.stage_dir();
        if stage.exists() {
            let _ = fs::remove_dir_all(&stage);
            let _ = fs::create_dir_all(&stage);
        }
    }
}

impl Drop for TempCache {
    fn drop(&mut self) {
        if self.mount_dir.exists() {
            tracing::info!("Cleaning up RAM VFS mount directory: {:?}", self.mount_dir);
            let _ = fs::remove_dir_all(&self.mount_dir);
        }
    }
}
