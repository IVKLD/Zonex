use gdk4::{DragAction, FileList};
use glib::{idle_add_local, ControlFlow};
use gtk4::prelude::*;
use gtk4::DropTarget;
use std::path::PathBuf;
use std::rc::Rc;

pub fn setup_drop_target<F>(widget: &impl IsA<gtk4::Widget>, on_drop: F)
where
    F: Fn(Vec<PathBuf>) + 'static,
{
    let target = DropTarget::new(FileList::static_type(), DragAction::COPY);
    let on_drop_rc = Rc::new(on_drop);

    target.connect_drop(move |_, value, _, _| {
        let Ok(file_list) = value.get::<FileList>() else {
            return false;
        };

        let file_paths: Vec<PathBuf> = file_list
            .files()
            .into_iter()
            .filter_map(|f| f.path())
            .collect();

        if file_paths.is_empty() {
            return false;
        }

        let mut file_paths_opt = Some(file_paths);
        let cb = on_drop_rc.clone();
        idle_add_local(move || {
            if let Some(paths) = file_paths_opt.take() {
                cb(paths);
            }
            ControlFlow::Break
        });
        true
    });

    widget.add_controller(target);
}
