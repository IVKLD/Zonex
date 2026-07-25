use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::path::PathBuf;
use std::rc::Rc;

pub struct PasswordOverlayWidget {
    container: gtk4::Box,
    #[allow(dead_code)]
    card_box: gtk4::Box,
    err_label: gtk4::Label,
    entry: gtk4::PasswordEntry,
    ok_btn: gtk4::Button,
    cancel_btn: gtk4::Button,
    current_verify_fn: Rc<std::cell::RefCell<Option<Rc<dyn Fn(String)>>>>,
}

impl PasswordOverlayWidget {
    pub fn new() -> Self {
        // Semi-transparent backdrop dimmer layer
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.set_hexpand(true);
        container.set_vexpand(true);
        container.set_halign(gtk4::Align::Fill);
        container.set_valign(gtk4::Align::Fill);
        container.add_css_class("password-dialog-scrim");
        container.set_visible(false);

        // Center card container
        let center_wrapper = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        center_wrapper.set_hexpand(true);
        center_wrapper.set_vexpand(true);
        center_wrapper.set_halign(gtk4::Align::Center);
        center_wrapper.set_valign(gtk4::Align::Center);

        let card_box = gtk4::Box::new(gtk4::Orientation::Vertical, 14);
        card_box.add_css_class("password-dialog-card");
        card_box.set_width_request(360);
        card_box.set_halign(gtk4::Align::Center);
        card_box.set_valign(gtk4::Align::Center);

        let lock_icon = gtk4::Image::from_icon_name("dialog-password-symbolic");
        lock_icon.set_pixel_size(48);
        lock_icon.set_halign(gtk4::Align::Center);
        lock_icon.add_css_class("accent");

        let title_label = gtk4::Label::new(Some("Password Required"));
        title_label.add_css_class("title-1");
        title_label.set_halign(gtk4::Align::Center);

        let subtitle_label = gtk4::Label::new(Some("Archive is password-protected. Enter password to unlock:"));
        subtitle_label.add_css_class("body");
        subtitle_label.add_css_class("dim-label");
        subtitle_label.set_wrap(true);
        subtitle_label.set_justify(gtk4::Justification::Center);
        subtitle_label.set_halign(gtk4::Align::Center);

        let err_label = gtk4::Label::new(None);
        err_label.add_css_class("error");
        err_label.set_hexpand(true);
        err_label.set_halign(gtk4::Align::Center);
        err_label.set_justify(gtk4::Justification::Center);
        err_label.set_visible(false);

        let entry = gtk4::PasswordEntry::builder()
            .placeholder_text("Enter archive password")
            .show_peek_icon(true)
            .width_request(280)
            .halign(gtk4::Align::Center)
            .build();

        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        button_box.set_halign(gtk4::Align::Center);
        button_box.set_margin_top(8);

        let cancel_btn = gtk4::Button::with_label("Cancel");
        cancel_btn.set_width_request(110);

        let ok_btn = gtk4::Button::with_label("Unlock");
        ok_btn.add_css_class("suggested-action");
        ok_btn.set_width_request(110);

        button_box.append(&cancel_btn);
        button_box.append(&ok_btn);

        card_box.append(&lock_icon);
        card_box.append(&title_label);
        card_box.append(&subtitle_label);
        card_box.append(&err_label);
        card_box.append(&entry);
        card_box.append(&button_box);

        center_wrapper.append(&card_box);
        container.append(&center_wrapper);

        let current_verify_fn: Rc<std::cell::RefCell<Option<Rc<dyn Fn(String)>>>> =
            Rc::new(std::cell::RefCell::new(None));

        let current_fn_c = current_verify_fn.clone();
        let entry_c = entry.clone();
        let submit_action = move || {
            let pwd = entry_c.text().to_string();
            if let Some(ref verify) = *current_fn_c.borrow() {
                verify(pwd);
            }
        };

        let submit_c1 = Rc::new(submit_action);
        let submit_c2 = submit_c1.clone();

        entry.connect_activate(move |_| {
            submit_c1();
        });

        ok_btn.connect_clicked(move |_| {
            submit_c2();
        });

        let container_c = container.clone();
        cancel_btn.connect_clicked(move |_| {
            container_c.set_visible(false);
        });

        Self {
            container,
            card_box,
            err_label,
            entry,
            ok_btn,
            cancel_btn,
            current_verify_fn,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn is_visible(&self) -> bool {
        self.container.is_visible()
    }

    pub fn prompt<F>(&self, verify_fn: F)
    where
        F: Fn(String, Rc<dyn Fn(bool, &str)>) + 'static,
    {
        self.err_label.set_visible(false);
        self.err_label.set_text("");
        self.entry.set_text("");
        self.container.set_visible(true);
        self.entry.grab_focus();

        let verify_fn_rc = Rc::new(verify_fn);
        let err_lbl_c = self.err_label.clone();
        let entry_c = self.entry.clone();
        let container_c = self.container.clone();
        let ok_btn_c = self.ok_btn.clone();
        let cancel_btn_c = self.cancel_btn.clone();

        let verify_wrapper = move |pwd: String| {
            if pwd.is_empty() {
                err_lbl_c.set_text("Please enter a password");
                err_lbl_c.set_visible(true);
                return;
            }

            ok_btn_c.set_sensitive(false);
            cancel_btn_c.set_sensitive(false);

            let err_lbl_cb = err_lbl_c.clone();
            let entry_cb = entry_c.clone();
            let container_cb = container_c.clone();
            let ok_btn_cb = ok_btn_c.clone();
            let cancel_btn_cb = cancel_btn_c.clone();

            let on_done = Rc::new(move |success: bool, err_msg: &str| {
                ok_btn_cb.set_sensitive(true);
                cancel_btn_cb.set_sensitive(true);

                if success {
                    container_cb.set_visible(false);
                } else {
                    err_lbl_cb.set_text(if err_msg.is_empty() {
                        "❌ Incorrect password. Please try another password."
                    } else {
                        err_msg
                    });
                    err_lbl_cb.set_visible(true);
                    entry_cb.grab_focus();
                    entry_cb.select_region(0, -1);
                }
            });

            verify_fn_rc(pwd, on_done);
        };

        *self.current_verify_fn.borrow_mut() = Some(Rc::new(verify_wrapper));
    }
}

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
        let default_name = if let Some(first) = source_paths.first() {
            if let Some(name) = first.file_name() {
                format!("{}.zip", name.to_string_lossy())
            } else {
                "archive.zip".to_string()
            }
        } else {
            "archive.zip".to_string()
        };

        let zip_filter = gtk4::FileFilter::new();
        zip_filter.set_name(Some("Zip Archive (*.zip)"));
        zip_filter.add_pattern("*.zip");

        let tar_gz_filter = gtk4::FileFilter::new();
        tar_gz_filter.set_name(Some("Tar Gzip Archive (*.tar.gz)"));
        tar_gz_filter.add_pattern("*.tar.gz");

        let tar_zst_filter = gtk4::FileFilter::new();
        tar_zst_filter.set_name(Some("Tar Zstd Archive (*.tar.zst)"));
        tar_zst_filter.add_pattern("*.tar.zst");

        let filters = gio::ListStore::new::<gtk4::FileFilter>();
        filters.append(&zip_filter);
        filters.append(&tar_gz_filter);
        filters.append(&tar_zst_filter);

        let dialog = gtk4::FileDialog::builder()
            .title("Create Archive")
            .initial_name(&default_name)
            .filters(&filters)
            .build();

        let cb = std::cell::RefCell::new(Some(callback));
        dialog.save(Some(parent), gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    if let Some(cb_fn) = cb.borrow_mut().take() {
                        cb_fn(path, "zip".to_string());
                    }
                }
            }
        });
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
