use crate::ui::file_view::FileView;
use gdk4::{Key, ModifierType};
use glib::Propagation;
use gtk4::prelude::*;
use gtk4::{Button, EventControllerKey};
use std::rc::Rc;

fn trigger_button(btn: &Button) -> Propagation {
    if btn.is_sensitive() {
        btn.emit_clicked();
        Propagation::Stop
    } else {
        Propagation::Proceed
    }
}

pub fn setup_shortcuts(file_view: &Rc<FileView>, open_btn: &Button) -> EventControllerKey {
    let key_controller = EventControllerKey::new();
    let file_view_key = file_view.clone();
    let up_btn_key = file_view.up_button().clone();
    let fw_btn_key = file_view.forward_button().clone();
    let open_btn_key = open_btn.clone();
    let list_box_key = file_view.list_box().clone();

    key_controller.connect_key_pressed(move |_, key, _keycode, state| {
        let is_ctrl = state.contains(ModifierType::CONTROL_MASK);
        let is_shift = state.contains(ModifierType::SHIFT_MASK);
        let is_alt = state.contains(ModifierType::ALT_MASK);

        match key {
            Key::BackSpace | Key::Back => return trigger_button(&up_btn_key),
            Key::Left if is_alt => return trigger_button(&up_btn_key),
            Key::Up if is_alt => return trigger_button(&up_btn_key),
            Key::Right if is_alt => return trigger_button(&fw_btn_key),
            Key::o | Key::O if is_ctrl => {
                open_btn_key.emit_clicked();
                return Propagation::Stop;
            }
            Key::n | Key::N if is_ctrl => {
                if is_shift {
                    let _ = file_view_key
                        .widget()
                        .activate_action("win.new_folder", None);
                } else {
                    let _ = file_view_key
                        .widget()
                        .activate_action("win.new_archive", None);
                }
                return Propagation::Stop;
            }
            Key::f | Key::F if is_ctrl => {
                file_view_key.search_entry().grab_focus();
                return Propagation::Stop;
            }
            Key::F5 | Key::r | Key::R if is_ctrl => {
                file_view_key.reapply_view();
                return Propagation::Stop;
            }
            Key::e | Key::E if is_ctrl => {
                let _ = file_view_key
                    .widget()
                    .activate_action("win.export_selected", None);
                return Propagation::Stop;
            }
            Key::a | Key::A if is_ctrl => {
                file_view_key.select_all();
                return Propagation::Stop;
            }
            Key::Escape => {
                if !file_view_key.search_entry().text().is_empty() {
                    file_view_key.search_entry().set_text("");
                    file_view_key.list_box().grab_focus();
                    return Propagation::Stop;
                }
                file_view_key.deselect_all();
                return Propagation::Stop;
            }
            Key::Return | Key::KP_Enter if is_alt => {
                let _ = file_view_key
                    .widget()
                    .activate_action("win.entry_properties", None);
                return Propagation::Stop;
            }
            Key::Return | Key::KP_Enter => {
                if let Some(row) = list_box_key.selected_rows().first() {
                    row.activate();
                    return Propagation::Stop;
                }
            }
            Key::space => {
                let _ = file_view_key
                    .widget()
                    .activate_action("win.quick_preview", None);
                return Propagation::Stop;
            }
            Key::Delete | Key::KP_Delete => {
                let _ = file_view_key
                    .widget()
                    .activate_action("win.delete_selected", None);
                return Propagation::Stop;
            }
            _ => {}
        }

        Propagation::Proceed
    });

    key_controller
}
