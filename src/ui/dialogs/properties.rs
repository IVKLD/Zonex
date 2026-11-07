use crate::archive::ArchiveEntry;
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow, MessageDialog};

pub fn show(parent: &ApplicationWindow, entry: &ArchiveEntry) {
    let title = format!("Properties: {}", entry.file_name());
    let body = format!(
        "Name: {}\nRelative Path: {}\nType: {}\nSize: {}",
        entry.file_name(),
        entry.path.display(),
        if entry.is_dir { "Directory" } else { "File" },
        entry.formatted_size()
    );

    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .heading(&title)
        .body(&body)
        .build();

    dialog.add_response("close", "Close");
    dialog.connect_response(None, move |d, _| {
        d.close();
    });
    dialog.present();
}
