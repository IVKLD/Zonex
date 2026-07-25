use super::archive_fs::ArchiveFuseFs;
use crate::archive::ArchiveEntry;
use fuser::MountOption;
use std::path::{Path, PathBuf};

/// Управляет жизненным циклом FUSE-монтирования архива.
///
/// При дропе (`Drop`) сессия завершается и точка монтирования автоматически
/// отмонтируется (благодаря `MountOption::AutoUnmount`).
/// Если процесс убит сигналом SIGKILL — ядро тоже размонтирует через auto_unmount.
pub struct FuseMountManager {
    /// Точка монтирования: /run/user/<uid>/zonex_fuse/<archive_stem>_<pid>/
    pub mount_path: PathBuf,
    /// BackgroundSession держит FUSE-поток живым. При дропе — размонтирование.
    _session: fuser::BackgroundSession,
}

impl FuseMountManager {
    /// Монтирует архив как виртуальную FUSE-ФС.
    ///
    /// Вызывается из фонового потока (при open_archive), не блокирует UI.
    /// Точка монтирования: `$XDG_RUNTIME_DIR/zonex_fuse/<name>_<pid>`
    pub fn mount(archive_path: PathBuf, entries: &[ArchiveEntry]) -> anyhow::Result<Self> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(format!("/run/user/{}", unsafe { libc::getuid() })));

        let archive_stem = archive_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "archive".to_string());

        // Уникальное имя монтирования — на случай нескольких одновременных архивов
        let mount_name = format!("{}_{}", archive_stem, std::process::id());
        let fuse_root = runtime_dir.join("zonex_fuse");
        let mount_path = fuse_root.join(&mount_name);

        std::fs::create_dir_all(&mount_path)
            .map_err(|e| anyhow::anyhow!("Cannot create FUSE mountpoint {:?}: {}", mount_path, e))?;

        tracing::info!(
            "FUSE: mounting archive '{}' at '{}' ({} entries)",
            archive_path.display(),
            mount_path.display(),
            entries.len()
        );

        let fs = ArchiveFuseFs::new(archive_path.clone(), entries);

        let options = vec![
            MountOption::RO, // Read-only — архиватор не пишет через FUSE
            // НЕ используем AutoUnmount: fuser добавляет к нему allow_other,
            // которому нужен user_allow_other=true в /etc/fuse.conf → EPERM.
            // BackgroundSession::drop() вызывает unmount при нормальном завершении.
            MountOption::FSName(format!("zonex:{}", archive_stem)),
        ];

        let session = fuser::spawn_mount2(fs, &mount_path, &options)
            .map_err(|e| anyhow::anyhow!("FUSE mount failed: {}. Is libfuse3 installed?", e))?;

        tracing::info!("FUSE: session started successfully at '{}'", mount_path.display());

        Ok(Self {
            mount_path,
            _session: session,
        })
    }

    /// Возвращает путь к смонтированному архиву.
    pub fn mount_path(&self) -> &Path {
        &self.mount_path
    }
}

impl Drop for FuseMountManager {
    fn drop(&mut self) {
        // BackgroundSession::drop() вызывает unmount2() — это размонтирует FUSE.
        // После размонтирования можно безопасно удалить пустую директорию.
        tracing::info!("FUSE: unmounting '{}'", self.mount_path.display());
        // Директория удаляется после drop _session (гарантируется порядком полей в структуре)
    }
}

impl std::fmt::Debug for FuseMountManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuseMountManager")
            .field("mount_path", &self.mount_path)
            .finish()
    }
}
