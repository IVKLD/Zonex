mod builder;
mod context_menu;
mod events;
mod filter_sort;
pub mod icons;
mod render;

pub use icons::get_icon_name_for_entry;

use crate::archive::ArchiveEntry;
use events::setup_events;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DropDown, Label, ListBox, PopoverMenu, SearchEntry};
use render::render_entries;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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

impl Default for FileView {
    fn default() -> Self {
        Self::new()
    }
}

impl FileView {
    pub fn new() -> Self {
        let el = builder::build();

        let anchor_index: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
        let all_entries: Rc<RefCell<Vec<ArchiveEntry>>> = Rc::new(RefCell::new(Vec::new()));
        let displayed_entries: Rc<RefCell<Vec<ArchiveEntry>>> = Rc::new(RefCell::new(Vec::new()));

        setup_events(&el, &all_entries, &displayed_entries, &anchor_index);

        Self {
            container: el.container,
            list_box: el.list_box,
            path_label: el.path_label,
            up_button: el.up_button,
            forward_button: el.forward_button,
            search_entry: el.search_entry,
            sort_dropdown: el.sort_dropdown,
            all_entries,
            displayed_entries,
            anchor_index,
            _popover_menu: el.popover_menu,
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

    pub fn set_forward_sensitive(&self, sensitive: bool) {
        self.forward_button.set_sensitive(sensitive);
    }

    pub fn update_folder_path(&self, folder: &Path) {
        let path_str = if folder.as_os_str().is_empty() {
            "/".to_string()
        } else {
            format!("/{}", folder.display())
        };
        self.path_label.set_text(&path_str);
        self.path_label.set_tooltip_text(Some(&path_str));
        self.up_button.set_sensitive(!folder.as_os_str().is_empty());
    }

    pub fn set_entries(&self, entries: Vec<ArchiveEntry>) {
        *self.all_entries.borrow_mut() = entries;
        self.search_entry.set_text("");
        self.reapply_view();
    }

    pub fn reapply_view(&self) {
        render_entries(
            &self.search_entry,
            &self.sort_dropdown,
            &self.all_entries,
            &self.displayed_entries,
            &self.list_box,
            &self.anchor_index,
        );
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
