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
use gtk4::{
    Box as GtkBox, Button, DropDown, Label, ListBox, PopoverMenu, ScrolledWindow, SearchEntry,
};
use render::render_entries;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, Default)]
pub struct SelectionStats {
    pub total_count: usize,
    pub total_size: u64,
    pub selected_count: usize,
    pub selected_size: u64,
}

pub type PathNavigateCallback = Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>;
pub type StatsChangedCallback = Rc<RefCell<Option<Box<dyn Fn(SelectionStats)>>>>;

pub struct FileView {
    container: GtkBox,
    list_box: ListBox,
    breadcrumb_box: GtkBox,
    breadcrumb_scroll: ScrolledWindow,
    up_button: Button,
    forward_button: Button,
    search_entry: SearchEntry,
    sort_dropdown: DropDown,
    all_entries: Rc<RefCell<Vec<ArchiveEntry>>>,
    displayed_entries: Rc<RefCell<Vec<ArchiveEntry>>>,
    anchor_index: Rc<RefCell<Option<usize>>>,
    path_navigate_cb: PathNavigateCallback,
    stats_changed_cb: StatsChangedCallback,
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
        let path_navigate_cb: PathNavigateCallback = Rc::new(RefCell::new(None));
        let stats_changed_cb: StatsChangedCallback = Rc::new(RefCell::new(None));

        let stats_cb_for_events = stats_changed_cb.clone();
        let disp_for_stats = displayed_entries.clone();
        let list_box_for_stats = el.list_box.clone();

        let notify_stats = Rc::new(move || {
            if let Some(ref cb) = *stats_cb_for_events.borrow() {
                let displayed = disp_for_stats.borrow();
                let total_count = displayed.len();
                let total_size = displayed.iter().map(|e| e.size).sum();

                let selected_rows = list_box_for_stats.selected_rows();
                let selected_count = selected_rows.len();
                let selected_size = selected_rows
                    .iter()
                    .filter_map(|r| displayed.get(r.index() as usize))
                    .map(|e| e.size)
                    .sum();

                cb(SelectionStats {
                    total_count,
                    total_size,
                    selected_count,
                    selected_size,
                });
            }
        });

        setup_events(
            &el,
            &all_entries,
            &displayed_entries,
            &anchor_index,
            notify_stats,
        );

        Self {
            container: el.container,
            list_box: el.list_box,
            breadcrumb_box: el.breadcrumb_box,
            breadcrumb_scroll: el.breadcrumb_scroll,
            up_button: el.up_button,
            forward_button: el.forward_button,
            search_entry: el.search_entry,
            sort_dropdown: el.sort_dropdown,
            all_entries,
            displayed_entries,
            anchor_index,
            path_navigate_cb,
            stats_changed_cb,
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

    pub fn search_entry(&self) -> &SearchEntry {
        &self.search_entry
    }

    pub fn set_forward_sensitive(&self, sensitive: bool) {
        self.forward_button.set_sensitive(sensitive);
    }

    pub fn connect_path_navigate<F: Fn(PathBuf) + 'static>(&self, f: F) {
        *self.path_navigate_cb.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_stats_changed<F: Fn(SelectionStats) + 'static>(&self, f: F) {
        *self.stats_changed_cb.borrow_mut() = Some(Box::new(f));
    }

    pub fn get_stats(&self) -> SelectionStats {
        let displayed = self.displayed_entries.borrow();
        let total_count = displayed.len();
        let total_size = displayed.iter().map(|e| e.size).sum();

        let selected_rows = self.list_box.selected_rows();
        let selected_count = selected_rows.len();
        let selected_size = selected_rows
            .iter()
            .filter_map(|r| displayed.get(r.index() as usize))
            .map(|e| e.size)
            .sum();

        SelectionStats {
            total_count,
            total_size,
            selected_count,
            selected_size,
        }
    }

    pub fn notify_stats_changed(&self) {
        if let Some(ref cb) = *self.stats_changed_cb.borrow() {
            cb(self.get_stats());
        }
    }

    pub fn update_folder_path(&self, folder: &Path) {
        while let Some(child) = self.breadcrumb_box.first_child() {
            self.breadcrumb_box.remove(&child);
        }

        let is_root = folder.as_os_str().is_empty();

        let root_btn = Button::builder()
            .label("/")
            .tooltip_text("Root Directory (/)")
            .build();
        root_btn.add_css_class("flat");
        root_btn.add_css_class("title-4");
        if is_root {
            root_btn.add_css_class("heading");
        }

        let nav_cb = self.path_navigate_cb.clone();
        root_btn.connect_clicked(move |_| {
            if let Some(ref cb) = *nav_cb.borrow() {
                cb(PathBuf::new());
            }
        });
        self.breadcrumb_box.append(&root_btn);

        if !is_root {
            let mut accum = PathBuf::new();
            let components: Vec<_> = folder.components().collect();
            let count = components.len();

            for (idx, comp) in components.into_iter().enumerate() {
                let name = comp.as_os_str().to_string_lossy().to_string();
                accum.push(&name);

                let sep = Label::new(Some("/"));
                sep.add_css_class("dim-label");
                self.breadcrumb_box.append(&sep);

                let btn = Button::builder()
                    .label(&name)
                    .tooltip_text(format!("/{}", accum.display()))
                    .build();
                btn.add_css_class("flat");

                let is_last = idx == count - 1;
                if is_last {
                    btn.add_css_class("heading");
                }

                let nav_target = accum.clone();
                let nav_cb_comp = self.path_navigate_cb.clone();
                btn.connect_clicked(move |_| {
                    if let Some(ref cb) = *nav_cb_comp.borrow() {
                        cb(nav_target.clone());
                    }
                });
                self.breadcrumb_box.append(&btn);
            }
        }

        self.up_button.set_sensitive(!is_root);

        let scroll = self.breadcrumb_scroll.clone();
        glib::idle_add_local_once(move || {
            let adj = scroll.hadjustment();
            adj.set_value(adj.upper());
        });
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
        self.notify_stats_changed();
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
