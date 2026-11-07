use super::context::ActionsContext;
use crate::archive::ArchiveEntry;
use crate::ui::dialogs;
use crate::ui::file_view::FileView;
use crate::ui::preview;
use gio::{AppInfo, AppLaunchContext, SimpleAction};
use gtk4::prelude::*;

fn get_single_selected_entry(file_view: &FileView) -> Option<ArchiveEntry> {
    let selected_rows = file_view.list_box().selected_rows();
    if selected_rows.len() == 1 {
        let idx = selected_rows[0].index() as usize;
        file_view.get_entry_at_index(idx)
    } else {
        None
    }
}

pub fn register(ctx: &ActionsContext) {
    let manager_open = ctx.manager.clone();
    let file_view_open = ctx.file_view.clone();
    let temp_cache_open = ctx.temp_cache.clone();
    let show_toast_open = ctx.show_toast.clone();

    let action_open_entry = SimpleAction::new("open_entry", None);
    action_open_entry.connect_activate(move |_, _| {
        let Some(entry) = get_single_selected_entry(&file_view_open) else {
            return;
        };

        if entry.is_dir {
            let mut mgr = manager_open.lock().unwrap_or_else(|e| e.into_inner());
            mgr.enter_folder(entry.path);
            let visible = mgr.visible_entries();
            let folder = mgr.current_folder().to_path_buf();
            drop(mgr);
            file_view_open.set_entries(visible);
            file_view_open.update_folder_path(&folder);
            file_view_open.set_forward_sensitive(false);
            return;
        }

        // 1. Try zero-latency FUSE virtual path first
        {
            let mgr = manager_open.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(mount) = mgr.fuse_mount_path() {
                let virtual_path = mount.join(&entry.path);
                if virtual_path.exists() {
                    let gio_file = gio::File::for_path(&virtual_path);
                    match AppInfo::launch_default_for_uri(&gio_file.uri(), AppLaunchContext::NONE) {
                        Ok(_) => {
                            tracing::info!(
                                "Opened via FUSE zero-latency: {}",
                                virtual_path.display()
                            );
                            return;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to open via FUSE URI, falling back to cache: {e}"
                            );
                        }
                    }
                }
            }
        }

        // 2. Extract asynchronously in background thread to open cache
        let open_cache_dir = temp_cache_open.open_cache_dir();
        let mgr = manager_open.clone();
        let toast = show_toast_open.clone();
        let entry_path = entry.path.clone();

        glib::MainContext::default().spawn_local(async move {
            let res = gio::spawn_blocking(move || {
                let mgr = mgr.lock().unwrap_or_else(|e| e.into_inner());
                match mgr.extract_selected(&[entry_path], &open_cache_dir) {
                    Ok(extracted) => {
                        if let Some(first) = extracted.first() {
                            Ok(first.clone())
                        } else {
                            Err("No file extracted".to_string())
                        }
                    }
                    Err(e) => Err(e.to_string()),
                }
            })
            .await
            .unwrap_or_else(|_| Err("Extraction panicked".to_string()));

            match res {
                Ok(path) => {
                    let gio_file = gio::File::for_path(&path);
                    if let Err(e) =
                        AppInfo::launch_default_for_uri(&gio_file.uri(), AppLaunchContext::NONE)
                    {
                        toast(&format!("Failed to launch default app: {e}"));
                    }
                }
                Err(e) => {
                    toast(&format!("Failed to open file: {e}"));
                }
            }
        });
    });
    ctx.window.add_action(&action_open_entry);

    let file_view_props = ctx.file_view.clone();
    let window_props = ctx.window.clone();
    let show_toast_props = ctx.show_toast.clone();

    let action_entry_properties = SimpleAction::new("entry_properties", None);
    action_entry_properties.connect_activate(move |_, _| {
        let selected_rows = file_view_props.list_box().selected_rows();
        match selected_rows.as_slice() {
            [row] => {
                let idx = row.index() as usize;
                if let Some(entry) = file_view_props.get_entry_at_index(idx) {
                    dialogs::show_entry_properties(&window_props, &entry);
                }
            }
            [] => show_toast_props("Select an item to view properties"),
            multiple => show_toast_props(&format!("{} items selected", multiple.len())),
        }
    });
    ctx.window.add_action(&action_entry_properties);

    let manager_prev = ctx.manager.clone();
    let file_view_prev = ctx.file_view.clone();
    let temp_cache_prev = ctx.temp_cache.clone();
    let window_prev = ctx.window.clone();
    let show_toast_prev = ctx.show_toast.clone();

    let action_quick_preview = SimpleAction::new("quick_preview", None);
    action_quick_preview.connect_activate(move |_, _| {
        let Some(entry) = get_single_selected_entry(&file_view_prev) else {
            return;
        };

        if !entry.is_dir {
            let fuse_path = {
                let mgr = manager_prev.lock().unwrap_or_else(|e| e.into_inner());
                mgr.fuse_mount_path().map(|m| m.join(&entry.path))
            };

            if let Some(p) = fuse_path.filter(|p| p.exists()) {
                preview::show(&window_prev, &p);
                return;
            }

            let open_cache_dir = temp_cache_prev.open_cache_dir();
            let mgr = manager_prev.clone();
            let win = window_prev.clone();
            let toast = show_toast_prev.clone();
            let entry_path = entry.path.clone();

            glib::MainContext::default().spawn_local(async move {
                let res = gio::spawn_blocking(move || {
                    let mgr = mgr.lock().unwrap_or_else(|e| e.into_inner());
                    match mgr.extract_selected(&[entry_path], &open_cache_dir) {
                        Ok(extracted) => {
                            if let Some(first) = extracted.first() {
                                Ok(first.clone())
                            } else {
                                Err("No file extracted".to_string())
                            }
                        }
                        Err(e) => Err(e.to_string()),
                    }
                })
                .await
                .unwrap_or_else(|_| Err("Preview extraction panicked".to_string()));

                match res {
                    Ok(file_path) => {
                        preview::show(&win, &file_path);
                    }
                    Err(e) => {
                        toast(&format!("Failed to extract file for preview: {e}"));
                    }
                }
            });
        }
    });
    ctx.window.add_action(&action_quick_preview);
}
