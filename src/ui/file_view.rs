use crate::archive::ArchiveEntry;
use gtk4::gdk;
use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, GestureClick, Image, Label, ListBox, ListBoxRow, Orientation,
    PopoverMenu, SearchEntry, SelectionMode,
};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Zero-allocation MIME-type icon lookup matching extensions without heap String creation
pub fn get_icon_name_for_entry(entry: &ArchiveEntry) -> &'static str {
    if entry.is_dir {
        return "folder-symbolic";
    }

    let ext_str = match entry.path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext,
        None => return "text-x-generic-symbolic",
    };

    if ext_str.eq_ignore_ascii_case("zip")
        || ext_str.eq_ignore_ascii_case("tar")
        || ext_str.eq_ignore_ascii_case("gz")
        || ext_str.eq_ignore_ascii_case("bz2")
        || ext_str.eq_ignore_ascii_case("xz")
        || ext_str.eq_ignore_ascii_case("zst")
        || ext_str.eq_ignore_ascii_case("7z")
        || ext_str.eq_ignore_ascii_case("rar")
        || ext_str.eq_ignore_ascii_case("iso")
        || ext_str.eq_ignore_ascii_case("deb")
        || ext_str.eq_ignore_ascii_case("rpm")
    {
        return "package-x-generic-symbolic";
    }

    if ext_str.eq_ignore_ascii_case("png")
        || ext_str.eq_ignore_ascii_case("jpg")
        || ext_str.eq_ignore_ascii_case("jpeg")
        || ext_str.eq_ignore_ascii_case("gif")
        || ext_str.eq_ignore_ascii_case("svg")
        || ext_str.eq_ignore_ascii_case("webp")
        || ext_str.eq_ignore_ascii_case("bmp")
        || ext_str.eq_ignore_ascii_case("ico")
    {
        return "image-x-generic-symbolic";
    }

    if ext_str.eq_ignore_ascii_case("rs")
        || ext_str.eq_ignore_ascii_case("py")
        || ext_str.eq_ignore_ascii_case("c")
        || ext_str.eq_ignore_ascii_case("cpp")
        || ext_str.eq_ignore_ascii_case("h")
        || ext_str.eq_ignore_ascii_case("hpp")
        || ext_str.eq_ignore_ascii_case("js")
        || ext_str.eq_ignore_ascii_case("ts")
        || ext_str.eq_ignore_ascii_case("json")
        || ext_str.eq_ignore_ascii_case("toml")
        || ext_str.eq_ignore_ascii_case("yaml")
        || ext_str.eq_ignore_ascii_case("yml")
        || ext_str.eq_ignore_ascii_case("sh")
        || ext_str.eq_ignore_ascii_case("nix")
    {
        return "text-x-script-symbolic";
    }

    if ext_str.eq_ignore_ascii_case("mp4")
        || ext_str.eq_ignore_ascii_case("mkv")
        || ext_str.eq_ignore_ascii_case("avi")
        || ext_str.eq_ignore_ascii_case("webm")
        || ext_str.eq_ignore_ascii_case("mov")
        || ext_str.eq_ignore_ascii_case("flv")
    {
        return "video-x-generic-symbolic";
    }

    if ext_str.eq_ignore_ascii_case("mp3")
        || ext_str.eq_ignore_ascii_case("flac")
        || ext_str.eq_ignore_ascii_case("wav")
        || ext_str.eq_ignore_ascii_case("ogg")
        || ext_str.eq_ignore_ascii_case("m4a")
        || ext_str.eq_ignore_ascii_case("aac")
    {
        return "audio-x-generic-symbolic";
    }

    if ext_str.eq_ignore_ascii_case("exe")
        || ext_str.eq_ignore_ascii_case("bin")
        || ext_str.eq_ignore_ascii_case("so")
        || ext_str.eq_ignore_ascii_case("dll")
        || ext_str.eq_ignore_ascii_case("appimage")
    {
        return "application-x-executable-symbolic";
    }

    "text-x-generic-symbolic"
}

pub struct FileView {
    container: GtkBox,
    list_box: ListBox,
    path_label: Label,
    up_button: Button,
    forward_button: Button,
    search_entry: SearchEntry,
    sort_dropdown: DropDown,
    all_entries: Rc<RefCell<Vec<ArchiveEntry>>>,
    displayed_entries: Rc<RefCell<Vec<ArchiveEntry>>>,
    anchor_index: Rc<RefCell<Option<usize>>>,
    _popover_menu: PopoverMenu,
}

impl FileView {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);

        // Navigation Bar: Back/Up button, Forward button, Path Label, Search Bar, Sort Selector
        let nav_bar = GtkBox::new(Orientation::Horizontal, 8);
        nav_bar.set_margin_start(12);
        nav_bar.set_margin_end(12);
        nav_bar.set_margin_top(6);
        nav_bar.set_margin_bottom(6);

        let up_button = Button::from_icon_name("go-up-symbolic");
        up_button.set_tooltip_text(Some("Back / Parent Directory (Backspace / Alt+Left / Mouse Back)"));

        let forward_button = Button::from_icon_name("go-next-symbolic");
        forward_button.set_tooltip_text(Some("Forward (Alt+Right / Mouse Forward)"));
        forward_button.set_sensitive(false);

        let path_label = Label::new(Some("/"));
        path_label.add_css_class("title-4");
        path_label.set_xalign(0.0);
        path_label.set_hexpand(true);

        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search files..."));
        search_entry.set_width_request(180);

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

        // Main List View
        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::Multiple);
        // Включаем активацию ТОЛЬКО по двойному клику (или Enter), не по одинарному
        list_box.set_activate_on_single_click(false);
        list_box.add_css_class("rich-list");

        // Create Context Menu (PopoverMenu)
        let menu_model = gio::Menu::new();
        menu_model.append(Some("Open / Enter"), Some("win.open_entry"));
        menu_model.append(Some("Extract Selected..."), Some("win.extract_selected"));
        menu_model.append(Some("Add Files..."), Some("win.add_files"));
        menu_model.append(Some("Select All"), Some("win.select_all"));
        menu_model.append(Some("Deselect All"), Some("win.deselect_all"));

        let popover_menu = PopoverMenu::from_model(Some(&menu_model));
        popover_menu.set_parent(&list_box);
        popover_menu.set_has_arrow(false);

        let anchor_index: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

        let all_entries: Rc<RefCell<Vec<ArchiveEntry>>> = Rc::new(RefCell::new(Vec::new()));
        let displayed_entries: Rc<RefCell<Vec<ArchiveEntry>>> = Rc::new(RefCell::new(Vec::new()));

        // Gesture Controller for Mouse Side Buttons & Context Menu
        let click_gesture = GestureClick::new();
        click_gesture.set_button(0); // Respond to any mouse button

        let list_box_clone = list_box.clone();
        let popover_clone = popover_menu.clone();
        let up_btn_clone = up_button.clone();
        let forward_btn_clone = forward_button.clone();

        click_gesture.connect_pressed(move |gesture, _n_press, x, y| {
            let button = gesture.current_button();

            // 1. Мышь: Назад (Боковая кнопка 8 или 6)
            if button == 8 || button == 6 {
                if up_btn_clone.is_sensitive() {
                    up_btn_clone.emit_clicked();
                }
                return;
            }

            // 2. Мышь: Вперёд (Боковая кнопка 9 или 7)
            if button == 9 || button == 7 {
                if forward_btn_clone.is_sensitive() {
                    forward_btn_clone.emit_clicked();
                }
                return;
            }

            // 3. Правый клик -> Контекстное меню
            if button == gdk::BUTTON_SECONDARY {
                if let Some(row) = list_box_clone.row_at_y(y as i32) {
                    if !row.is_selected() {
                        list_box_clone.unselect_all();
                        list_box_clone.select_row(Some(&row));
                    }
                }
                popover_clone.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                popover_clone.popup();
            }
        });

        list_box.add_controller(click_gesture);

        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&list_box)
            .vexpand(true)
            .build();

        container.append(&scroll);

        // Wire search entry & sort dropdown
        let search_clone = search_entry.clone();
        let sort_clone = sort_dropdown.clone();
        let all_e_clone = all_entries.clone();
        let disp_e_clone = displayed_entries.clone();
        let list_box_render = list_box.clone();
        let anchor_render = anchor_index.clone();

        let reapply = move || {
            Self::reapply_view_static(
                &search_clone,
                &sort_clone,
                &all_e_clone,
                &disp_e_clone,
                &list_box_render,
                &anchor_render,
            );
        };

        let reapply_rc = Rc::new(reapply);

        let reapply_search = reapply_rc.clone();
        search_entry.connect_search_changed(move |_| {
            reapply_search();
        });

        let reapply_sort = reapply_rc.clone();
        sort_dropdown.connect_selected_notify(move |_| {
            reapply_sort();
        });

        Self {
            container,
            list_box,
            path_label,
            up_button,
            forward_button,
            search_entry,
            sort_dropdown,
            all_entries,
            displayed_entries,
            anchor_index,
            _popover_menu: popover_menu,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn list_box(&self) -> &ListBox {
        &self.list_box
    }

    pub fn up_button(&self) -> &Button {
        &self.up_button
    }

    pub fn forward_button(&self) -> &Button {
        &self.forward_button
    }

    pub fn update_folder_path(&self, folder: &Path) {
        let path_str = if folder.as_os_str().is_empty() {
            "/".to_string()
        } else {
            format!("/{}", folder.display())
        };
        self.path_label.set_text(&path_str);
        self.up_button.set_sensitive(!folder.as_os_str().is_empty());
    }

    pub fn set_forward_sensitive(&self, sensitive: bool) {
        self.forward_button.set_sensitive(sensitive);
    }

    pub fn set_entries(&self, entries: Vec<ArchiveEntry>) {
        *self.all_entries.borrow_mut() = entries;
        self.search_entry.set_text("");
        self.reapply_view();
    }

    pub fn reapply_view(&self) {
        Self::reapply_view_static(
            &self.search_entry,
            &self.sort_dropdown,
            &self.all_entries,
            &self.displayed_entries,
            &self.list_box,
            &self.anchor_index,
        );
    }

    fn reapply_view_static(
        search_entry: &SearchEntry,
        sort_dropdown: &DropDown,
        all_entries: &Rc<RefCell<Vec<ArchiveEntry>>>,
        displayed_entries: &Rc<RefCell<Vec<ArchiveEntry>>>,
        list_box: &ListBox,
        anchor_index: &Rc<RefCell<Option<usize>>>,
    ) {
        let query = search_entry.text().to_lowercase();
        let sort_idx = sort_dropdown.selected();
        let all = all_entries.borrow();

        let mut filtered: Vec<ArchiveEntry> = all
            .iter()
            .filter(|e| {
                if query.is_empty() {
                    true
                } else {
                    e.file_name().to_lowercase().contains(&query)
                }
            })
            .cloned()
            .collect();

        match sort_idx {
            0 => {
                filtered.sort_by(|a, b| {
                    b.is_dir
                        .cmp(&a.is_dir)
                        .then_with(|| a.file_name().to_lowercase().cmp(&b.file_name().to_lowercase()))
                });
            }
            1 => {
                filtered.sort_by(|a, b| {
                    a.file_name().to_lowercase().cmp(&b.file_name().to_lowercase())
                });
            }
            2 => {
                filtered.sort_by(|a, b| {
                    b.file_name().to_lowercase().cmp(&a.file_name().to_lowercase())
                });
            }
            3 => {
                filtered.sort_by(|a, b| b.size.cmp(&a.size));
            }
            4 => {
                filtered.sort_by(|a, b| a.size.cmp(&b.size));
            }
            _ => {}
        }

        while let Some(row) = list_box.row_at_index(0) {
            list_box.remove(&row);
        }

        *displayed_entries.borrow_mut() = filtered.clone();
        *anchor_index.borrow_mut() = None;

        for entry in &filtered {
            let row = ListBoxRow::new();
            let hbox = GtkBox::new(Orientation::Horizontal, 12);
            hbox.set_margin_start(12);
            hbox.set_margin_end(12);
            hbox.set_margin_top(8);
            hbox.set_margin_bottom(8);

            let icon_name = get_icon_name_for_entry(entry);
            let icon = Image::from_icon_name(icon_name);

            let name_label = Label::new(Some(&entry.file_name()));
            name_label.set_hexpand(true);
            name_label.set_xalign(0.0);

            let size_label = Label::new(Some(&entry.formatted_size()));
            size_label.add_css_class("dim-label");

            hbox.append(&icon);
            hbox.append(&name_label);
            hbox.append(&size_label);

            row.set_child(Some(&hbox));
            list_box.append(&row);
        }
    }

    pub fn get_entry_at_index(&self, index: usize) -> Option<ArchiveEntry> {
        self.displayed_entries.borrow().get(index).cloned()
    }

    pub fn get_selected_entry_paths(&self) -> Vec<PathBuf> {
        let entries = self.displayed_entries.borrow();
        let selected_rows = self.list_box.selected_rows();

        selected_rows
            .iter()
            .filter_map(|row| {
                let idx = row.index() as usize;
                entries.get(idx).map(|e| e.path.clone())
            })
            .collect()
    }

    pub fn select_all(&self) {
        self.list_box.select_all();
    }

    pub fn deselect_all(&self) {
        self.list_box.unselect_all();
    }
}
