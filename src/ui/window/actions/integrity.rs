use super::context::ActionsContext;
use crate::archive::ArchiveManager;
use crate::ui::dialogs;
use gio::SimpleAction;
use gtk4::prelude::*;

pub fn register(ctx: &ActionsContext) {
    let manager = ctx.manager.clone();
    let window = ctx.window.clone();
    let show_toast = ctx.show_toast.clone();
    let set_loading = ctx.set_loading.clone();

    let action = SimpleAction::new("test_integrity", None);
    action.connect_activate(move |_, _| {
        let path = {
            let mgr = manager.lock().unwrap_or_else(|e| e.into_inner());
            match mgr.current_path() {
                Some(p) => p.to_path_buf(),
                None => {
                    show_toast("No archive currently open");
                    return;
                }
            }
        };

        set_loading(true, "Verifying archive integrity in background...");

        let path_task = path.clone();
        let manager_retry = manager.clone();
        let toast_retry = show_toast.clone();
        let window_retry = window.clone();
        let sl_retry = set_loading.clone();
        let path_retry = path.clone();

        glib::MainContext::default().spawn_local(async move {
            let res = gio::spawn_blocking(move || {
                ArchiveManager::get_engine_for_path(&path_task)
                    .and_then(|engine| engine.verify_integrity(&path_task, None))
                    .map_err(|e| e.to_string())
            })
            .await
            .unwrap_or_else(|_| Err("Integrity verification task panicked".to_string()));

            sl_retry(false, "Integrity check finished");
            match res {
                Ok(report) => {
                    if report.failed_files.is_empty() {
                        toast_retry(&format!(
                            "✅ Integrity Check PASSED: {} file(s) verified",
                            report.ok_files
                        ));
                    } else {
                        toast_retry(&format!(
                            "❌ Integrity Check FAILED: {}/{} corrupted file(s)",
                            report.failed_files.len(),
                            report.total_files
                        ));
                    }
                }
                Err(_) => {
                    let sl_p = sl_retry.clone();
                    let toast_p = toast_retry.clone();
                    let path_p = path_retry.clone();
                    let _mgr_p = manager_retry.clone();

                    dialogs::prompt_password(
                        &window_retry,
                        "Integrity check requested password. Enter password:",
                        move |pwd| {
                            sl_p(true, "Verifying archive integrity with password...");
                            let path_task_p = path_p.clone();
                            let sl_p2 = sl_p.clone();
                            let toast_p2 = toast_p.clone();

                            glib::MainContext::default().spawn_local(async move {
                                let res_p = gio::spawn_blocking(move || {
                                    ArchiveManager::get_engine_for_path(&path_task_p)
                                        .and_then(|engine| {
                                            engine.verify_integrity(&path_task_p, pwd.as_deref())
                                        })
                                        .map_err(|e| e.to_string())
                                })
                                .await
                                .unwrap_or_else(|_| Err("Verification panicked".to_string()));

                                sl_p2(false, "Integrity check finished");
                                match res_p {
                                    Ok(report) => {
                                        if report.failed_files.is_empty() {
                                            toast_p2(&format!(
                                                "✅ Integrity Check PASSED: {} file(s) verified",
                                                report.ok_files
                                            ));
                                        } else {
                                            toast_p2(&format!(
                                                "❌ Integrity Check FAILED: {}/{} corrupted file(s)",
                                                report.failed_files.len(),
                                                report.total_files
                                            ));
                                        }
                                    }
                                    Err(err) => {
                                        toast_p2(&format!("Integrity check failed: {err}"));
                                    }
                                }
                            });
                        },
                    );
                }
            }
        });
    });
    ctx.window.add_action(&action);
}
