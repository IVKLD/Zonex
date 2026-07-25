use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation, ProgressBar};

pub struct StatusBar {
    container: GtkBox,
    status_label: Label,
    #[allow(dead_code)]
    progress_bar: ProgressBar,
}

impl StatusBar {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(6);
        container.set_margin_bottom(6);

        let status_label = Label::new(Some("Ready (Drop files to open or create archive)"));
        status_label.set_hexpand(true);
        status_label.set_xalign(0.0);
        status_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        status_label.set_width_chars(1);
        status_label.set_max_width_chars(1);

        let progress_bar = ProgressBar::new();
        progress_bar.set_visible(false);
        progress_bar.set_width_request(150);

        container.append(&status_label);
        container.append(&progress_bar);

        Self {
            container,
            status_label,
            progress_bar,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn set_text(&self, text: &str) {
        self.status_label.set_text(text);
        self.status_label.set_tooltip_text(Some(text));
    }

    #[allow(dead_code)]
    pub fn set_progress(&self, visible: bool, fraction: Option<f64>, pulse: bool) {
        self.progress_bar.set_visible(visible);
        if pulse {
            self.progress_bar.pulse();
        } else if let Some(frac) = fraction {
            self.progress_bar.set_fraction(frac);
        }
    }
}
