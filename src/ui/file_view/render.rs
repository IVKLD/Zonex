use super::filter_sort::filter_and_sort;
use super::icons::get_icon_name_for_entry;
use crate::archive::ArchiveEntry;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, DropDown, Image, Label, ListBox, ListBoxRow, Orientation, SearchEntry};
use std::cell::RefCell;
use std::rc::Rc;

pub fn build_row(entry: &ArchiveEntry) -> ListBoxRow {
    let row = ListBoxRow::new();
    let hbox = GtkBox::new(Orientation::Horizontal, 12);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(8);
    hbox.set_margin_bottom(8);

    let icon_name = get_icon_name_for_entry(entry);
    let icon = Image::from_icon_name(icon_name);

    let fname = entry.file_name();
    let name_label = Label::new(Some(&fname));
    name_label.set_hexpand(true);
    name_label.set_xalign(0.0);
    name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name_label.set_width_chars(1);
    name_label.set_max_width_chars(1);
    name_label.set_tooltip_text(Some(&fname));

    let size_label = Label::new(Some(&entry.formatted_size()));
    size_label.add_css_class("dim-label");

    hbox.append(&icon);
    hbox.append(&name_label);
    hbox.append(&size_label);

    row.set_child(Some(&hbox));
    row
}

pub fn render_entries(
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

    let filtered = filter_and_sort(&all, &query, sort_idx);

    for i in (0..displayed_entries.borrow().len()).rev() {
        if let Some(row) = list_box.row_at_index(i as i32) {
            list_box.remove(&row);
        }
    }
    while let Some(row) = list_box.row_at_index(0) {
        list_box.remove(&row);
    }

    *anchor_index.borrow_mut() = None;
    let mut displayed = displayed_entries.borrow_mut();
    *displayed = filtered;

    for entry in displayed.iter() {
        let row = build_row(entry);
        list_box.append(&row);
    }
}
