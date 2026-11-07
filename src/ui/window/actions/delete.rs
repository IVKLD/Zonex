use super::context::ActionsContext;
use gio::SimpleAction;
use gtk4::prelude::*;
use libadwaita::prelude::*;

pub fn register(ctx: &ActionsContext) {
    let manager = ctx.manager.clone();
    let file_view = ctx.file_view.clone();
    let show_toast = ctx.show_toast.clone();
    let set_loading = ctx.set_loading.clone();
    let window = ctx.window.clone();

    let action = SimpleAction::new("delete_selected", None);
    action.connect_activate(move |_, _| {
        let selected_paths = file_view.get_selected_entry_paths();
        if selected_paths.is_empty() {
            show_toast("No files selected to delete");
            return;
        }

        let count = selected_paths.len();
        let item_str = if count == 1 {
            "1 item".to_string()
        } else {
            format!("{count} items")
        };

        let dialog = libadwaita::MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .heading("Delete from Archive?")
            .body(format!(
                "Are you sure you want to permanently delete {item_str}? This operation cannot be undone."
            ))
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("delete", "Delete");
        dialog.set_response_appearance("delete", libadwaita::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        let mgr_task = manager.clone();
        let selected_clone = selected_paths;
        let fv_del = file_view.clone();
        let toast_del = show_toast.clone();
        let sl_del = set_loading.clone();

        let state_cell = std::cell::RefCell::new(Some((
            mgr_task,
            selected_clone,
            fv_del,
            toast_del,
            sl_del,
        )));

        dialog.connect_response(None, move |d, response| {
            d.close();
            if response != "delete" {
                return;
            }

            let Some((mgr_task, selected_clone, fv_del, toast_del, sl_del)) =
                state_cell.borrow_mut().take()
            else {
                return;
            };

            sl_del(true, "Deleting selected item(s)...");

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

        dialog.present();
    });
    ctx.window.add_action(&action);
}
