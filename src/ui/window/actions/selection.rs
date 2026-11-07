use super::context::ActionsContext;
use gio::SimpleAction;
use gtk4::prelude::*;

pub fn register(ctx: &ActionsContext) {
    let fv_select = ctx.file_view.clone();
    let action_select_all = SimpleAction::new("select_all", None);
    action_select_all.connect_activate(move |_, _| {
        fv_select.select_all();
    });
    ctx.window.add_action(&action_select_all);

    let fv_deselect = ctx.file_view.clone();
    let action_deselect_all = SimpleAction::new("deselect_all", None);
    action_deselect_all.connect_activate(move |_, _| {
        fv_deselect.deselect_all();
    });
    ctx.window.add_action(&action_deselect_all);
}
