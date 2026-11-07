pub mod actions;
pub mod builder;
pub mod handlers;

use actions::ActionsContext;
use handlers::{
    create_open_handler, setup_navigation, setup_open_dialog, setup_shortcuts, setup_window_drag,
    setup_window_drop,
};

use super::file_view::FileView;
use super::status_bar::StatusBar;
use crate::archive::ArchiveManager;
use crate::ui::dialogs::PasswordOverlayWidget;
use crate::utils::TempCache;
use gtk4::prelude::*;
use gtk4::Overlay;
use libadwaita::prelude::*;
use libadwaita::{Application, ApplicationWindow, Toast};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct ZonexWindow {
    window: ApplicationWindow,
    _manager: Arc<Mutex<ArchiveManager>>,
    _temp_cache: Rc<TempCache>,
    open_path_fn: Rc<dyn Fn(PathBuf)>,
}

impl ZonexWindow {
    pub fn new(app: &Application) -> Self {
        let manager = Arc::new(Mutex::new(ArchiveManager::new()));
        let temp_cache = Rc::new(TempCache::new().expect("Failed to initialize temp cache"));

        let layout = builder::build(app);

        let file_view = Rc::new(FileView::new());
        let status_bar = Rc::new(StatusBar::new());
        let pwd_overlay = Rc::new(PasswordOverlayWidget::new());

        let content_overlay = Overlay::new();
        content_overlay.set_child(Some(file_view.widget()));
        content_overlay.add_overlay(pwd_overlay.widget());

        layout.main_box.append(&content_overlay);
        layout.main_box.append(status_bar.widget());
        layout.toast_overlay.set_child(Some(&layout.main_box));
        layout.window.set_content(Some(&layout.toast_overlay));

        let toast_overlay_c = layout.toast_overlay.clone();
        let show_toast = move |msg: &str| {
            let toast = Toast::new(msg);
            toast.set_timeout(3);
            toast_overlay_c.add_toast(toast);
        };
        let show_toast_rc = Rc::new(show_toast);

        let spinner_c = layout.spinner.clone();
        let status_c = status_bar.clone();
        let set_loading = move |loading: bool, msg: &str| {
            if loading {
                spinner_c.start();
            } else {
                spinner_c.stop();
            }
            status_c.set_text(msg);
        };
        let set_loading_rc = Rc::new(set_loading);

        let status_metrics = status_bar;
        file_view.connect_stats_changed(move |stats| {
            status_metrics.update_metrics(
                stats.total_count,
                stats.total_size,
                stats.selected_count,
                stats.selected_size,
            );
        });

        setup_navigation(&file_view, &manager);

        let do_open_rc = create_open_handler(
            manager.clone(),
            temp_cache.clone(),
            file_view.clone(),
            pwd_overlay.clone(),
            show_toast_rc.clone(),
            set_loading_rc.clone(),
        );

        let actions_ctx = ActionsContext {
            window: layout.window.clone(),
            manager: manager.clone(),
            temp_cache: temp_cache.clone(),
            file_view: file_view.clone(),
            password_overlay: pwd_overlay,
            show_toast: show_toast_rc.clone(),
            set_loading: set_loading_rc.clone(),
            do_open: do_open_rc.clone(),
        };
        actions::register(&actions_ctx);

        setup_window_drop(
            &layout.window,
            manager.clone(),
            file_view.clone(),
            set_loading_rc.clone(),
            show_toast_rc.clone(),
            do_open_rc.clone(),
        );

        setup_window_drag(
            file_view.clone(),
            manager.clone(),
            temp_cache.clone(),
            show_toast_rc.clone(),
        );

        setup_open_dialog(&layout.open_btn, &file_view, do_open_rc.clone());

        let key_controller = setup_shortcuts(&file_view, &layout.open_btn);
        layout.window.add_controller(key_controller);

        Self {
            window: layout.window,
            _manager: manager,
            _temp_cache: temp_cache,
            open_path_fn: do_open_rc,
        }
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn open_path(&self, path: PathBuf) {
        (self.open_path_fn)(path);
    }
}
