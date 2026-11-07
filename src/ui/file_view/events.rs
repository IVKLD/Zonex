use super::builder::FileViewElements;
use super::render::render_entries;
use crate::archive::ArchiveEntry;
use gdk4::{Rectangle, BUTTON_SECONDARY};
use gtk4::prelude::*;
use gtk4::GestureClick;
use std::cell::RefCell;
use std::rc::Rc;

pub fn setup_events(
    el: &FileViewElements,
    all_entries: &Rc<RefCell<Vec<ArchiveEntry>>>,
    displayed_entries: &Rc<RefCell<Vec<ArchiveEntry>>>,
    anchor_index: &Rc<RefCell<Option<usize>>>,
) {
    let click_gesture = GestureClick::new();
    click_gesture.set_button(0);

    let list_box_clone = el.list_box.clone();
    let popover_clone = el.popover_menu.clone();
    let up_btn_clone = el.up_button.clone();
    let forward_btn_clone = el.forward_button.clone();

    click_gesture.connect_pressed(move |gesture, _n_press, x, y| {
        let button = gesture.current_button();

        if (button == 8 || button == 6) && up_btn_clone.is_sensitive() {
            up_btn_clone.emit_clicked();
            return;
        }

        if (button == 9 || button == 7) && forward_btn_clone.is_sensitive() {
            forward_btn_clone.emit_clicked();
            return;
        }

        if button == BUTTON_SECONDARY {
            if let Some(row) = list_box_clone.row_at_y(y as i32) {
                if !row.is_selected() {
                    list_box_clone.unselect_all();
                    list_box_clone.select_row(Some(&row));
                }
            }
            popover_clone.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
            popover_clone.popup();
        }
    });

    el.list_box.add_controller(click_gesture);

    let search_clone = el.search_entry.clone();
    let sort_clone = el.sort_dropdown.clone();
    let all_e_clone = all_entries.clone();
    let disp_e_clone = displayed_entries.clone();
    let list_box_render = el.list_box.clone();
    let anchor_render = anchor_index.clone();

    let reapply = move || {
        render_entries(
            &search_clone,
            &sort_clone,
            &all_e_clone,
            &disp_e_clone,
            &list_box_render,
            &anchor_render,
        );
    };

    let reapply_rc = Rc::new(reapply);

    let r1 = reapply_rc.clone();
    el.search_entry.connect_search_changed(move |_| {
        r1();
    });

    let r2 = reapply_rc;
    el.sort_dropdown.connect_selected_notify(move |_| {
        r2();
    });

    let anchor_c = anchor_index.clone();
    let list_box_c = el.list_box.clone();
    el.list_box.connect_row_selected(move |_, row| {
        if let Some(r) = row {
            *anchor_c.borrow_mut() = Some(r.index() as usize);
        } else if list_box_c.selected_rows().is_empty() {
            *anchor_c.borrow_mut() = None;
        }
    });
}
