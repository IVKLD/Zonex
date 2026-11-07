use gdk4::{ContentProvider, DragAction, FileList};
use gio::File as GioFile;
use gtk4::prelude::*;
use gtk4::DragSource;
use std::path::PathBuf;
use std::rc::Rc;

pub fn setup_drag_source<F, C>(
    widget: &impl IsA<gtk4::Widget>,
    prepare_drag_files: F,
    on_drag_end: C,
) where
    F: Fn(f64, f64) -> Vec<PathBuf> + 'static,
    C: Fn(bool) + 'static,
{
    let drag_source = DragSource::new();
    drag_source.set_actions(DragAction::COPY);

    let prepare_cb = Rc::new(prepare_drag_files);
    drag_source.connect_prepare(move |_, x, y| {
        let paths = prepare_cb(x, y);
        if paths.is_empty() {
            tracing::warn!(
                "DragSource prepare: no files provided or drag started outside selected item"
            );
            return None;
        }

        tracing::info!(
            "DragSource: providing {} file(s) to Wayland session",
            paths.len()
        );

        let gfiles: Vec<GioFile> = paths.into_iter().map(GioFile::for_path).collect();
        let file_list = FileList::from_array(&gfiles);
        Some(ContentProvider::for_value(&file_list.to_value()))
    });

    let end_cb = Rc::new(on_drag_end);
    drag_source.connect_drag_end(move |_, _, success| {
        tracing::info!("DragSource: session ended. success={}", success);
        end_cb(success);
    });

    widget.add_controller(drag_source);
}
