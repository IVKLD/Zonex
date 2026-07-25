use super::file_view::FileView;
use super::status_bar::StatusBar;
use crate::archive::ArchiveManager;
use crate::dnd::{setup_drag_source, setup_drop_target};
use crate::utils::TempCache;
use gtk4::gio;
use gtk4::prelude::*;
#[allow(deprecated)]
use gtk4::{
    gdk, Box as GtkBox, Button, EventControllerKey, FileChooserAction, FileChooserNative,
    Orientation, ResponseType, Spinner,
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
        }

        let toast_overlay = adw::ToastOverlay::new();

        let header_bar = adw::HeaderBar::new();
        let open_btn = Button::with_label("Open Archive");
        let spinner = Spinner::new();
        spinner.set_visible(false);

        header_bar.pack_start(&open_btn);
        header_bar.pack_end(&spinner);

        let main_box = GtkBox::new(Orientation::Vertical, 0);
        main_box.append(&header_bar);

        let file_view = Rc::new(FileView::new());
        main_box.append(file_view.widget());

        let status_bar = Rc::new(StatusBar::new());
        main_box.append(status_bar.widget());

        toast_overlay.set_child(Some(&main_box));
        window.set_content(Some(&toast_overlay));

        // Вспомогательная функция отображения Toast
        let toast_overlay_c = toast_overlay.clone();
        let show_toast = move |msg: &str| {
            let toast = adw::Toast::new(msg);
            toast.set_timeout(3);
            toast_overlay_c.add_toast(toast);
        };
        let show_toast_rc = Rc::new(show_toast);

        // Действия окна для контекстного меню
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

        // Навигация по двойному клику на папку
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

        // ─────────────────────────────────────────────────────────────────────
        // DRAG IN (Файловый менеджер → Zonex): Файлы читаются в фоновом потоке
        // ─────────────────────────────────────────────────────────────────────
        let manager_drop = manager.clone();
        let file_view_drop = file_view.clone();
        let set_loading_drop = set_loading_rc.clone();
        let show_toast_drop = show_toast_rc.clone();

        setup_drop_target(&window, move |paths| {
            if paths.is_empty() {
                return;
            }

            let first_path = paths[0].clone();
            let is_single = paths.len() == 1;
            let is_archive = ArchiveManager::get_engine_for_path(&first_path).is_ok();

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
                    // Проверяем, не брошены ли файлы из этого же архива (через FUSE) или сам файл архива
                    let is_self_drop = if let Some(mount_path) = mgr.fuse_mount_path() {
                        paths.iter().any(|p| p.starts_with(mount_path))
                    } else {
                        false
                    } || paths.iter().any(|p| Some(p.as_path()) == mgr.current_path());

                    if is_self_drop {
                        visible_entries = mgr.visible_entries();
                        current_folder = mgr.current_folder().to_path_buf();
                        result_msg = "Selected items are already in this archive".to_string();
                        success = false;
                    } else {
                        // Добавляем файлы в открытый архив
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
                    }
                } else if is_single && is_archive {
                    // Открываем брошенный архив (FUSE монтируется внутри open_archive)
                    match mgr.open_archive(&first_path) {
                        Ok(entries) => {
                            visible_entries = entries;
                            current_folder = mgr.current_folder().to_path_buf();
                            result_msg = format!(
                                "Opened: {}",
                                first_path.file_name().unwrap_or_default().to_string_lossy()
                            );
                            success = true;
                        }
                        Err(err) => {
                            tracing::error!("Failed to open archive {:?}: {:?}", first_path, err);
                            result_msg = format!("Failed to open archive: {}", err);
                        }
                    }
                } else {
                    // Создаём новый архив из брошенных файлов
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

            // Неблокирующая проверка результата ~60 FPS (0% CPU в idle)
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
        //
        // Принцип: connect_prepare выполняется на ГЛАВНОМ GTK-потоке.
        // Мы НЕ извлекаем файлы здесь. Вместо этого:
        //   1. Берём FUSE-путь (смонтирован при open_archive — мгновенно)
        //   2. Возвращаем виртуальный путь файловому менеджеру
        //   3. FM читает файл → FUSE-поток извлекает байты из архива на лету
        //   4. UI не зависает ни на миллисекунду
        //
        // Fallback: если libfuse3 недоступен — извлечение в tmpfs в connect_prepare
        // с предупреждением. Но это не блокирует UI: мы используем уже открытый
        // архив и кэш (для маленьких файлов почти мгновенно).
        // ─────────────────────────────────────────────────────────────────────
        let manager_drag = manager.clone();
        let temp_cache_drag = temp_cache.clone();
        let file_view_drag = file_view.clone();
        let show_toast_end = show_toast_rc.clone();

        setup_drag_source(
            file_view.list_box(),
            // prepare_drag_files: вызывается мгновенно в connect_prepare.
            // При FUSE возвращает готовые виртуальные пути за ~0ms (только конкатенация строк).
            move |_, y| {
                // Проверяем, что зажата строка (элемент), а не пустое место
                let row = match file_view_drag.list_box().row_at_y(y as i32) {
                    Some(r) => r,
                    None => {
                        tracing::warn!("Drag-out ignored: drag started outside list items");
                        return Vec::new();
                    }
                };

                if !row.is_selected() {
                    tracing::warn!("Drag-out ignored: dragged row is not selected");
                    return Vec::new();
                }

                let selected = file_view_drag.get_selected_entry_paths();
                if selected.is_empty() {
                    tracing::warn!("No entries selected for Drag-out");
                    return Vec::new();
                }

                let mgr = manager_drag.lock().unwrap_or_else(|e| e.into_inner());

                // ── Путь 1: FUSE доступен — возвращаем виртуальные пути мгновенно ──
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

                // ── Путь 2: FUSE недоступен — fallback на tmpfs ──
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
            // on_drag_end
            move |success| {
                if success {
                    show_toast_end("Successfully transferred to File Manager!");
                } else {
                    tracing::warn!("Drag-out: cancelled or rejected by file manager");
                }
            },
        );



        // ─────────────────────────────────────────────────────────────────────
        // Процедура открытия архива по пути (вызывается из CLI/Nautilus/FileChooser)
        // ─────────────────────────────────────────────────────────────────────
        let manager_open = manager.clone();
        let file_view_open = file_view.clone();
        let set_loading_open = set_loading_rc.clone();
        let show_toast_open = show_toast_rc.clone();
        let temp_cache_open = temp_cache.clone();

        let do_open_path = move |path: PathBuf| {
            set_loading_open(true, "Opening archive (mounting FUSE...)");
            temp_cache_open.clear_stage();

            let mgr_task = manager_open.clone();
            let fv_task = file_view_open.clone();
            let sl_task = set_loading_open.clone();
            let toast_task = show_toast_open.clone();

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

                let mut mgr = mgr_task.lock().unwrap_or_else(|e| e.into_inner());
                match mgr.open_archive(&path) {
                    Ok(entries) => {
                        entries_res = entries;
                        msg = format!(
                            "Opened: {}",
                            path.file_name()
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
                    let status_suffix = if has_fuse { " [FUSE]" } else { " [tmpfs]" };
                    sl_task(false, &format!("{}{}", msg, status_suffix));
                    if success {
                        fv_task.set_entries(entries);
                        fv_task.update_folder_path(Path::new(""));
                        let dnd_info = if has_fuse {
                            "⚡ DnD via FUSE (zero-latency)"
                        } else {
                            "⚠ DnD via tmpfs (install libfuse3 for streaming)"
                        };
                        toast_task(&format!("{} ({:.2}s) — {}", msg, duration, dnd_info));
                    } else {
                        toast_task(&msg);
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        };

        let do_open_rc: Rc<dyn Fn(PathBuf)> = Rc::new(do_open_path);
        let open_btn_do = do_open_rc.clone();
        let fv_root = file_view.clone();

        #[allow(deprecated)]
        open_btn.connect_clicked(move |_| {
            if let Some(root_window) = fv_root.widget().root() {
                let window = root_window.downcast_ref::<gtk4::Window>().unwrap();
                let dialog = FileChooserNative::new(
                    Some("Open Archive"),
                    Some(window),
                    FileChooserAction::Open,
                    Some("Open"),
                    Some("Cancel"),
                );

                let do_open_c = open_btn_do.clone();
                dialog.connect_response(move |d, response| {
                    if response == ResponseType::Accept {
                        if let Some(file) = d.file() {
                            if let Some(path) = file.path() {
                                do_open_c(path);
                            }
                        }
                    }
                    d.destroy();
                });

                dialog.show();
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
                // Enter/Space → Войти в директорию
                gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::space => {
                    let selected_rows = list_box_key.selected_rows();
                    if selected_rows.len() == 1 {
                        let idx = selected_rows[0].index() as usize;
                        if let Some(entry) = file_view_key.get_entry_at_index(idx) {
                            if entry.is_dir {
                                let mut mgr =
                                    manager_key.lock().unwrap_or_else(|e| e.into_inner());
                                mgr.set_current_folder(entry.path.clone());
                                let visible = mgr.visible_entries();
                                let folder = mgr.current_folder().to_path_buf();
                                file_view_key.set_entries(visible);
                                file_view_key.update_folder_path(&folder);
                                return glib::Propagation::Stop;
                            }
                        }
                    }
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
