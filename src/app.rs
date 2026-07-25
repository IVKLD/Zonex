use crate::ui::ZonexWindow;
use gtk4::gio;
use gtk4::prelude::*;
use libadwaita as adw;
use std::env;

pub struct ZonexApplication {
    app: adw::Application,
}

impl ZonexApplication {
    pub fn new() -> Self {
        // Enforce Wayland backend
        env::set_var("GDK_BACKEND", "wayland");

        let app = adw::Application::builder()
            .application_id("io.github.zonex.ArchiveManager")
            .flags(gio::ApplicationFlags::HANDLES_OPEN)
            .build();

        app.connect_activate(|app| {
            let window = ZonexWindow::new(app);
            window.present();
        });

        app.connect_open(|app, files, _hint| {
            let window = ZonexWindow::new(app);
            if let Some(file) = files.first() {
                if let Some(path) = file.path() {
                    window.open_path(path);
                }
            }
            window.present();
        });

        Self { app }
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.run()
    }
}
