use gio::Menu;

pub fn build_context_menu() -> Menu {
    let menu_model = Menu::new();

    let open_section = Menu::new();
    open_section.append(Some("Open / Enter"), Some("win.open_entry"));
    open_section.append(Some("Quick Preview (Space)"), Some("win.quick_preview"));
    open_section.append(Some("Export Selected... (Ctrl+E)"), Some("win.export_selected"));
    open_section.append(Some("Extract Selected..."), Some("win.extract_selected"));
    open_section.append(Some("Extract Here"), Some("win.extract_here"));
    menu_model.append_section(None, &open_section);

    let edit_section = Menu::new();
    edit_section.append(Some("Add Files..."), Some("win.add_files"));
    edit_section.append(Some("Delete Selected"), Some("win.delete_selected"));
    menu_model.append_section(None, &edit_section);

    let select_section = Menu::new();
    select_section.append(Some("Select All"), Some("win.select_all"));
    select_section.append(Some("Deselect All"), Some("win.deselect_all"));
    select_section.append(Some("Properties..."), Some("win.entry_properties"));
    menu_model.append_section(None, &select_section);

    menu_model
}
