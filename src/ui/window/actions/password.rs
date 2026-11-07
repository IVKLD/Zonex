use super::context::ActionsContext;
use crate::ui::dialogs;
use gio::SimpleAction;
use gtk4::prelude::*;

pub fn register(ctx: &ActionsContext) {
    let manager = ctx.manager.clone();
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
        let toast_c = show_toast.clone();
        let sl_c = set_loading.clone();

        dialogs::prompt_password(
            &window,
            "Enter a password to encrypt all files in this archive:",
            move |pwd_opt| {
                let pwd = match pwd_opt {
                    Some(p) if !p.is_empty() => p,
                    _ => return,
                };

                sl_c(true, "Encrypting archive with password in background...");
                let mgr_task = mgr_c.clone();
                let sl_res = sl_c.clone();
                let toast_res = toast_c.clone();

                glib::MainContext::default().spawn_local(async move {
                    let res = gio::spawn_blocking(move || {
                        let mut mgr = mgr_task.lock().unwrap_or_else(|e| e.into_inner());
                        match mgr.encrypt_archive(&pwd) {
                            Ok(()) => {
                                Ok("✅ Archive successfully encrypted with password!".to_string())
                            }
                            Err(e) => Err(format!("Failed to encrypt archive: {}", e)),
                        }
                    })
                    .await
                    .unwrap_or_else(|_| Err("Encryption task panicked".to_string()));

                    sl_res(false, "Encryption complete");
                    match res {
                        Ok(msg) => toast_res(&msg),
                        Err(err_msg) => toast_res(&err_msg),
                    }
                });
            },
        );
    });
    ctx.window.add_action(&action);
}
