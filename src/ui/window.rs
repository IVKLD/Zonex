use super::file_view::FileView;
use super::status_bar::StatusBar;
use crate::archive::ArchiveManager;
use crate::dnd::{setup_drag_source, setup_drop_target};
use crate::utils::TempCache;
use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{
    gdk, Box as GtkBox, Button, EventControllerKey, Orientation, Spinner,
};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct ZonexWindow {
    window: adw::ApplicationWindow,
    _manager: Arc<Mutex<ArchiveManager>>,
    _temp_cache: Rc<TempCache>,
    open_path_fn: Rc<dyn Fn(PathBuf)>,
}

impl ZonexWindow {
    pub fn new(app: &adw::Application) -> Self {
        let manager = Arc::new(Mutex::new(ArchiveManager::new()));
        let temp_cache = Rc::new(TempCache::new().expect("Failed to initialize temp cache"));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Zonex - Wayland Archive Manager")
            .icon_name("io.github.zonex.ArchiveManager")
            .default_width(800)
            .default_height(550)
            .build();

        // Регистрируем локальную директорию иконки в IconTheme для dev-режима
        if let Some(display) = gdk::Display::default() {
            let icon_theme = gtk4::IconTheme::for_display(&display);
            let local_icon_dir = std::env::current_dir().unwrap_or_default().join("data/icons");
            if local_icon_dir.exists() {
                icon_theme.add_search_path(local_icon_dir);
            }

            let provider = gtk4::CssProvider::new();
            provider.load_from_string(
                "
                .password-dialog-scrim {
                    background-color: rgba(0, 0, 0, 0.55);
                }
                .password-dialog-card {
                    background-color: @dialog_bg_color;
                    color: @dialog_fg_color;
                    border-radius: 16px;
                    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.45);
                    border: 1px solid rgba(255, 255, 255, 0.15);
                    padding: 24px;
                }
                "
            );
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let toast_overlay = adw::ToastOverlay::new();

        let header_bar = adw::HeaderBar::new();
        let open_btn = Button::with_label("Open Archive");
        let spinner = Spinner::new();
        spinner.set_visible(false);

        let menu = gio::Menu::new();
        menu.append(Some("New Archive..."), Some("win.new_archive"));
        menu.append(Some("Protect with Password..."), Some("win.set_password"));
        menu.append(Some("Test Integrity"), Some("win.test_integrity"));
        menu.append(Some("Delete Selected"), Some("win.delete_selected"));
        menu.append(Some("Select All"), Some("win.select_all"));
        menu.append(Some("Deselect All"), Some("win.deselect_all"));

        let menu_button = gtk4::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .build();

        header_bar.pack_start(&open_btn);
        header_bar.pack_end(&menu_button);
        header_bar.pack_end(&spinner);

        let main_box = GtkBox::new(Orientation::Vertical, 0);
        main_box.append(&header_bar);

        let file_view = Rc::new(FileView::new());
        main_box.append(file_view.widget());

        let status_bar = Rc::new(StatusBar::new());
        main_box.append(status_bar.widget());

        let window_overlay = gtk4::Overlay::new();
        window_overlay.set_child(Some(&main_box));

        let pwd_overlay = Rc::new(crate::ui::dialogs::PasswordOverlayWidget::new());
        window_overlay.add_overlay(pwd_overlay.widget());

        toast_overlay.set_child(Some(&window_overlay));
        window.set_content(Some(&toast_overlay));

        // Вспомогательная функция отображения Toast
        let toast_overlay_c = toast_overlay.clone();
        let show_toast = move |msg: &str| {
            let toast = adw::Toast::new(msg);
            toast.set_timeout(3);
            toast_overlay_c.add_toast(toast);
        };
        let show_toast_rc = Rc::new(show_toast);

        // Вспомогательная функция spinner + статус
        let spinner_c = spinner.clone();
        let status_c = status_bar.clone();
        let set_loading = move |loading: bool, msg: &str| {
            spinner_c.set_visible(loading);
            if loading {
                spinner_c.start();
            } else {
                spinner_c.stop();
            }
            status_c.set_text(msg);
        };
        let set_loading_rc = Rc::new(set_loading);

        // Действия окна для меню и контекстного меню
        let fv_select = file_view.clone();
        let action_select_all = gio::SimpleAction::new("select_all", None);
        action_select_all.connect_activate(move |_, _| {
            fv_select.select_all();
        });
        window.add_action(&action_select_all);

        let fv_deselect = file_view.clone();
        let action_deselect_all = gio::SimpleAction::new("deselect_all", None);
        action_deselect_all.connect_activate(move |_, _| {
            fv_deselect.deselect_all();
        });
        window.add_action(&action_deselect_all);

        // ── Action: New Archive ──
        let window_new = window.clone();
        let action_new_archive = gio::SimpleAction::new("new_archive", None);
        action_new_archive.connect_activate(move |_, _| {
            let dialog = gtk4::FileDialog::builder()
                .title("Create New Archive")
                .initial_name("archive.zip")
                .build();

            dialog.save(Some(&window_new), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(path) = file.path() {
                        thread::spawn(move || {
                            let _ = std::fs::File::create(&path);
                        });
                    }
                }
            });
        });
        window.add_action(&action_new_archive);

        // ── Action: Test Integrity (Async, Non-Blocking) ──
        let manager_integrity = manager.clone();
        let window_integrity = window.clone();
        let show_toast_integrity = show_toast_rc.clone();
        let set_loading_integrity = set_loading_rc.clone();
        let action_test_integrity = gio::SimpleAction::new("test_integrity", None);
        action_test_integrity.connect_activate(move |_, _| {
            let path = {
                let mgr = manager_integrity.lock().unwrap_or_else(|e| e.into_inner());
                match mgr.current_path() {
                    Some(p) => p.to_path_buf(),
                    None => {
                        show_toast_integrity("No archive currently open");
                        return;
                    }
                }
            };

            set_loading_integrity(true, "Verifying archive integrity in background...");

            let (tx, rx) = mpsc::channel::<Result<crate::archive::IntegrityReport, String>>();
            let path_task = path.clone();

            thread::spawn(move || {
                let engine_res = ArchiveManager::get_engine_for_path(&path_task);
                match engine_res {
                    Ok(engine) => {
                        let res = engine.verify_integrity(&path_task, None).map_err(|e| e.to_string());
                        let _ = tx.send(res);
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                    }
                }
            });

            let manager_retry = manager_integrity.clone();
            let toast_retry = show_toast_integrity.clone();
            let window_retry = window_integrity.clone();
            let sl_retry = set_loading_integrity.clone();
            let path_retry = path.clone();

            glib::timeout_add_local(Duration::from_millis(16), move || {
                if let Ok(res) = rx.try_recv() {
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
                            use crate::ui::dialogs::Dialogs;
                            let sl_p = sl_retry.clone();
                            let toast_p = toast_retry.clone();
                            let path_p = path_retry.clone();
                            let _mgr_p = manager_retry.clone();

                            Dialogs::prompt_password(
                                &window_retry,
                                "Integrity check requested password. Enter password:",
                                move |pwd| {
                                    sl_p(true, "Verifying archive integrity with password...");
                                    let (tx_p, rx_p) = mpsc::channel::<Result<crate::archive::IntegrityReport, String>>();
                                    let path_task_p = path_p.clone();

                                    thread::spawn(move || {
                                        if let Ok(engine) = ArchiveManager::get_engine_for_path(&path_task_p) {
                                            let res = engine.verify_integrity(&path_task_p, pwd.as_deref()).map_err(|e| e.to_string());
                                            let _ = tx_p.send(res);
                                        }
                                    });

                                    glib::timeout_add_local(Duration::from_millis(16), move || {
                                        if let Ok(res_p) = rx_p.try_recv() {
                                            sl_p(false, "Integrity check finished");
                                            match res_p {
                                                Ok(report) => {
                                                    if report.failed_files.is_empty() {
                                                        toast_p(&format!("✅ Integrity Check PASSED: {} file(s) verified", report.ok_files));
                                                    } else {
                                                        toast_p(&format!("❌ Integrity Check FAILED: {}/{} error(s)", report.failed_files.len(), report.total_files));
                                                    }
                                                }
                                                Err(_) => {
                                                    toast_p("❌ Integrity Check FAILED: Password incorrect or corrupt format");
                                                }
                                            }
                                            glib::ControlFlow::Break
                                        } else {
                                            glib::ControlFlow::Continue
                                        }
                                    });
                                },
                            );
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        });
        window.add_action(&action_test_integrity);

        // ── Action: Delete Selected (Async, Non-Blocking) ──
        let manager_delete = manager.clone();
        let file_view_delete = file_view.clone();
        let show_toast_delete = show_toast_rc.clone();
        let set_loading_delete = set_loading_rc.clone();
        let action_delete_selected = gio::SimpleAction::new("delete_selected", None);
        action_delete_selected.connect_activate(move |_, _| {
            let selected_paths = file_view_delete.get_selected_entry_paths();
            if selected_paths.is_empty() {
                show_toast_delete("No files selected to delete");
                return;
            }

            set_loading_delete(true, "Deleting selected item(s)...");

            let (tx, rx) = mpsc::channel::<(
                Vec<crate::archive::ArchiveEntry>,
                PathBuf,
                String,
                bool,
            )>();

            let mgr_task = manager_delete.clone();
            let selected_clone = selected_paths.clone();

            thread::spawn(move || {
                let mut mgr = mgr_task.lock().unwrap_or_else(|e| e.into_inner());
                match mgr.delete_selected_entries(&selected_clone) {
                    Ok(_) => {
                        let visible = mgr.visible_entries();
                        let folder = mgr.current_folder().to_path_buf();
                        let msg = format!("Deleted {} item(s)", selected_clone.len());
                        let _ = tx.send((visible, folder, msg, true));
                    }
                    Err(err) => {
                        let msg = format!("Failed to delete items: {}", err);
                        let _ = tx.send((Vec::new(), PathBuf::new(), msg, false));
                    }
                }
            });

            let fv_del = file_view_delete.clone();
            let toast_del = show_toast_delete.clone();
            let sl_del = set_loading_delete.clone();

            glib::timeout_add_local(Duration::from_millis(16), move || {
                if let Ok((entries, folder, msg, success)) = rx.try_recv() {
                    sl_del(false, &msg);
                    if success {
                        fv_del.set_entries(entries);
                        fv_del.update_folder_path(&folder);
                    }
                    toast_del(&msg);
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        });
        window.add_action(&action_delete_selected);

        // ── Action: Extract Selected (Async, Non-Blocking) ──
        let manager_extract = manager.clone();
        let file_view_extract = file_view.clone();
        let show_toast_extract = show_toast_rc.clone();
        let set_loading_extract = set_loading_rc.clone();
        let window_extract = window.clone();

        let action_extract_selected = gio::SimpleAction::new("extract_selected", None);
        action_extract_selected.connect_activate(move |_, _| {
            let selected_paths = file_view_extract.get_selected_entry_paths();
            if selected_paths.is_empty() {
                show_toast_extract("No files selected to extract");
                return;
            }

            let dialog = gtk4::FileDialog::builder()
                .title("Select Destination Folder")
                .build();

            let mgr_task = manager_extract.clone();
            let sl_task = set_loading_extract.clone();
            let toast_task = show_toast_extract.clone();
            let selected_task = selected_paths.clone();

            dialog.select_folder(Some(&window_extract), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(dest_dir) = file.path() {
                        sl_task(true, "Extracting selected files...");
                        let (tx, rx) = mpsc::channel::<Result<usize, String>>();

                        let mgr_c = mgr_task.clone();
                        let sel_c = selected_task.clone();
                        let dest_c = dest_dir.clone();
                        let dest_toast = dest_dir.clone();

                        thread::spawn(move || {
                            let mgr = mgr_c.lock().unwrap_or_else(|e| e.into_inner());
                            match mgr.extract_selected(&sel_c, &dest_c) {
                                Ok(extracted) => {
                                    let _ = tx.send(Ok(extracted.len()));
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(e.to_string()));
                                }
                            }
                        });

                        let sl_res = sl_task.clone();
                        let toast_res = toast_task.clone();

                        glib::timeout_add_local(Duration::from_millis(16), move || {
                            if let Ok(res) = rx.try_recv() {
                                sl_res(false, "Extraction finished");
                                match res {
                                    Ok(n) => {
                                        toast_res(&format!("Extracted {} item(s) to {}", n, dest_toast.display()));
                                    }
                                    Err(e) => {
                                        toast_res(&format!("Failed to extract: {}", e));
                                    }
                                }
                                glib::ControlFlow::Break
                            } else {
                                glib::ControlFlow::Continue
                            }
                        });
                    }
                }
            });
        });
        window.add_action(&action_extract_selected);

        // ── Action: Add Files (Async, Non-Blocking) ──
        let manager_add = manager.clone();
        let file_view_add = file_view.clone();
        let show_toast_add = show_toast_rc.clone();
        let set_loading_add = set_loading_rc.clone();
        let window_add = window.clone();

        let action_add_files = gio::SimpleAction::new("add_files", None);
        action_add_files.connect_activate(move |_, _| {
            let mgr_check = manager_add.lock().unwrap_or_else(|e| e.into_inner());
            if mgr_check.current_path().is_none() {
                show_toast_add("No archive open to add files to");
                return;
            }
            drop(mgr_check);

            let dialog = gtk4::FileDialog::builder()
                .title("Add Files to Archive")
                .build();

            let mgr_task = manager_add.clone();
            let fv_task = file_view_add.clone();
            let sl_task = set_loading_add.clone();
            let toast_task = show_toast_add.clone();

            dialog.open_multiple(Some(&window_add), gio::Cancellable::NONE, move |res| {
                if let Ok(files) = res {
                    let mut paths = Vec::new();
                    for i in 0..files.n_items() {
                        if let Some(file) = files.item(i).and_downcast::<gio::File>() {
                            if let Some(path) = file.path() {
                                paths.push(path);
                            }
                        }
                    }

                    if !paths.is_empty() {
                        sl_task(true, "Adding files to archive...");
                        let (tx, rx) = mpsc::channel::<(Vec<crate::archive::ArchiveEntry>, PathBuf, String, bool)>();

                        let mgr_c = mgr_task.clone();
                        let paths_c = paths.clone();

                        thread::spawn(move || {
                            let mut mgr = mgr_c.lock().unwrap_or_else(|e| e.into_inner());
                            let msg;
                            let success;
                            let mut visible = Vec::new();
                            let mut curr_dir = PathBuf::new();

                            match mgr.add_files(&paths_c) {
                                Ok(()) => {
                                    visible = mgr.visible_entries();
                                    curr_dir = mgr.current_folder().to_path_buf();
                                    msg = format!("Added {} item(s) to archive", paths_c.len());
                                    success = true;
                                }
                                Err(e) => {
                                    msg = format!("Failed to add items: {}", e);
                                    success = false;
                                }
                            }

                            let _ = tx.send((visible, curr_dir, msg, success));
                        });

                        let sl_task_c = sl_task.clone();
                        let fv_task_c = fv_task.clone();
                        let toast_task_c = toast_task.clone();
                        glib::timeout_add_local(Duration::from_millis(16), move || {
                            if let Ok((visible, curr_dir, msg, success)) = rx.try_recv() {
                                sl_task_c(false, &msg);
                                if success {
                                    fv_task_c.set_entries(visible);
                                    fv_task_c.update_folder_path(&curr_dir);
                                }
                                toast_task_c(&msg);
                                glib::ControlFlow::Break
                            } else {
                                glib::ControlFlow::Continue
                            }
                        });
                    }
                }
            });
        });
        window.add_action(&action_add_files);

        // ── Action: Open Entry / Folder ──
        let manager_open_e = manager.clone();
        let file_view_open_e = file_view.clone();
        let show_toast_open_e = show_toast_rc.clone();
        let temp_cache_open_e = temp_cache.clone();

        let action_open_entry = gio::SimpleAction::new("open_entry", None);
        action_open_entry.connect_activate(move |_, _| {
            let selected_rows = file_view_open_e.list_box().selected_rows();
            if selected_rows.len() == 1 {
                let idx = selected_rows[0].index() as usize;
                if let Some(entry) = file_view_open_e.get_entry_at_index(idx) {
                    if entry.is_dir {
                        let mut mgr = manager_open_e.lock().unwrap_or_else(|e| e.into_inner());
                        mgr.enter_folder(entry.path);
                        let visible = mgr.visible_entries();
                        let folder = mgr.current_folder().to_path_buf();
                        file_view_open_e.set_entries(visible);
                        file_view_open_e.update_folder_path(&folder);
                    } else {
                        let mgr = manager_open_e.lock().unwrap_or_else(|e| e.into_inner());
                        let stage_dir = temp_cache_open_e.stage_dir();
                        let toast_c = show_toast_open_e.clone();
                        if let Ok(extracted) = mgr.extract_selected(&[entry.path], &stage_dir) {
                            if let Some(first) = extracted.first() {
                                let uri = format!("file://{}", first.display());
                                let _ = gio::AppInfo::launch_default_for_uri(&uri, gio::AppLaunchContext::NONE);
                            }
                        } else {
                            toast_c("Failed to open file entry");
                        }
                    }
                }
            }
        });
        window.add_action(&action_open_entry);

        // ── Action: Extract Here (Async, Non-Blocking) ──
        let manager_extract_here = manager.clone();
        let file_view_extract_here = file_view.clone();
        let show_toast_extract_here = show_toast_rc.clone();
        let set_loading_extract_here = set_loading_rc.clone();

        let action_extract_here = gio::SimpleAction::new("extract_here", None);
        action_extract_here.connect_activate(move |_, _| {
            let selected_paths = file_view_extract_here.get_selected_entry_paths();
            if selected_paths.is_empty() {
                show_toast_extract_here("No files selected to extract");
                return;
            }

            let dest_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            set_loading_extract_here(true, "Extracting files here...");

            let (tx, rx) = mpsc::channel::<Result<usize, String>>();
            let mgr_task = manager_extract_here.clone();
            let selected_task = selected_paths.clone();
            let dest_task = dest_dir.clone();

            thread::spawn(move || {
                let mgr = mgr_task.lock().unwrap_or_else(|e| e.into_inner());
                match mgr.extract_selected(&selected_task, &dest_task) {
                    Ok(extracted) => {
                        let _ = tx.send(Ok(extracted.len()));
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                    }
                }
            });

            let sl_res = set_loading_extract_here.clone();
            let toast_res = show_toast_extract_here.clone();

            glib::timeout_add_local(Duration::from_millis(16), move || {
                if let Ok(res) = rx.try_recv() {
                    sl_res(false, "Extraction finished");
                    match res {
                        Ok(n) => {
                            toast_res(&format!("Extracted {} file(s) to {}", n, dest_dir.display()));
                        }
                        Err(e) => {
                            toast_res(&format!("Failed to extract: {}", e));
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        });
        window.add_action(&action_extract_here);

        // ── Action: Entry Properties ──
        let file_view_props = file_view.clone();
        let window_props = window.clone();
        let show_toast_props = show_toast_rc.clone();

        let action_entry_properties = gio::SimpleAction::new("entry_properties", None);
        action_entry_properties.connect_activate(move |_, _| {
            let selected_rows = file_view_props.list_box().selected_rows();
            if selected_rows.len() == 1 {
                let idx = selected_rows[0].index() as usize;
                if let Some(entry) = file_view_props.get_entry_at_index(idx) {
                    use crate::ui::dialogs::Dialogs;
                    Dialogs::show_entry_properties(&window_props, &entry);
                }
            } else if selected_rows.is_empty() {
                show_toast_props("Select an item to view properties");
            } else {
                show_toast_props(&format!("{} items selected", selected_rows.len()));
            }
        });
        window.add_action(&action_entry_properties);

        // ── Action: Quick Preview (Spacebar / Context Menu) ──
        let manager_prev = manager.clone();
        let file_view_prev = file_view.clone();
        let temp_cache_prev = temp_cache.clone();
        let window_prev = window.clone();
        let show_toast_prev = show_toast_rc.clone();

        let action_quick_preview = gio::SimpleAction::new("quick_preview", None);
        action_quick_preview.connect_activate(move |_, _| {
            let selected_rows = file_view_prev.list_box().selected_rows();
            if selected_rows.len() == 1 {
                let idx = selected_rows[0].index() as usize;
                if let Some(entry) = file_view_prev.get_entry_at_index(idx) {
                    if !entry.is_dir {
                        let mgr = manager_prev.lock().unwrap_or_else(|e| e.into_inner());
                        let stage_dir = temp_cache_prev.stage_dir();
                        let toast_c = show_toast_prev.clone();
                        let win_c = window_prev.clone();
                        if let Ok(extracted) = mgr.extract_selected(&[entry.path], &stage_dir) {
                            if let Some(file_path) = extracted.first() {
                                crate::ui::PreviewWindow::show(&win_c, file_path);
                            }
                        } else {
                            toast_c("Failed to extract file for preview");
                        }
                    }
                }
            }
        });
        window.add_action(&action_quick_preview);

        // ── Action: Set Password / Encrypt Archive (Async, Non-Blocking) ──
        let manager_pwd = manager.clone();
        let window_pwd = window.clone();
        let show_toast_pwd = show_toast_rc.clone();
        let set_loading_pwd = set_loading_rc.clone();

        let action_set_password = gio::SimpleAction::new("set_password", None);
        action_set_password.connect_activate(move |_, _| {
            let has_open = {
                let mgr = manager_pwd.lock().unwrap_or_else(|e| e.into_inner());
                mgr.current_path().is_some()
            };

            if !has_open {
                show_toast_pwd("No archive currently open to protect");
                return;
            }

            use crate::ui::dialogs::Dialogs;
            let mgr_c = manager_pwd.clone();
            let toast_c = show_toast_pwd.clone();
            let sl_c = set_loading_pwd.clone();

            Dialogs::prompt_password(
                &window_pwd,
                "Enter a password to encrypt all files in this archive:",
                move |pwd_opt| {
                    let pwd = match pwd_opt {
                        Some(p) if !p.is_empty() => p,
                        _ => return,
                    };

                    sl_c(true, "Encrypting archive with password in background...");
                    let (tx, rx) = mpsc::channel::<Result<String, String>>();
                    let mgr_task = mgr_c.clone();

                    thread::spawn(move || {
                        let mut mgr = mgr_task.lock().unwrap_or_else(|e| e.into_inner());
                        match mgr.encrypt_archive(&pwd) {
                            Ok(()) => {
                                let _ = tx.send(Ok("✅ Archive successfully encrypted with password!".to_string()));
                            }
                            Err(e) => {
                                let _ = tx.send(Err(format!("Failed to encrypt archive: {}", e)));
                            }
                        }
                    });

                    let sl_res = sl_c.clone();
                    let toast_res = toast_c.clone();

                    glib::timeout_add_local(Duration::from_millis(16), move || {
                        if let Ok(res) = rx.try_recv() {
                            sl_res(false, "Encryption complete");
                            match res {
                                Ok(msg) => toast_res(&msg),
                                Err(err_msg) => toast_res(&err_msg),
                            }
                            glib::ControlFlow::Break
                        } else {
                            glib::ControlFlow::Continue
                        }
                    });
                },
            );
        });
        window.add_action(&action_set_password);

        // Навигация по двойному клику на папку или открытию файла
        let manager_nav = manager.clone();
        let file_view_nav = file_view.clone();
        file_view.list_box().connect_row_activated(move |_, row| {
            let idx = row.index() as usize;
            if let Some(entry) = file_view_nav.get_entry_at_index(idx) {
                if entry.is_dir {
                    let mut mgr = manager_nav.lock().unwrap_or_else(|e| e.into_inner());
                    mgr.enter_folder(entry.path);
                    let visible = mgr.visible_entries();
                    let current_folder = mgr.current_folder().to_path_buf();
                    file_view_nav.set_entries(visible);
                    file_view_nav.update_folder_path(&current_folder);
                    file_view_nav.set_forward_sensitive(false);
                } else {
                    let _ = file_view_nav.widget().activate_action("win.quick_preview", None);
                }
            }
        });

        // Кнопка «Назад» (и боковая клавиша мыши Back / Button 8/6)
        let manager_up = manager.clone();
        let file_view_up = file_view.clone();
        file_view.up_button().connect_clicked(move |_| {
            let mut mgr = manager_up.lock().unwrap_or_else(|e| e.into_inner());
            if mgr.navigate_back() {
                let visible = mgr.visible_entries();
                let current_folder = mgr.current_folder().to_path_buf();
                file_view_up.set_entries(visible);
                file_view_up.update_folder_path(&current_folder);
                file_view_up.set_forward_sensitive(true);
            }
        });

        // Кнопка «Вперёд» (и боковая клавиша мыши Forward / Button 9/7)
        let manager_fw = manager.clone();
        let file_view_fw = file_view.clone();
        file_view.forward_button().connect_clicked(move |_| {
            let mut mgr = manager_fw.lock().unwrap_or_else(|e| e.into_inner());
            if mgr.navigate_forward() {
                let visible = mgr.visible_entries();
                let current_folder = mgr.current_folder().to_path_buf();
                file_view_fw.set_entries(visible);
                file_view_fw.update_folder_path(&current_folder);
            }
        });
        // ─────────────────────────────────────────────────────────────────────
        // Процедура открытия архива по пути (с неблокирующим запросом пароля)
        // ─────────────────────────────────────────────────────────────────────
        let manager_open = manager.clone();
        let file_view_open = file_view.clone();
        let set_loading_open = set_loading_rc.clone();
        let show_toast_open = show_toast_rc.clone();
        let temp_cache_open = temp_cache.clone();
        let window_open = window.clone();

        let pwd_overlay_open = pwd_overlay.clone();

        let do_open_path_with_pwd = move |path: PathBuf, password: Option<String>, _was_wrong_pwd: bool| {
            set_loading_open(true, "Opening archive (mounting FUSE...)");
            temp_cache_open.clear_stage();

            let mgr_task = manager_open.clone();
            let mgr_verify_base = manager_open.clone();
            let fv_task = file_view_open.clone();
            let sl_task = set_loading_open.clone();
            let toast_task = show_toast_open.clone();
            let _window_task = window_open.clone();
            let pwd_overlay_task = pwd_overlay_open.clone();

            let (tx, rx) = mpsc::channel::<(
                Vec<crate::archive::ArchiveEntry>,
                String,
                bool,
                f64,
                bool, // has_fuse
                PathBuf,
            )>();

            let pwd_task = password.clone();
            let path_task = path.clone();

            thread::spawn(move || {
                let start_time = Instant::now();
                let mut entries_res = Vec::new();
                let msg;
                let success;

                let mut mgr = mgr_task.lock().unwrap_or_else(|e| e.into_inner());
                match mgr.open_archive_with_password(&path_task, pwd_task.as_deref()) {
                    Ok(entries) => {
                        crate::utils::RecentManager::add_recent(&path_task);
                        entries_res = entries;
                        msg = format!(
                            "Opened: {}",
                            path_task.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        );
                        success = true;
                    }
                    Err(err) => {
                        msg = format!("Failed to open: {}", err);
                        success = false;
                    }
                }

                let has_fuse = mgr.fuse_mount_path().is_some();
                let duration = start_time.elapsed().as_secs_f64();
                let _ = tx.send((entries_res, msg, success, duration, has_fuse, path_task));
            });

            glib::timeout_add_local(Duration::from_millis(16), move || {
                if let Ok((entries, msg, success, duration, has_fuse, open_p)) = rx.try_recv() {
                    if success {
                        let status_suffix = if has_fuse { " [FUSE]" } else { " [tmpfs]" };
                        sl_task(false, &format!("{}{}", msg, status_suffix));
                        fv_task.set_entries(entries);
                        fv_task.update_folder_path(Path::new(""));
                        let dnd_info = if has_fuse {
                            "⚡ DnD via FUSE (zero-latency)"
                        } else {
                            "⚠ DnD via tmpfs (install libfuse3 for streaming)"
                        };
                        toast_task(&format!("{} ({:.2}s) — {}", msg, duration, dnd_info));
                    } else {
                        sl_task(false, "Password required or invalid format");

                        if !pwd_overlay_task.is_visible() {
                            let mgr_verify = mgr_verify_base.clone();
                            let fv_verify = fv_task.clone();
                            let sl_verify = sl_task.clone();
                            let toast_verify = toast_task.clone();
                            let open_path_target = open_p.clone();

                            pwd_overlay_task.prompt(move |pwd_input: String, on_done| {
                                    let mgr_v = mgr_verify.clone();
                                    let fv_v = fv_verify.clone();
                                    let sl_v = sl_verify.clone();
                                    let toast_v = toast_verify.clone();
                                    let path_v = open_path_target.clone();
                                    let on_done_cb = on_done.clone();

                                    let (tx, rx) = mpsc::channel::<(
                                        Vec<crate::archive::ArchiveEntry>,
                                        String,
                                        bool,
                                        f64,
                                        bool, // has_fuse
                                    )>();

                                    thread::spawn(move || {
                                        let start_time = Instant::now();
                                        let mut entries_res = Vec::new();
                                        let msg;
                                        let success;

                                        let mut mgr = mgr_v.lock().unwrap_or_else(|e| e.into_inner());
                                        match mgr.open_archive_with_password(&path_v, Some(&pwd_input)) {
                                            Ok(entries) => {
                                                crate::utils::RecentManager::add_recent(&path_v);
                                                entries_res = entries;
                                                msg = format!(
                                                    "Opened: {}",
                                                    path_v.file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy()
                                                );
                                                success = true;
                                            }
                                            Err(err) => {
                                                msg = format!("Failed to open: {}", err);
                                                success = false;
                                            }
                                        }

                                        let has_fuse = mgr.fuse_mount_path().is_some();
                                        let duration = start_time.elapsed().as_secs_f64();
                                        let _ = tx.send((entries_res, msg, success, duration, has_fuse));
                                    });

                                    glib::timeout_add_local(Duration::from_millis(16), move || {
                                        if let Ok((entries, msg, success, duration, has_fuse)) = rx.try_recv() {
                                             if success {
                                                let status_suffix = if has_fuse { " [FUSE]" } else { " [tmpfs]" };
                                                sl_v(false, &format!("{}{}", msg, status_suffix));
                                                fv_v.set_entries(entries);
                                                fv_v.update_folder_path(Path::new(""));
                                                let dnd_info = if has_fuse {
                                                    "⚡ DnD via FUSE (zero-latency)"
                                                } else {
                                                    "⚠ DnD via tmpfs (install libfuse3 for streaming)"
                                                };
                                                toast_v(&format!("{} ({:.2}s) — {}", msg, duration, dnd_info));
                                                on_done_cb(true, "");
                                            } else {
                                                sl_v(false, "Password required or invalid format");
                                                on_done_cb(false, "❌ Incorrect password. Please try another password.");
                                            }
                                            glib::ControlFlow::Break
                                        } else {
                                            glib::ControlFlow::Continue
                                        }
                                    });
                                },
                            );
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        };

        let do_open_pwd_rc: Rc<dyn Fn(PathBuf, Option<String>, bool)> = Rc::new(do_open_path_with_pwd);

        let do_open_rc: Rc<dyn Fn(PathBuf)> = {
            let pwd_fn = do_open_pwd_rc.clone();
            Rc::new(move |p| pwd_fn(p, None, false))
        };

        // ─────────────────────────────────────────────────────────────────────
        // DRAG IN (Файловый менеджер → Zonex): Файлы читаются в фоновом потоке
        // ─────────────────────────────────────────────────────────────────────
        let manager_drop = manager.clone();
        let file_view_drop = file_view.clone();
        let set_loading_drop = set_loading_rc.clone();
        let show_toast_drop = show_toast_rc.clone();
        let do_open_drop = do_open_rc.clone();

        setup_drop_target(&window, move |paths| {
            if paths.is_empty() {
                return;
            }

            // Быстрая проверка на самокопирование еще до формата и дочерних потоков
            {
                let mgr_check = manager_drop.lock().unwrap_or_else(|e| e.into_inner());
                let is_self_drop = if let Some(mount_path) = mgr_check.fuse_mount_path() {
                    paths.iter().any(|p| p.starts_with(mount_path))
                } else {
                    false
                } || paths.iter().any(|p| Some(p.as_path()) == mgr_check.current_path());

                if is_self_drop {
                    tracing::info!("Drop-in ignored: files belong to the currently open archive");
                    return;
                }
            }

            let first_path = paths[0].clone();
            let is_single = paths.len() == 1;
            let is_archive = ArchiveManager::get_engine_for_path(&first_path).is_ok();

            if is_single && is_archive {
                let mgr_check = manager_drop.lock().unwrap_or_else(|e| e.into_inner());
                if mgr_check.current_path().is_none() {
                    drop(mgr_check);
                    do_open_drop(first_path);
                    return;
                }
            }

            let manager_task = manager_drop.clone();
            let fv_task = file_view_drop.clone();
            let sl_task = set_loading_drop.clone();
            let toast_task = show_toast_drop.clone();

            sl_task(true, "Processing files in background...");

            let (tx, rx) = mpsc::channel::<(
                Vec<crate::archive::ArchiveEntry>,
                PathBuf,
                String,
                bool,
                f64,
            )>();

            thread::spawn(move || {
                let start_time = Instant::now();
                let result_msg;
                let mut visible_entries = Vec::new();
                let mut current_folder = PathBuf::new();
                let mut success = false;

                let mut mgr = manager_task.lock().unwrap_or_else(|e| e.into_inner());
                let has_open = mgr.current_path().is_some();

                if has_open {
                    match mgr.add_files(&paths) {
                        Ok(()) => {
                            visible_entries = mgr.visible_entries();
                            current_folder = mgr.current_folder().to_path_buf();
                            result_msg = format!("Added {} item(s) to open archive", paths.len());
                            success = true;
                        }
                        Err(err) => {
                            tracing::error!("Error adding files to archive: {:?}", err);
                            result_msg = format!("Error adding files: {}", err);
                        }
                    }
                } else {
                    let default_archive = paths[0]
                        .parent()
                        .unwrap_or(Path::new("."))
                        .join("archive.zip");
                    match mgr.create_archive(&default_archive, &paths) {
                        Ok(()) => {
                            visible_entries = mgr.visible_entries();
                            current_folder = mgr.current_folder().to_path_buf();
                            result_msg =
                                format!("Created archive: {}", default_archive.display());
                            success = true;
                        }
                        Err(err) => {
                            tracing::error!(
                                "Error creating archive {:?}: {:?}",
                                default_archive,
                                err
                            );
                            result_msg = format!("Error creating archive: {}", err);
                        }
                    }
                }

                let duration = start_time.elapsed().as_secs_f64();
                let _ = tx.send((visible_entries, current_folder, result_msg, success, duration));
            });

            glib::timeout_add_local(Duration::from_millis(16), move || {
                if let Ok((entries, folder, msg, success, duration)) = rx.try_recv() {
                    sl_task(false, &msg);
                    if success {
                        fv_task.set_entries(entries);
                        fv_task.update_folder_path(&folder);
                        toast_task(&format!("{} ({:.2}s)", msg, duration));
                    } else {
                        toast_task(&msg);
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        });

        // ─────────────────────────────────────────────────────────────────────
        // DRAG OUT (Zonex → Файловый менеджер): Zero-latency via FUSE
        // ─────────────────────────────────────────────────────────────────────
        let manager_drag = manager.clone();
        let temp_cache_drag = temp_cache.clone();
        let file_view_drag = file_view.clone();
        let show_toast_end = show_toast_rc.clone();

        setup_drag_source(
            file_view.list_box(),
            move |_, y| {
                let row = match file_view_drag.list_box().row_at_y(y as i32) {
                    Some(r) => r,
                    None => return Vec::new(),
                };

                if !row.is_selected() {
                    file_view_drag.list_box().unselect_all();
                    file_view_drag.list_box().select_row(Some(&row));
                }

                let selected = file_view_drag.get_selected_entry_paths();
                if selected.is_empty() {
                    return Vec::new();
                }

                let mgr = manager_drag.lock().unwrap_or_else(|e| e.into_inner());

                if let Some(mount_path) = mgr.fuse_mount_path() {
                    let drag_paths: Vec<PathBuf> = selected
                        .iter()
                        .map(|entry_path| mount_path.join(entry_path))
                        .collect();

                    tracing::info!(
                        "DnD via FUSE: providing {} virtual path(s) — ~0ms latency",
                        drag_paths.len()
                    );
                    return drag_paths;
                }

                tracing::warn!("FUSE unavailable — extracting to tmpfs");
                drop(mgr);

                let mgr_for_extract = manager_drag.lock().unwrap_or_else(|e| e.into_inner());
                temp_cache_drag.clear_stage();
                let stage_folder = temp_cache_drag.stage_dir();

                match mgr_for_extract.extract_selected(&selected, &stage_folder) {
                    Ok(_) => {
                        let top_level = get_top_level_drag_paths(&stage_folder);
                        tracing::info!("Fallback: {} path(s) ready in tmpfs", top_level.len());
                        top_level
                    }
                    Err(e) => {
                        tracing::error!("Fallback extraction failed: {}", e);
                        Vec::new()
                    }
                }
            },
            move |success| {
                if success {
                    show_toast_end("Successfully transferred to File Manager!");
                } else {
                    tracing::warn!("Drag-out: cancelled or rejected by file manager");
                }
            },
        );

        let open_btn_do = do_open_rc.clone();
        let fv_root = file_view.clone();

        open_btn.connect_clicked(move |_| {
            if let Some(root_window) = fv_root.widget().root() {
                let window = root_window.downcast_ref::<gtk4::Window>().unwrap();
                let dialog = gtk4::FileDialog::builder()
                    .title("Open Archive")
                    .build();

                let do_open_c = open_btn_do.clone();
                dialog.open(Some(window), gio::Cancellable::NONE, move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            do_open_c(path);
                        }
                    }
                });
            }
        });

        // ─────────────────────────────────────────────────────────────────────
        // Клавиатурные сочетания
        // ─────────────────────────────────────────────────────────────────────
        let key_controller = EventControllerKey::new();
        let file_view_key = file_view.clone();
        let up_btn_key = file_view.up_button().clone();
        let fw_btn_key = file_view.forward_button().clone();
        let open_btn_key = open_btn.clone();
        let list_box_key = file_view.list_box().clone();
        let manager_key = manager.clone();

        key_controller.connect_key_pressed(move |_, key, _keycode, state| {
            let is_ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
            let is_alt = state.contains(gdk::ModifierType::ALT_MASK);

            match key {
                // Backspace или Alt+Left → Вверх по дереву
                gdk::Key::BackSpace | gdk::Key::Back => {
                    if up_btn_key.is_sensitive() {
                        up_btn_key.emit_clicked();
                        return glib::Propagation::Stop;
                    }
                }
                gdk::Key::Left if is_alt => {
                    if up_btn_key.is_sensitive() {
                        up_btn_key.emit_clicked();
                        return glib::Propagation::Stop;
                    }
                }
                gdk::Key::Right if is_alt => {
                    if fw_btn_key.is_sensitive() {
                        fw_btn_key.emit_clicked();
                        return glib::Propagation::Stop;
                    }
                }
                // Ctrl+O → Открыть архив
                gdk::Key::o | gdk::Key::O if is_ctrl => {
                    open_btn_key.emit_clicked();
                    return glib::Propagation::Stop;
                }
                // Ctrl+A → Выбрать всё
                gdk::Key::a | gdk::Key::A if is_ctrl => {
                    file_view_key.select_all();
                    return glib::Propagation::Stop;
                }
                // Escape → Снять выделение
                gdk::Key::Escape => {
                    file_view_key.deselect_all();
                    return glib::Propagation::Stop;
                }
                // Enter → Enter directory, Space → Quick Preview (QuickLook style)
                gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::space => {
                    let selected_rows = list_box_key.selected_rows();
                    if selected_rows.len() == 1 {
                        let idx = selected_rows[0].index() as usize;
                        if let Some(entry) = file_view_key.get_entry_at_index(idx) {
                            if entry.is_dir && key != gdk::Key::space {
                                let mut mgr =
                                    manager_key.lock().unwrap_or_else(|e| e.into_inner());
                                mgr.set_current_folder(entry.path.clone());
                                let visible = mgr.visible_entries();
                                let folder = mgr.current_folder().to_path_buf();
                                file_view_key.set_entries(visible);
                                file_view_key.update_folder_path(&folder);
                                return glib::Propagation::Stop;
                            } else if !entry.is_dir && key == gdk::Key::space {
                                let _ = file_view_key.widget().activate_action("win.quick_preview", None);
                                return glib::Propagation::Stop;
                            }
                        }
                    }
                }
                // Delete → Удалить выделенные элементы
                gdk::Key::Delete | gdk::Key::KP_Delete => {
                    let _ = file_view_key.widget().activate_action("win.delete_selected", None);
                    return glib::Propagation::Stop;
                }
                _ => {}
            }

            glib::Propagation::Proceed
        });

        window.add_controller(key_controller);

        Self {
            window,
            _manager: manager,
            _temp_cache: temp_cache,
            open_path_fn: do_open_rc,
        }
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn open_path(&self, path: PathBuf) {
        (self.open_path_fn)(path);
    }
}

/// Возвращает пути верхнего уровня внутри drag_folder (для fallback-режима без FUSE)
fn get_top_level_drag_paths(drag_folder: &Path) -> Vec<PathBuf> {
    let mut top_level = Vec::new();
    if let Ok(entries) = std::fs::read_dir(drag_folder) {
        for entry in entries.filter_map(|e| e.ok()) {
            top_level.push(entry.path());
        }
    }
    top_level
}
