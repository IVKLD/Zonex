use super::drop_worker::{execute_drop_action, is_self_drop};
use crate::archive::ArchiveManager;
use crate::dnd::setup_drop_target;
use crate::ui::file_view::FileView;
use crate::ui::window::actions::{LoadingCallback, OpenCallback, ToastCallback};
use libadwaita::ApplicationWindow;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub fn setup_window_drop(
    window: &ApplicationWindow,
    manager: Arc<Mutex<ArchiveManager>>,
    file_view: Rc<FileView>,
    set_loading: LoadingCallback,
    show_toast: ToastCallback,
    do_open: OpenCallback,
) {
    setup_drop_target(window, move |paths| {
        if paths.is_empty() {
            return;
        }

        {
            let mgr = manager.lock().unwrap_or_else(|e| e.into_inner());
            if is_self_drop(&mgr, &paths) {
                tracing::info!("Drop-in ignored: files belong to the currently open archive");
                return;
            }

            if paths.len() == 1
                && mgr.current_path().is_none()
                && ArchiveManager::get_engine_for_path(&paths[0]).is_ok()
            {
                drop(mgr);
                do_open(paths[0].clone());
                return;
            }
        }

        let manager_task = manager.clone();
        let fv_task = file_view.clone();
        let sl_task = set_loading.clone();
        let toast_task = show_toast.clone();

        sl_task(true, "Processing files in background...");

        glib::MainContext::default().spawn_local(async move {
            let res = gio::spawn_blocking(move || {
                let start_time = Instant::now();
                let mut mgr = manager_task.lock().unwrap_or_else(|e| e.into_inner());
                let (entries, folder, msg, success) = execute_drop_action(&mut mgr, &paths);
                let duration = start_time.elapsed().as_secs_f64();
                (entries, folder, msg, success, duration)
            })
            .await;

            if let Ok((entries, folder, msg, success, duration)) = res {
                sl_task(false, &msg);
                if success {
                    fv_task.set_entries(entries);
                    fv_task.update_folder_path(&folder);
                    toast_task(&format!("{} ({:.2}s)", msg, duration));
                } else {
                    toast_task(&msg);
                }
            } else {
                sl_task(false, "Drop processing failed");
                toast_task("Drop processing panicked or cancelled");
            }
        });
    });
}
