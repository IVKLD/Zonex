use crate::ui::ZonexWindow;
use gio::ApplicationFlags;
use gtk4::prelude::*;
use libadwaita::Application;
use std::env;

pub struct ZonexApplication {
    app: Application,
}

impl Default for ZonexApplication {
    fn default() -> Self {
        Self::new()
    }
}

impl ZonexApplication {
    pub fn new() -> Self {
        env::set_var("GDK_BACKEND", "wayland");

        let app = Application::builder()
            .application_id("io.github.zonex.ArchiveManager")
            .flags(ApplicationFlags::HANDLES_OPEN)
            .build();

        app.connect_activate(|app| {
            let window = ZonexWindow::new(app);
            window.present();
        });

        app.connect_open(|app, files, _hint| {
            let window = ZonexWindow::new(app);
            if let Some(path) = files.first().and_then(|file| file.path()) {
                window.open_path(path);
            }
            window.present();
        });

        Self { app }
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.run()
    }
}
