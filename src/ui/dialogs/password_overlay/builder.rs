use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Image, Justification, Label, Orientation, PasswordEntry};

pub struct PasswordOverlayElements {
    pub container: GtkBox,
    pub err_label: Label,
    pub entry: PasswordEntry,
    pub ok_btn: Button,
    pub cancel_btn: Button,
}

pub fn build() -> PasswordOverlayElements {
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.set_hexpand(true);
    container.set_vexpand(true);
    container.set_halign(Align::Fill);
    container.set_valign(Align::Fill);
    container.add_css_class("password-dialog-scrim");
    container.set_visible(false);

    let center_wrapper = GtkBox::new(Orientation::Vertical, 0);
    center_wrapper.set_hexpand(true);
    center_wrapper.set_vexpand(true);
    center_wrapper.set_halign(Align::Center);
    center_wrapper.set_valign(Align::Center);

    let card_box = GtkBox::new(Orientation::Vertical, 14);
    card_box.add_css_class("password-dialog-card");
    card_box.set_width_request(360);
    card_box.set_halign(Align::Center);
    card_box.set_valign(Align::Center);

    let lock_icon = Image::from_icon_name("dialog-password-symbolic");
    lock_icon.set_pixel_size(48);
    lock_icon.set_halign(Align::Center);
    lock_icon.add_css_class("accent");

    let title_label = Label::new(Some("Password Required"));
    title_label.add_css_class("title-1");
    title_label.set_halign(Align::Center);

    let subtitle_label = Label::new(Some(
        "Archive is password-protected. Enter password to unlock:",
    ));
    subtitle_label.add_css_class("body");
    subtitle_label.add_css_class("dim-label");
    subtitle_label.set_wrap(true);
    subtitle_label.set_justify(Justification::Center);
    subtitle_label.set_halign(Align::Center);

    let err_label = Label::new(None);
    err_label.add_css_class("error");
    err_label.set_hexpand(true);
    err_label.set_halign(Align::Center);
    err_label.set_justify(Justification::Center);
    err_label.set_visible(false);

    let entry = PasswordEntry::builder()
        .placeholder_text("Enter archive password")
        .show_peek_icon(true)
        .width_request(280)
        .halign(Align::Center)
        .build();

    let button_box = GtkBox::new(Orientation::Horizontal, 12);
    button_box.set_halign(Align::Center);
    button_box.set_margin_top(8);

    let cancel_btn = Button::with_label("Cancel");
    cancel_btn.set_width_request(110);

    let ok_btn = Button::with_label("Unlock");
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

    PasswordOverlayElements {
        container,
        err_label,
        entry,
        ok_btn,
        cancel_btn,
    }
}
