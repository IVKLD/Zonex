use super::context::ActionsContext;
use gio::SimpleAction;
use gtk4::prelude::*;

pub fn register(ctx: &ActionsContext) {
    let manager = ctx.manager.clone();
    let file_view = ctx.file_view.clone();
    let show_toast = ctx.show_toast.clone();
    let set_loading = ctx.set_loading.clone();

    let action = SimpleAction::new("delete_selected", None);
    action.connect_activate(move |_, _| {
        let selected_paths = file_view.get_selected_entry_paths();
        if selected_paths.is_empty() {
            show_toast("No files selected to delete");
            return;
        }

        set_loading(true, "Deleting selected item(s)...");

        let mgr_task = manager.clone();
        let selected_clone = selected_paths.clone();
        let fv_del = file_view.clone();
        let toast_del = show_toast.clone();
        let sl_del = set_loading.clone();

        glib::MainContext::default().spawn_local(async move {
            let (visible, folder, msg, success) = gio::spawn_blocking(move || {
                let mut mgr = mgr_task.lock().unwrap_or_else(|e| e.into_inner());
                match mgr.delete_selected_entries(&selected_clone) {
                    Ok(_) => {
                        let visible = mgr.visible_entries();
                        let folder = mgr.current_folder().to_path_buf();
                        let msg = format!("Deleted {} item(s)", selected_clone.len());
                        (visible, folder, msg, true)
                    }
                    Err(err) => {
                        let msg = format!("Failed to delete items: {}", err);
                        (Vec::new(), std::path::PathBuf::new(), msg, false)
                    }
                }
            })
            .await
            .unwrap_or_else(|_| {
                (
                    Vec::new(),
                    std::path::PathBuf::new(),
                    "Task panicked".to_string(),
                    false,
                )
            });

            sl_del(false, &msg);
            if success {
                fv_del.set_entries(visible);
                fv_del.update_folder_path(&folder);
            }
            toast_del(&msg);
        });
    });
    ctx.window.add_action(&action);
}
