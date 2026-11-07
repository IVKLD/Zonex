use super::context::{ActionsContext, LoadingCallback, ToastCallback};
use crate::archive::ArchiveManager;
use gio::{Cancellable, SimpleAction};
use gtk4::prelude::*;
use gtk4::FileDialog;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn run_background_extraction(
    mgr: Arc<Mutex<ArchiveManager>>,
    selected_paths: Vec<PathBuf>,
    dest_dir: PathBuf,
    sl: LoadingCallback,
    toast: ToastCallback,
) {
    sl(true, "Extracting selected files...");
    let dest_task = dest_dir.clone();

    glib::MainContext::default().spawn_local(async move {
        let res = gio::spawn_blocking(move || {
            let mgr = mgr.lock().unwrap_or_else(|e| e.into_inner());
            mgr.extract_selected(&selected_paths, &dest_task)
                .map(|e| e.len())
                .map_err(|e| e.to_string())
        })
        .await
        .unwrap_or_else(|_| Err("Extraction task panicked".to_string()));

        sl(false, "Extraction finished");
        match res {
            Ok(n) => {
                toast(&format!(
                    "Extracted {} item(s) to {}",
                    n,
                    dest_dir.display()
                ));
            }
            Err(e) => {
                toast(&format!("Failed to extract: {}", e));
            }
        }
    });
}

fn run_single_file_export(
    mgr: Arc<Mutex<ArchiveManager>>,
    source_entry: PathBuf,
    target_file: PathBuf,
    sl: LoadingCallback,
    toast: ToastCallback,
) {
    sl(true, "Exporting file...");
    let target_task = target_file.clone();

    glib::MainContext::default().spawn_local(async move {
        let res = gio::spawn_blocking(move || -> Result<PathBuf, String> {
            let temp_dir = tempfile::tempdir()
                .map_err(|e| format!("Failed to create temporary folder: {e}"))?;

            let mgr = mgr.lock().unwrap_or_else(|e| e.into_inner());
            let extracted = mgr
                .extract_selected(&[source_entry], temp_dir.path())
                .map_err(|e| e.to_string())?;

            if let Some(src) = extracted.first() {
                if let Some(parent) = target_task.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::copy(src, &target_task)
                    .map_err(|e| format!("Failed to save exported file: {e}"))?;
                Ok(target_task)
            } else {
                Err("No file extracted".to_string())
            }
        })
        .await
        .unwrap_or_else(|_| Err("Export task panicked".to_string()));

        sl(false, "Export complete");
        match res {
            Ok(path) => {
                toast(&format!("Exported to {}", path.display()));
            }
            Err(e) => {
                toast(&format!("Failed to export: {}", e));
            }
        }
    });
}

pub fn register(ctx: &ActionsContext) {
    let manager = ctx.manager.clone();
    let file_view = ctx.file_view.clone();
    let show_toast = ctx.show_toast.clone();
    let set_loading = ctx.set_loading.clone();
    let window = ctx.window.clone();

    let action_extract_selected = SimpleAction::new("extract_selected", None);
    action_extract_selected.connect_activate(move |_, _| {
        let selected_paths = file_view.get_selected_entry_paths();
        if selected_paths.is_empty() {
            show_toast("No files selected to extract");
            return;
        }

        let dialog = FileDialog::builder()
            .title("Select Destination Folder")
            .build();

        let mgr = manager.clone();
        let sl = set_loading.clone();
        let toast = show_toast.clone();

        dialog.select_folder(Some(&window), Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                if let Some(dest_dir) = file.path() {
                    run_background_extraction(mgr, selected_paths, dest_dir, sl, toast);
                }
            }
        });
    });
    ctx.window.add_action(&action_extract_selected);

    let manager_here = ctx.manager.clone();
    let file_view_here = ctx.file_view.clone();
    let show_toast_here = ctx.show_toast.clone();
    let set_loading_here = ctx.set_loading.clone();

    let action_extract_here = SimpleAction::new("extract_here", None);
    action_extract_here.connect_activate(move |_, _| {
        let selected_paths = file_view_here.get_selected_entry_paths();
        if selected_paths.is_empty() {
            show_toast_here("No files selected to extract");
            return;
        }

        let mgr = manager_here.lock().unwrap_or_else(|e| e.into_inner());
        let dest_dir = mgr
            .current_path()
            .and_then(|p| p.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        drop(mgr);

        run_background_extraction(
            manager_here.clone(),
            selected_paths,
            dest_dir,
            set_loading_here.clone(),
            show_toast_here.clone(),
        );
    });
    ctx.window.add_action(&action_extract_here);

    let manager_export = ctx.manager.clone();
    let file_view_export = ctx.file_view.clone();
    let show_toast_export = ctx.show_toast.clone();
    let set_loading_export = ctx.set_loading.clone();
    let window_export = ctx.window.clone();

    let action_export_selected = SimpleAction::new("export_selected", None);
    action_export_selected.connect_activate(move |_, _| {
        let selected_paths = file_view_export.get_selected_entry_paths();
        if selected_paths.is_empty() {
            show_toast_export("No files selected to export");
            return;
        }

        let mgr = manager_export.clone();
        let sl = set_loading_export.clone();
        let toast = show_toast_export.clone();

        if selected_paths.len() == 1 {
            let single_path = selected_paths[0].clone();
            let initial_name = single_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "export".to_string());

            let dialog = FileDialog::builder()
                .title("Export File As...")
                .initial_name(&initial_name)
                .build();

            dialog.save(Some(&window_export), Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(target_path) = file.path() {
                        run_single_file_export(mgr, single_path, target_path, sl, toast);
                    }
                }
            });
        } else {
            let dialog = FileDialog::builder()
                .title("Select Destination Folder for Export")
                .build();

            dialog.select_folder(Some(&window_export), Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(dest_dir) = file.path() {
                        run_background_extraction(mgr, selected_paths, dest_dir, sl, toast);
                    }
                }
            });
        }
    });
    ctx.window.add_action(&action_export_selected);
}
