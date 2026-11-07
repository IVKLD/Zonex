use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation};

pub struct StatusBar {
    container: GtkBox,
    status_label: Label,
    metrics_label: Label,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
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

        let metrics_label = Label::new(None);
        metrics_label.set_xalign(1.0);
        metrics_label.add_css_class("dim-label");
        metrics_label.set_single_line_mode(true);

        container.append(&status_label);
        container.append(&metrics_label);

        Self {
            container,
            status_label,
            metrics_label,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn set_text(&self, text: &str) {
        self.status_label.set_text(text);
        self.status_label.set_tooltip_text(Some(text));
    }

    pub fn update_metrics(
        &self,
        total_count: usize,
        total_size: u64,
        selected_count: usize,
        selected_size: u64,
    ) {
        if total_count == 0 {
            self.metrics_label.set_text("");
            return;
        }

        let total_formatted = format_size(total_size);
        let items_suffix = if total_count == 1 { "item" } else { "items" };

        let text = if selected_count > 0 {
            format!(
                "{} of {} selected ({}) • Total: {}",
                selected_count,
                total_count,
                format_size(selected_size),
                total_formatted,
            )
        } else {
            format!("{} {} • {}", total_count, items_suffix, total_formatted)
        };

        self.metrics_label.set_text(&text);
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
