use gdk4::FileList;
use gtk4::prelude::*;
use gtk4::{gdk, DropTarget};
use std::path::PathBuf;
use std::rc::Rc;

/// Configures Wayland DropTarget on a widget to accept files from external File Managers
pub fn setup_drop_target<F>(widget: &impl IsA<gtk4::Widget>, on_drop: F)
where
    F: Fn(Vec<PathBuf>) + 'static,
{
    let target = DropTarget::new(FileList::static_type(), gdk::DragAction::COPY);
    let on_drop_rc = Rc::new(on_drop);

    target.connect_drop(move |_, value, _, _| {
        let mut file_paths = Vec::new();

        if let Ok(file_list) = value.get::<FileList>() {
            for file in file_list.files() {
                if let Some(path) = file.path() {
                    file_paths.push(path);
                }
            }
        }

        if !file_paths.is_empty() {
            let cb = on_drop_rc.clone();
            glib::idle_add_local(move || {
                cb(file_paths.clone());
                glib::ControlFlow::Break
            });
            true
        } else {
            false
        }
    });

    widget.add_controller(target);
}
