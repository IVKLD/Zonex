use super::builder::FileViewElements;
use super::render::render_entries;
use crate::archive::ArchiveEntry;
use gdk4::{Rectangle, BUTTON_PRIMARY, BUTTON_SECONDARY};
use gtk4::prelude::*;
use gtk4::GestureClick;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub fn setup_events(
    el: &FileViewElements,
    all_entries: &Rc<RefCell<Vec<ArchiveEntry>>>,
    displayed_entries: &Rc<RefCell<Vec<ArchiveEntry>>>,
    anchor_index: &Rc<RefCell<Option<usize>>>,
    notify_stats: Rc<dyn Fn()>,
) {
    let click_gesture = GestureClick::new();
    click_gesture.set_button(0);

    let list_box_clone = el.list_box.clone();
    let popover_clone = el.popover_menu.clone();
    let up_btn_clone = el.up_button.clone();
    let forward_btn_clone = el.forward_button.clone();
    let anchor_click = anchor_index.clone();

    click_gesture.connect_pressed(move |gesture, n_press, x, y| {
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
                    *anchor_click.borrow_mut() = Some(row.index() as usize);
                }
            }
            popover_clone.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
            popover_clone.popup();
            return;
        }

        if button == BUTTON_PRIMARY && n_press == 1 {
            let state = gesture.current_event_state();
            let has_shift = state.contains(gdk4::ModifierType::SHIFT_MASK);
            let has_ctrl = state.contains(gdk4::ModifierType::CONTROL_MASK);

            if let Some(target_row) = list_box_clone.row_at_y(y as i32) {
                let target_idx = target_row.index() as usize;

                if has_shift {
                    let anchor = anchor_click.borrow().unwrap_or(target_idx);
                    let start = anchor.min(target_idx);
                    let end = anchor.max(target_idx);

                    if !has_ctrl {
                        list_box_clone.unselect_all();
                    }

                    for i in start..=end {
                        if let Some(r) = list_box_clone.row_at_index(i as i32) {
                            list_box_clone.select_row(Some(&r));
                        }
                    }
                } else if !has_ctrl && !target_row.is_selected() {
                    list_box_clone.unselect_all();
                    list_box_clone.select_row(Some(&target_row));
                    *anchor_click.borrow_mut() = Some(target_idx);
                }
            }
        }
    });

    el.list_box.add_controller(click_gesture);

    let search_clone = el.search_entry.clone();
    let sort_clone = el.sort_dropdown.clone();
    let all_e_clone = all_entries.clone();
    let disp_e_clone = displayed_entries.clone();
    let list_box_render = el.list_box.clone();
    let anchor_render = anchor_index.clone();
    let stats_render = notify_stats.clone();

    let reapply = move || {
        render_entries(
            &search_clone,
            &sort_clone,
            &all_e_clone,
            &disp_e_clone,
            &list_box_render,
            &anchor_render,
        );
        stats_render();
    };

    let reapply_rc = Rc::new(reapply);

    let debounce_source: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let debounce_c = debounce_source.clone();
    let r_search = reapply_rc.clone();
    let search_entry_for_change = el.search_entry.clone();

    el.search_entry.connect_search_changed(move |_| {
        if let Some(src) = debounce_c.borrow_mut().take() {
            src.remove();
        }
        if search_entry_for_change.text().is_empty() {
            r_search();
            return;
        }
        let r_inner = r_search.clone();
        let deb_inner = debounce_c.clone();
        let src = glib::timeout_add_local_once(Duration::from_millis(150), move || {
            *deb_inner.borrow_mut() = None;
            r_inner();
        });
        *debounce_c.borrow_mut() = Some(src);
    });

    let debounce_act = debounce_source;
    let r_act = reapply_rc.clone();
    el.search_entry.connect_activate(move |_| {
        if let Some(src) = debounce_act.borrow_mut().take() {
            src.remove();
        }
        r_act();
    });

    let r_sort = reapply_rc;
    el.sort_dropdown.connect_selected_notify(move |_| {
        r_sort();
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

    let stats_sel = notify_stats;
    el.list_box.connect_selected_rows_changed(move |_| {
        stats_sel();
    });
}
