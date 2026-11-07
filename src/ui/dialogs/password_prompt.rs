use crate::archive::EncryptionMethod;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, DropDown, Justification, Label, Orientation, PasswordEntry};
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow, MessageDialog, ResponseAppearance};
use std::cell::RefCell;

pub fn prompt_encrypt_archive<F: FnOnce(Option<(String, EncryptionMethod)>) + 'static>(
    parent: &ApplicationWindow,
    callback: F,
) {
    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .heading("Protect Archive with Password")
        .body("Choose a password and encryption algorithm:")
        .build();

    let content_box = GtkBox::new(Orientation::Vertical, 8);
    content_box.set_margin_top(8);
    content_box.set_margin_bottom(8);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);
    content_box.set_hexpand(true);
    content_box.set_halign(Align::Fill);

    let pwd_label = Label::new(Some("Password:"));
    pwd_label.set_xalign(0.0);
    pwd_label.add_css_class("dim-label");
    content_box.append(&pwd_label);

    let entry = PasswordEntry::builder()
        .placeholder_text("Enter password")
        .show_peek_icon(true)
        .width_request(280)
        .halign(Align::Fill)
        .build();
    content_box.append(&entry);

    let method_label = Label::new(Some("Encryption Algorithm:"));
    method_label.set_xalign(0.0);
    method_label.add_css_class("dim-label");
    content_box.append(&method_label);

    let methods = &[
        EncryptionMethod::Aes256.label(),
        EncryptionMethod::Aes192.label(),
        EncryptionMethod::Aes128.label(),
        EncryptionMethod::ZipCrypto.label(),
    ];
    let method_dropdown = DropDown::from_strings(methods);
    method_dropdown.set_selected(0);
    method_dropdown.set_halign(Align::Fill);
    content_box.append(&method_dropdown);

    dialog.set_extra_child(Some(&content_box));

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("encrypt", "Encrypt");
    dialog.set_response_appearance("encrypt", ResponseAppearance::Suggested);

    let cb = RefCell::new(Some(callback));

    entry.connect_activate({
        let dialog = dialog.clone();
        move |_| {
            dialog.response("encrypt");
        }
    });

    dialog.connect_response(None, move |d, response| {
        d.close();
        if let Some(cb_fn) = cb.borrow_mut().take() {
            if response == "encrypt" {
                let text = entry.text().to_string();
                if text.is_empty() {
                    cb_fn(None);
                } else {
                    let method = match method_dropdown.selected() {
                        0 => EncryptionMethod::Aes256,
                        1 => EncryptionMethod::Aes192,
                        2 => EncryptionMethod::Aes128,
                        3 => EncryptionMethod::ZipCrypto,
                        _ => EncryptionMethod::Aes256,
                    };
                    cb_fn(Some((text, method)));
                }
            } else {
                cb_fn(None);
            }
        }
    });

    dialog.present();
}

pub fn prompt_password<F: FnOnce(Option<String>) + 'static>(
    parent: &ApplicationWindow,
    title_text: &str,
    callback: F,
) {
    prompt_password_full(parent, title_text, None, callback);
}

pub fn prompt_password_full<F: FnOnce(Option<String>) + 'static>(
    parent: &ApplicationWindow,
    title_text: &str,
    error_msg: Option<&str>,
    callback: F,
) {
    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .heading("Password Required")
        .body(title_text)
        .build();

    let content_box = GtkBox::new(Orientation::Vertical, 6);
    content_box.set_margin_top(8);
    content_box.set_margin_bottom(8);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);
    content_box.set_hexpand(true);
    content_box.set_halign(Align::Fill);

    if let Some(err) = error_msg {
        let err_label = Label::new(Some(err));
        err_label.add_css_class("error");
        err_label.set_hexpand(true);
        err_label.set_halign(Align::Center);
        err_label.set_xalign(0.5);
        err_label.set_justify(Justification::Center);
        content_box.append(&err_label);
    }

    let entry = PasswordEntry::builder()
        .placeholder_text("Enter archive password")
        .show_peek_icon(true)
        .width_request(280)
        .halign(Align::Center)
        .build();

    content_box.append(&entry);
    dialog.set_extra_child(Some(&content_box));

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("ok", "Unlock");
    dialog.set_response_appearance("ok", ResponseAppearance::Suggested);

    let cb = RefCell::new(Some(callback));

    entry.connect_activate({
        let dialog = dialog.clone();
        move |_| {
            dialog.response("ok");
        }
    });

    dialog.connect_response(None, move |d, response| {
        d.close();
        if let Some(cb_fn) = cb.borrow_mut().take() {
            let pwd = (response == "ok")
                .then(|| entry.text().to_string())
                .filter(|s| !s.is_empty());
            cb_fn(pwd);
        }
    });

    dialog.present();
}
