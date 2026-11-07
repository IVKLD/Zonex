use crate::archive::ArchiveManager;
use crate::ui::file_view::FileView;
use crate::ui::window::actions::OpenCallback;
use gio::Cancellable;
use gtk4::prelude::*;
use gtk4::{Button, FileDialog, Window};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn sync_file_view(file_view: &FileView, mgr: &ArchiveManager) {
    file_view.set_entries(mgr.visible_entries());
    file_view.update_folder_path(mgr.current_folder());
}

pub fn setup_navigation(file_view: &Rc<FileView>, manager: &Arc<Mutex<ArchiveManager>>) {
    let manager_nav = manager.clone();
    let file_view_nav = file_view.clone();
    file_view.list_box().connect_row_activated(move |_, row| {
        let idx = row.index() as usize;
        if let Some(entry) = file_view_nav.get_entry_at_index(idx) {
            if entry.is_dir {
                let mut mgr = manager_nav.lock().unwrap_or_else(|e| e.into_inner());
                mgr.enter_folder(entry.path);
                sync_file_view(&file_view_nav, &mgr);
                file_view_nav.set_forward_sensitive(false);
            } else {
                let _ = file_view_nav
                    .widget()
                    .activate_action("win.open_entry", None);
            }
        }
    });

    let manager_up = manager.clone();
    let file_view_up = file_view.clone();
    file_view.up_button().connect_clicked(move |_| {
        let mut mgr = manager_up.lock().unwrap_or_else(|e| e.into_inner());
        if mgr.navigate_back() {
            sync_file_view(&file_view_up, &mgr);
            file_view_up.set_forward_sensitive(true);
        }
    });

    let manager_fw = manager.clone();
    let file_view_fw = file_view.clone();
    file_view.forward_button().connect_clicked(move |_| {
        let mut mgr = manager_fw.lock().unwrap_or_else(|e| e.into_inner());
        if mgr.navigate_forward() {
            sync_file_view(&file_view_fw, &mgr);
        }
    });
}

pub fn setup_open_dialog(open_btn: &Button, file_view: &Rc<FileView>, do_open: OpenCallback) {
    let fv_root = file_view.clone();

    open_btn.connect_clicked(move |_| {
        if let Some(root_window) = fv_root.widget().root() {
            let window = root_window.downcast_ref::<Window>().unwrap();
            let dialog = FileDialog::builder().title("Open Archive").build();

            let do_open_c = do_open.clone();
            dialog.open(Some(window), Cancellable::NONE, move |res| {
                if let Some(path) = res.ok().and_then(|f| f.path()) {
                    do_open_c(path);
                }
            });
        }
    });
}
