use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::path::PathBuf;

pub struct Dialogs;

impl Dialogs {
    /// Shows a password prompt dialog using Libadwaita
    pub fn prompt_password<F: FnOnce(Option<String>) + 'static>(
        parent: &adw::ApplicationWindow,
        title_text: &str,
        callback: F,
    ) {
        let dialog = adw::MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .heading("Password Required")
            .body(title_text)
            .build();

        let entry = gtk4::PasswordEntry::builder()
            .placeholder_text("Enter archive password")
            .show_peek_icon(true)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(16)
            .margin_end(16)
            .build();

        dialog.set_extra_child(Some(&entry));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("ok", "Unlock");
        dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);

        let cb = std::cell::RefCell::new(Some(callback));

        entry.connect_activate({
            let dialog = dialog.clone();
            move |_| {
                dialog.response("ok");
            }
        });

        dialog.connect_response(None, move |_, response| {
            if let Some(cb_fn) = cb.borrow_mut().take() {
                if response == "ok" {
                    let pwd = entry.text().to_string();
                    if pwd.is_empty() {
                        cb_fn(None);
                    } else {
                        cb_fn(Some(pwd));
                    }
                } else {
                    cb_fn(None);
                }
            }
        });

        dialog.present();
    }

    /// Shows a dialog for creating a new archive with format, compression, and optional password
    pub fn show_create_archive_dialog<F: FnOnce(PathBuf, String) + 'static>(
        parent: &adw::ApplicationWindow,
        source_paths: &[PathBuf],
        callback: F,
    ) {
        let file_chooser = gtk4::FileChooserNative::new(
            Some("Create Archive"),
            Some(parent),
            gtk4::FileChooserAction::Save,
            Some("Create"),
            Some("Cancel"),
        );

        let default_name = if let Some(first) = source_paths.first() {
            if let Some(name) = first.file_name() {
                format!("{}.zip", name.to_string_lossy())
            } else {
                "archive.zip".to_string()
            }
        } else {
            "archive.zip".to_string()
        };

        file_chooser.set_current_name(&default_name);

        let zip_filter = gtk4::FileFilter::new();
        zip_filter.set_name(Some("Zip Archive (*.zip)"));
        zip_filter.add_pattern("*.zip");
        file_chooser.add_filter(&zip_filter);

        let tar_gz_filter = gtk4::FileFilter::new();
        tar_gz_filter.set_name(Some("Tar Gzip Archive (*.tar.gz)"));
        tar_gz_filter.add_pattern("*.tar.gz");
        file_chooser.add_filter(&tar_gz_filter);

        let tar_zst_filter = gtk4::FileFilter::new();
        tar_zst_filter.set_name(Some("Tar Zstd Archive (*.tar.zst)"));
        tar_zst_filter.add_pattern("*.tar.zst");
        file_chooser.add_filter(&tar_zst_filter);

        let cb = std::cell::RefCell::new(Some(callback));
        file_chooser.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        if let Some(cb_fn) = cb.borrow_mut().take() {
                            cb_fn(path, "zip".to_string());
                        }
                    }
                }
            }
        });

        file_chooser.show();
    }
}
