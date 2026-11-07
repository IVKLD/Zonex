use gdk4::Key;
use glib::Propagation;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, ContentFit, EventControllerKey, Orientation, Picture, PolicyType,
    ScrolledWindow, TextView,
};
use libadwaita::prelude::*;
use libadwaita::{ApplicationWindow, HeaderBar, Window as AdwWindow};
use std::fs;
use std::io::Read;
use std::path::Path;

fn read_preview_text(file_path: &Path, file_name: &str) -> (String, bool) {
    let Ok(file) = fs::File::open(file_path) else {
        return (format!("Could not read file ({file_name})"), false);
    };

    let mut bytes = Vec::new();
    if file.take(512 * 1024 + 1).read_to_end(&mut bytes).is_err() {
        return (
            format!("Binary file ({file_name}) cannot be previewed as text."),
            false,
        );
    }

    let is_truncated = bytes.len() > 512 * 1024;
    bytes.truncate(512 * 1024);

    match String::from_utf8(bytes) {
        Ok(text) => (text, is_truncated),
        Err(e) => {
            let valid = e.utf8_error().valid_up_to();
            let mut b = e.into_bytes();
            b.truncate(valid);
            let text = String::from_utf8(b).unwrap_or_default();
            (format!("{text}[Binary sequence suppressed]"), is_truncated)
        }
    }
}

pub fn show(parent: &ApplicationWindow, file_path: &Path) {
    let file_name = file_path
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| "File Preview".into());

    let window = AdwWindow::builder()
        .transient_for(parent)
        .modal(true)
        .title(format!("Preview: {file_name}"))
        .default_width(700)
        .default_height(500)
        .build();

    let header = HeaderBar::new();
    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.append(&header);

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let is_image = matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "svg" | "bmp" | "ico" | "gif"
    );

    if is_image {
        let picture = Picture::for_filename(file_path);
        picture.set_content_fit(ContentFit::Contain);
        picture.set_can_shrink(true);
        picture.set_vexpand(true);
        picture.set_hexpand(true);
        main_box.append(&picture);
    } else {
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .build();

        let text_view = TextView::new();
        text_view.set_editable(false);
        text_view.set_cursor_visible(false);
        text_view.set_monospace(true);
        text_view.set_left_margin(12);
        text_view.set_right_margin(12);
        text_view.set_top_margin(12);
        text_view.set_bottom_margin(12);

        let (display_text, is_truncated) = read_preview_text(file_path, &file_name);

        let final_text = if is_truncated {
            let limit = display_text.floor_char_boundary(500_000);
            format!(
                "{}\n\n... [Content truncated for preview]",
                &display_text[..limit]
            )
        } else {
            display_text
        };

        text_view.buffer().set_text(&final_text);
        scrolled.set_child(Some(&text_view));
        main_box.append(&scrolled);
    }

    let key_controller = EventControllerKey::new();
    let win_clone = window.clone();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == Key::Escape || key == Key::space {
            win_clone.close();
            Propagation::Stop
        } else {
            Propagation::Proceed
        }
    });

    window.add_controller(key_controller);
    window.set_content(Some(&main_box));
    window.present();
}
