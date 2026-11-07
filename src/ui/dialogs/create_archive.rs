use gio::{Cancellable, ListStore};
use gtk4::prelude::*;
use gtk4::{FileDialog, FileFilter};
use libadwaita::ApplicationWindow;
use std::cell::RefCell;
use std::path::PathBuf;

pub fn show_create_archive_dialog<F: FnOnce(PathBuf, String) + 'static>(
    parent: &ApplicationWindow,
    source_paths: &[PathBuf],
    callback: F,
) {
    let default_name = source_paths
        .first()
        .and_then(|p| p.file_name())
        .map_or_else(
            || "archive.zip".to_string(),
            |n| format!("{}.zip", n.to_string_lossy()),
        );

    let zip_filter = FileFilter::new();
    zip_filter.set_name(Some("Zip Archive (*.zip)"));
    zip_filter.add_pattern("*.zip");

    let tar_gz_filter = FileFilter::new();
    tar_gz_filter.set_name(Some("Tar Gzip Archive (*.tar.gz)"));
    tar_gz_filter.add_pattern("*.tar.gz");

    let tar_zst_filter = FileFilter::new();
    tar_zst_filter.set_name(Some("Tar Zstd Archive (*.tar.zst)"));
    tar_zst_filter.add_pattern("*.tar.zst");

    let filters = ListStore::new::<FileFilter>();
    filters.append(&zip_filter);
    filters.append(&tar_gz_filter);
    filters.append(&tar_zst_filter);

    let dialog = FileDialog::builder()
        .title("Create Archive")
        .initial_name(&default_name)
        .filters(&filters)
        .build();

    let cb = RefCell::new(Some(callback));
    dialog.save(Some(parent), Cancellable::NONE, move |res| {
        if let Some(path) = res.ok().and_then(|f| f.path()) {
            if let Some(cb_fn) = cb.borrow_mut().take() {
                cb_fn(path, "zip".to_string());
            }
        }
    });
}
