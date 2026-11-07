use super::context::ActionsContext;
use gio::{Cancellable, File as GioFile, SimpleAction};
use gtk4::prelude::*;
use gtk4::FileDialog;
use std::path::PathBuf;

pub fn register(ctx: &ActionsContext) {
    let manager = ctx.manager.clone();
    let file_view = ctx.file_view.clone();
    let show_toast = ctx.show_toast.clone();
    let set_loading = ctx.set_loading.clone();
    let window = ctx.window.clone();

    let action = SimpleAction::new("add_files", None);
    action.connect_activate(move |_, _| {
        let mgr_check = manager.lock().unwrap_or_else(|e| e.into_inner());
        if mgr_check.current_path().is_none() {
            show_toast("No archive open to add files to");
            return;
        }
        drop(mgr_check);

        let dialog = FileDialog::builder().title("Add Files to Archive").build();

        let mgr_task = manager.clone();
        let fv_task = file_view.clone();
        let sl_task = set_loading.clone();
        let toast_task = show_toast.clone();

        dialog.open_multiple(Some(&window), Cancellable::NONE, move |res| {
            if let Ok(files) = res {
                let paths: Vec<PathBuf> = (0..files.n_items())
                    .filter_map(|i| files.item(i)?.downcast::<GioFile>().ok()?.path())
                    .collect();

                if !paths.is_empty() {
                    sl_task(true, "Adding files to archive...");

                    let mgr_c = mgr_task.clone();
                    let paths_c = paths.clone();
                    let sl_task_c = sl_task.clone();
                    let fv_task_c = fv_task.clone();
                    let toast_task_c = toast_task.clone();

                    glib::MainContext::default().spawn_local(async move {
                        let (visible, curr_dir, msg, success) = gio::spawn_blocking(move || {
                            let mut mgr = mgr_c.lock().unwrap_or_else(|e| e.into_inner());
                            match mgr.add_files(&paths_c) {
                                Ok(()) => (
                                    mgr.visible_entries(),
                                    mgr.current_folder().to_path_buf(),
                                    format!("Added {} item(s) to archive", paths_c.len()),
                                    true,
                                ),
                                Err(e) => (
                                    Vec::new(),
                                    PathBuf::new(),
                                    format!("Failed to add items: {e}"),
                                    false,
                                ),
                            }
                        })
                        .await
                        .unwrap_or_else(|_| {
                            (
                                Vec::new(),
                                PathBuf::new(),
                                "Task panicked".to_string(),
                                false,
                            )
                        });

                        sl_task_c(false, &msg);
                        if success {
                            fv_task_c.set_entries(visible);
                            fv_task_c.update_folder_path(&curr_dir);
                        }
                        toast_task_c(&msg);
                    });
                }
            }
        });
    });
    ctx.window.add_action(&action);
}
