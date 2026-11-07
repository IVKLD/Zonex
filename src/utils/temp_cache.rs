use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub struct TempCache {
    mount_dir: PathBuf,
    stage_dir: PathBuf,
}

impl TempCache {
    pub fn new() -> Result<Self> {
        let base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .unwrap_or_else(std::env::temp_dir);

        let mount_dir = base.join(format!("zonex_ram_{}", std::process::id()));
        let stage_dir = mount_dir.join("stage");
        fs::create_dir_all(&stage_dir)?;

        Ok(Self {
            mount_dir,
            stage_dir,
        })
    }

    pub fn stage_dir(&self) -> PathBuf {
        self.stage_dir.clone()
    }

    pub fn open_cache_dir(&self) -> PathBuf {
        let open_dir = self.mount_dir.join("open");
        let _ = fs::create_dir_all(&open_dir);
        open_dir
    }

    pub fn clear_stage(&self) {
        let _ = fs::remove_dir_all(&self.stage_dir);
        let _ = fs::create_dir_all(&self.stage_dir);
    }
}

impl Drop for TempCache {
    fn drop(&mut self) {
        tracing::info!("Cleaning up RAM VFS mount directory: {:?}", self.mount_dir);
        let _ = fs::remove_dir_all(&self.mount_dir);
    }
}
