use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::path::PathBuf;
use std::rc::Rc;

pub struct Dialogs;

impl Dialogs {
    /// Shows a password prompt dialog using Libadwaita
    pub fn prompt_password<F: FnOnce(Option<String>) + 'static>(
        parent: &adw::ApplicationWindow,
        title_text: &str,
        callback: F,
    ) {
        Self::prompt_password_full(parent, title_text, None, callback);
    }

    /// Shows a persistent password prompt dialog that verifies password until valid or cancelled
    pub fn prompt_password_verify<F>(
        parent: &adw::ApplicationWindow,
        title_text: &str,
        verify_fn: F,
    ) where
        F: Fn(String, Rc<dyn Fn(bool, &str)>) + 'static,
    {
        let dialog = adw::MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .heading("Password Required")
            .body(title_text)
            .build();

        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        content_box.set_margin_top(8);
        content_box.set_margin_bottom(8);
        content_box.set_margin_start(16);
        content_box.set_margin_end(16);
        content_box.set_hexpand(true);
        content_box.set_halign(gtk4::Align::Fill);

        let err_label = gtk4::Label::new(None);
        err_label.add_css_class("error");
        err_label.set_hexpand(true);
        err_label.set_halign(gtk4::Align::Center);
        err_label.set_xalign(0.5);
        err_label.set_justify(gtk4::Justification::Center);
        err_label.set_visible(false);
        content_box.append(&err_label);

        let entry = gtk4::PasswordEntry::builder()
            .placeholder_text("Enter archive password")
            .show_peek_icon(true)
            .width_request(280)
            .halign(gtk4::Align::Center)
            .build();

        content_box.append(&entry);
        dialog.set_extra_child(Some(&content_box));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("ok", "Unlock");
        dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);

        let verify_fn_rc = Rc::new(verify_fn);
        let dialog_c = dialog.clone();
        let err_label_c = err_label.clone();
        let entry_c = entry.clone();

        let perform_unlock = Rc::new(move || {
            let pwd = entry_c.text().to_string();
            if pwd.is_empty() {
                err_label_c.set_text("Please enter a password");
                err_label_c.set_visible(true);
                return;
            }

            dialog_c.set_response_enabled("ok", false);
            dialog_c.set_response_enabled("cancel", false);

            let d_sub = dialog_c.clone();
            let err_lbl_sub = err_label_c.clone();
            let entry_sub = entry_c.clone();

            let on_done = Rc::new(move |success: bool, err_msg: &str| {
                d_sub.set_response_enabled("ok", true);
                d_sub.set_response_enabled("cancel", true);

                if success {
                    d_sub.close();
                } else {
                    err_lbl_sub.set_text(if err_msg.is_empty() {
                        "❌ Incorrect password. Please try again."
                    } else {
                        err_msg
                    });
                    err_lbl_sub.set_visible(true);
                    entry_sub.grab_focus();
                    entry_sub.select_region(0, -1);
                }
            });

            verify_fn_rc(pwd, on_done);
        });

        let perform_unlock_c1 = perform_unlock.clone();
        entry.connect_activate(move |_| {
            perform_unlock_c1();
        });

        dialog.connect_response(None, move |d, response| {
            if response == "cancel" {
                d.close();
            } else if response == "ok" {
                perform_unlock();
            }
        });

        dialog.present();
    }

    /// Shows a password prompt dialog using Libadwaita with optional error feedback
    pub fn prompt_password_full<F: FnOnce(Option<String>) + 'static>(
        parent: &adw::ApplicationWindow,
        title_text: &str,
        error_msg: Option<&str>,
        callback: F,
    ) {
        let dialog = adw::MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .heading("Password Required")
            .body(title_text)
            .build();

        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        content_box.set_margin_top(8);
        content_box.set_margin_bottom(8);
        content_box.set_margin_start(16);
        content_box.set_margin_end(16);
        content_box.set_hexpand(true);
        content_box.set_halign(gtk4::Align::Fill);

        if let Some(err) = error_msg {
            let err_label = gtk4::Label::new(Some(err));
            err_label.add_css_class("error");
            err_label.set_hexpand(true);
            err_label.set_halign(gtk4::Align::Center);
            err_label.set_xalign(0.5);
            err_label.set_justify(gtk4::Justification::Center);
            content_box.append(&err_label);
        }

        let entry = gtk4::PasswordEntry::builder()
            .placeholder_text("Enter archive password")
            .show_peek_icon(true)
            .width_request(280)
            .halign(gtk4::Align::Center)
            .build();

        content_box.append(&entry);
        dialog.set_extra_child(Some(&content_box));

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

        dialog.connect_response(None, move |d, response| {
            d.close();
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

    /// Shows a dialog displaying entry properties (name, path, size, type)
    pub fn show_entry_properties(
        parent: &adw::ApplicationWindow,
        entry: &crate::archive::ArchiveEntry,
    ) {
        let title = format!("Properties: {}", entry.file_name());
        let body = format!(
            "Name: {}\nRelative Path: {}\nType: {}\nSize: {}",
            entry.file_name(),
            entry.path.display(),
            if entry.is_dir { "Directory" } else { "File" },
            entry.formatted_size()
        );

        let dialog = adw::MessageDialog::builder()
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
}
