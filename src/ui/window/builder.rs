use gdk4::Display;
use gio::Menu;
use gtk4::prelude::*;
use gtk4::{
    style_context_add_provider_for_display, Box as GtkBox, Button, CssProvider, IconTheme,
    MenuButton, Orientation, Spinner, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use libadwaita::{Application, ApplicationWindow, HeaderBar, ToastOverlay};

pub struct WindowLayout {
    pub window: ApplicationWindow,
    pub toast_overlay: ToastOverlay,
    pub open_btn: Button,
    pub spinner: Spinner,
    pub main_box: GtkBox,
}

fn setup_css() {
    if let Some(display) = Display::default() {
        let icon_theme = IconTheme::for_display(&display);
        let local_icon_dir = std::env::current_dir()
            .unwrap_or_default()
            .join("data/icons");
        if local_icon_dir.exists() {
            icon_theme.add_search_path(local_icon_dir);
        }

        let provider = CssProvider::new();
        provider.load_from_string(
            "
            .password-dialog-scrim {
                background-color: rgba(0, 0, 0, 0.55);
            }
            .password-dialog-card {
                background-color: @dialog_bg_color;
                color: @dialog_fg_color;
                border-radius: 16px;
                box-shadow: 0 16px 36px rgba(0, 0, 0, 0.45);
                border: 1px solid rgba(255, 255, 255, 0.15);
                padding: 24px;
            }
            ",
        );
        style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

pub fn build(app: &Application) -> WindowLayout {
    setup_css();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Zonex - Wayland Archive Manager")
        .icon_name("io.github.zonex.ArchiveManager")
        .default_width(800)
        .default_height(550)
        .build();

    let toast_overlay = ToastOverlay::new();

    let header_bar = HeaderBar::new();
    let open_btn = Button::with_label("Open Archive");
    let spinner = Spinner::new();
    spinner.set_visible(false);

    let menu = Menu::new();
    menu.append(Some("New Archive..."), Some("win.new_archive"));
    menu.append(Some("New Folder..."), Some("win.new_folder"));
    menu.append(Some("Export Selected..."), Some("win.export_selected"));
    menu.append(Some("Protect with Password..."), Some("win.set_password"));
    menu.append(Some("Test Integrity"), Some("win.test_integrity"));
    menu.append(Some("Delete Selected"), Some("win.delete_selected"));
    menu.append(Some("Select All"), Some("win.select_all"));
    menu.append(Some("Deselect All"), Some("win.deselect_all"));

    let menu_button = MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .build();

    header_bar.pack_start(&open_btn);
    header_bar.pack_end(&menu_button);
    header_bar.pack_end(&spinner);

    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.append(&header_bar);

    WindowLayout {
        window,
        toast_overlay,
        open_btn,
        spinner,
        main_box,
    }
}
