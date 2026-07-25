use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::fs;
use std::path::Path;

pub struct PreviewWindow;

impl PreviewWindow {
    pub fn show(parent: &adw::ApplicationWindow, file_path: &Path) {
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let window = adw::Window::builder()
            .transient_for(parent)
            .modal(true)
            .default_width(680)
            .default_height(500)
            .title(&format!("Quick Preview — {}", file_name))
            .build();

        let header = adw::HeaderBar::new();
        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
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
            let picture = gtk4::Picture::for_filename(file_path);
            picture.set_content_fit(gtk4::ContentFit::Contain);
            picture.set_can_shrink(true);
            picture.set_vexpand(true);
            picture.set_hexpand(true);
            main_box.append(&picture);
        } else {
            let scrolled = gtk4::ScrolledWindow::builder()
                .hscrollbar_policy(gtk4::PolicyType::Automatic)
                .vscrollbar_policy(gtk4::PolicyType::Automatic)
                .vexpand(true)
                .hexpand(true)
                .build();

            let text_view = gtk4::TextView::new();
            text_view.set_editable(false);
            text_view.set_cursor_visible(false);
            text_view.set_monospace(true);
            text_view.set_left_margin(12);
            text_view.set_right_margin(12);
            text_view.set_top_margin(12);
            text_view.set_bottom_margin(12);

            let text_content = fs::read_to_string(file_path).unwrap_or_else(|_| {
                format!(
                    "Binary file ({}) cannot be previewed as text.",
                    file_name
                )
            });

            // Truncate preview if text file is super large (> 500KB)
            let display_text = if text_content.len() > 500_000 {
                format!("{}\n\n... [Content truncated for preview]", &text_content[..500_000])
            } else {
                text_content
            };

            text_view.buffer().set_text(&display_text);
            scrolled.set_child(Some(&text_view));
            main_box.append(&scrolled);
        }

        let key_controller = gtk4::EventControllerKey::new();
        let win_clone = window.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape || key == gtk4::gdk::Key::space {
                win_clone.close();
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });

        window.add_controller(key_controller);
        window.set_content(Some(&main_box));
        window.present();
    }
}
