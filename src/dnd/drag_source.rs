use gdk4::FileList;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::DragSource;
use std::path::PathBuf;
use std::rc::Rc;

/// Configures Wayland DragSource with zero-latency payload preparation.
///
/// In GTK4 `GtkDragSource`:
/// `connect_prepare` is emitted FIRST when the drag starts.
/// It MUST return a `GdkContentProvider` synchronously without blocking the UI thread.
///
/// When FUSE is active:
/// `prepare_drag_files` returns the FUSE virtual paths (`/run/user/.../zonex_fuse/...`)
/// in ~0ms (pure string path joining).
pub fn setup_drag_source<F, C>(
    widget: &impl IsA<gtk4::Widget>,
    prepare_drag_files: F,
    on_drag_end: C,
) where
    F: Fn(f64, f64) -> Vec<PathBuf> + 'static,
    C: Fn(bool) + 'static,
{
    let drag_source = DragSource::new();
    drag_source.set_actions(gdk::DragAction::COPY);

    let prepare_cb = Rc::new(prepare_drag_files);
    drag_source.connect_prepare(move |_, x, y| {
        let paths = prepare_cb(x, y);
        if paths.is_empty() {
            tracing::warn!("DragSource prepare: no files provided or drag started outside selected item");
            return None;
        }

        tracing::info!("DragSource: providing {} file(s) to Wayland session", paths.len());

        let gfiles: Vec<gio::File> = paths
            .into_iter()
            .map(|path| gio::File::for_path(path))
            .collect();

        let file_list = FileList::from_array(&gfiles);
        Some(gdk::ContentProvider::for_value(&file_list.to_value()))
    });

    let end_cb = Rc::new(on_drag_end);
    drag_source.connect_drag_end(move |_, _, success| {
        tracing::info!("DragSource: session ended. success={}", success);
        end_cb(success);
    });

    widget.add_controller(drag_source);
}
