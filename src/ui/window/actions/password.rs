use super::context::ActionsContext;
use crate::ui::dialogs;
use gio::SimpleAction;
use gtk4::prelude::*;

pub fn register(ctx: &ActionsContext) {
    let manager = ctx.manager.clone();
    let file_view = ctx.file_view.clone();
    let window = ctx.window.clone();
    let show_toast = ctx.show_toast.clone();
    let set_loading = ctx.set_loading.clone();

    let action = SimpleAction::new("set_password", None);
    action.connect_activate(move |_, _| {
        if manager
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .current_path()
            .is_none()
        {
            show_toast("No archive currently open to protect");
            return;
        }

        let mgr_c = manager.clone();
        let fv_c = file_view.clone();
        let toast_c = show_toast.clone();
        let sl_c = set_loading.clone();

        dialogs::prompt_encrypt_archive(&window, move |result_opt| {
            let (pwd, method) = match result_opt {
                Some((p, m)) if !p.is_empty() => (p, m),
                _ => return,
            };

            sl_c(
                true,
                &format!("Encrypting archive with {} in background...", method.label()),
            );
            let mgr_task = mgr_c.clone();
            let fv_task = fv_c.clone();
            let sl_res = sl_c.clone();
            let toast_res = toast_c.clone();

            glib::MainContext::default().spawn_local(async move {
                let res = gio::spawn_blocking(move || {
                    let mut mgr = mgr_task.lock().unwrap_or_else(|e| e.into_inner());
                    match mgr.encrypt_archive_with_method(&pwd, method) {
                        Ok(()) => {
                            let visible = mgr.visible_entries();
                            let folder = mgr.current_folder().to_path_buf();
                            let msg = format!(
                                "✅ Archive successfully encrypted using {}!",
                                method.label()
                            );
                            Ok((visible, folder, msg))
                        }
                        Err(e) => Err(format!("Failed to encrypt archive: {}", e)),
                    }
                })
                .await
                .unwrap_or_else(|_| Err("Encryption task panicked".to_string()));

                sl_res(false, "Encryption complete");
                match res {
                    Ok((visible, folder, msg)) => {
                        fv_task.set_entries(visible);
                        fv_task.update_folder_path(&folder);
                        toast_res(&msg);
                    }
                    Err(err_msg) => toast_res(&err_msg),
                }
            });
        });
    });
    ctx.window.add_action(&action);
}
