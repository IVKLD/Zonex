use crate::archive::ArchiveManager;
use crate::dnd::setup_drag_source;
use crate::ui::file_view::FileView;
use crate::ui::window::actions::ToastCallback;
use crate::utils::TempCache;
use gtk4::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn get_top_level_drag_paths(drag_folder: &Path) -> Vec<PathBuf> {
    fs::read_dir(drag_folder)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect()
}

fn resolve_drag_paths(
    file_view: &FileView,
    manager: &Arc<Mutex<ArchiveManager>>,
    temp_cache: &TempCache,
    y: f64,
) -> Vec<PathBuf> {
    let row = match file_view.list_box().row_at_y(y as i32) {
        Some(r) => r,
        None => return Vec::new(),
    };

    if !row.is_selected() {
        file_view.list_box().unselect_all();
        file_view.list_box().select_row(Some(&row));
    }

    let selected = file_view.get_selected_entry_paths();
    if selected.is_empty() {
        return Vec::new();
    }

    let mgr = manager.lock().unwrap_or_else(|e| e.into_inner());

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
    temp_cache.clear_stage();
    let stage_folder = temp_cache.stage_dir();

    match mgr.extract_selected(&selected, &stage_folder) {
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
}

pub fn setup_window_drag(
    file_view: Rc<FileView>,
    manager: Arc<Mutex<ArchiveManager>>,
    temp_cache: Rc<TempCache>,
    show_toast: ToastCallback,
) {
    let fv = file_view.clone();
    let mgr = manager.clone();
    let cache = temp_cache.clone();

    setup_drag_source(
        file_view.list_box(),
        move |_, y| resolve_drag_paths(&fv, &mgr, &cache, y),
        move |success| {
            if success {
                show_toast("Successfully transferred to File Manager!");
            } else {
                tracing::warn!("Drag-out: cancelled or rejected by file manager");
            }
        },
    );
}
