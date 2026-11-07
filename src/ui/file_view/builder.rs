use super::context_menu::build_context_menu;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Label, ListBox, Orientation, PopoverMenu, SearchEntry,
    SelectionMode,
};

pub struct FileViewElements {
    pub container: GtkBox,
    pub list_box: ListBox,
    pub path_label: Label,
    pub up_button: Button,
    pub forward_button: Button,
    pub search_entry: SearchEntry,
    pub sort_dropdown: DropDown,
    pub popover_menu: PopoverMenu,
}

pub fn build() -> FileViewElements {
    let container = GtkBox::new(Orientation::Vertical, 0);
    let nav_bar = GtkBox::new(Orientation::Horizontal, 8);
    nav_bar.set_margin_start(12);
    nav_bar.set_margin_end(12);
    nav_bar.set_margin_top(6);
    nav_bar.set_margin_bottom(6);

    let up_button = Button::from_icon_name("go-up-symbolic");
    up_button.set_tooltip_text(Some(
        "Back / Parent Directory (Backspace / Alt+Left / Mouse Back)",
    ));

    let forward_button = Button::from_icon_name("go-next-symbolic");
    forward_button.set_tooltip_text(Some("Forward (Alt+Right / Mouse Forward)"));
    forward_button.set_sensitive(false);

    let path_label = Label::new(Some("/"));
    path_label.add_css_class("title-4");
    path_label.set_xalign(0.0);
    path_label.set_hexpand(true);
    path_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    path_label.set_width_chars(1);
    path_label.set_max_width_chars(1);

    let search_entry = SearchEntry::new();
    search_entry.set_placeholder_text(Some("Search files..."));
    search_entry.set_width_request(120);

    let sort_options = &[
        "Folders First",
        "Name (A-Z)",
        "Name (Z-A)",
        "Size (Largest)",
        "Size (Smallest)",
    ];
    let sort_dropdown = DropDown::from_strings(sort_options);
    sort_dropdown.set_selected(0);
    sort_dropdown.set_tooltip_text(Some("Sort entries"));

    nav_bar.append(&up_button);
    nav_bar.append(&forward_button);
    nav_bar.append(&path_label);
    nav_bar.append(&search_entry);
    nav_bar.append(&sort_dropdown);
    container.append(&nav_bar);

    let list_box = ListBox::new();
    list_box.set_selection_mode(SelectionMode::Multiple);
    list_box.set_activate_on_single_click(false);
    list_box.add_css_class("rich-list");

    let menu_model = build_context_menu();
    let popover_menu = PopoverMenu::from_model(Some(&menu_model));
    popover_menu.set_parent(&list_box);
    popover_menu.set_has_arrow(false);

    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .child(&list_box)
        .vexpand(true)
        .build();

    container.append(&scroll);

    FileViewElements {
        container,
        list_box,
        path_label,
        up_button,
        forward_button,
        search_entry,
        sort_dropdown,
        popover_menu,
    }
}
