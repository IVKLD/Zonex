use crate::archive::{ArchiveEntry, ArchiveManager};
use crate::ui::dialogs::PasswordOverlayWidget;
use crate::ui::file_view::FileView;
use crate::ui::window::actions::{LoadingCallback, ToastCallback};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct OpenResult {
    pub entries: Vec<ArchiveEntry>,
    pub msg: String,
    pub success: bool,
    pub duration: f64,
    pub has_fuse: bool,
    pub path: PathBuf,
}

pub async fn run_open_worker(
    mgr: Arc<Mutex<ArchiveManager>>,
    path: PathBuf,
    password: Option<String>,
) -> OpenResult {
    gio::spawn_blocking(move || {
        let start_time = Instant::now();
        let mut m = mgr.lock().unwrap_or_else(|e| e.into_inner());
        let (entries, msg, success) = match m.open_archive_with_password(&path, password.as_deref())
        {
            Ok(opened) => {
                crate::utils::add_recent(&path);
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                (opened, format!("Opened: {name}"), true)
            }
            Err(err) => (Vec::new(), format!("Failed to open: {err}"), false),
        };

        let has_fuse = m.fuse_mount_path().is_some();
        let duration = start_time.elapsed().as_secs_f64();
        OpenResult {
            entries,
            msg,
            success,
            duration,
            has_fuse,
            path,
        }
    })
    .await
    .unwrap_or_else(|_| OpenResult {
        entries: Vec::new(),
        msg: "Worker panicked".to_string(),
        success: false,
        duration: 0.0,
        has_fuse: false,
        path: PathBuf::new(),
    })
}

pub fn apply_open_success(
    res: OpenResult,
    fv: &FileView,
    sl: &dyn Fn(bool, &str),
    toast: &dyn Fn(&str),
) {
    let status_suffix = if res.has_fuse { " [FUSE]" } else { " [tmpfs]" };
    sl(false, &format!("{}{}", res.msg, status_suffix));
    fv.set_entries(res.entries);
    fv.update_folder_path(Path::new(""));
    let dnd_info = if res.has_fuse {
        "⚡ DnD via FUSE (zero-latency)"
    } else {
        "⚠ DnD via tmpfs (install libfuse3 for streaming)"
    };
    toast(&format!(
        "{} ({:.2}s) — {}",
        res.msg, res.duration, dnd_info
    ));
}

pub fn prompt_password_retry(
    overlay: Rc<PasswordOverlayWidget>,
    mgr: Arc<Mutex<ArchiveManager>>,
    fv: Rc<FileView>,
    sl: LoadingCallback,
    toast: ToastCallback,
    path: PathBuf,
) {
    overlay.prompt(move |pwd_input: String, on_done| {
        let fv_c = fv.clone();
        let sl_c = sl.clone();
        let toast_c = toast.clone();
        let on_done_c = on_done.clone();
        let mgr_c = mgr.clone();
        let path_c = path.clone();

        glib::MainContext::default().spawn_local(async move {
            let res = run_open_worker(mgr_c, path_c, Some(pwd_input)).await;
            if res.success {
                apply_open_success(res, &fv_c, &*sl_c, &*toast_c);
                on_done_c(true, "");
            } else {
                sl_c(false, "Password required or invalid format");
                on_done_c(false, "❌ Incorrect password. Please try another password.");
            }
        });
    });
}
