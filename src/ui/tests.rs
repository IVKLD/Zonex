use crate::archive::entry::format_bytes;
use crate::archive::ArchiveEntry;
use crate::ui::file_view::{get_icon_name_for_entry, FileView};

#[test]
fn test_mimetype_icon_resolution() {
    let folder = ArchiveEntry::new("my_folder", 0, true);
    assert_eq!(get_icon_name_for_entry(&folder), "folder-symbolic");

    let zip = ArchiveEntry::new("arch.zip", 100, false);
    assert_eq!(get_icon_name_for_entry(&zip), "package-x-generic-symbolic");

    let img = ArchiveEntry::new("photo.png", 500, false);
    assert_eq!(get_icon_name_for_entry(&img), "image-x-generic-symbolic");

    let code = ArchiveEntry::new("main.rs", 200, false);
    assert_eq!(get_icon_name_for_entry(&code), "text-x-script-symbolic");

    let video = ArchiveEntry::new("movie.mp4", 10000, false);
    assert_eq!(get_icon_name_for_entry(&video), "video-x-generic-symbolic");

    let audio = ArchiveEntry::new("song.mp3", 3000, false);
    assert_eq!(get_icon_name_for_entry(&audio), "audio-x-generic-symbolic");

    let exec = ArchiveEntry::new("runner.sh", 400, false);
    assert_eq!(get_icon_name_for_entry(&exec), "text-x-script-symbolic");

    let binary = ArchiveEntry::new("app.AppImage", 4000, false);
    assert_eq!(
        get_icon_name_for_entry(&binary),
        "application-x-executable-symbolic"
    );
}

#[test]
fn test_byte_formatting() {
    assert_eq!(format_bytes(500), "500 B");
    assert_eq!(format_bytes(1024), "1.00 KB");
    assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    assert_eq!(format_bytes(2500 * 1024 * 1024), "2.44 GB");
}

#[test]
fn test_gtk_headless_file_view_component() {
    if gtk4::init().is_err() {
        eprintln!("GTK init skipped (headless env without display)");
        return;
    }

    let file_view = FileView::new();

    let entries = vec![
        ArchiveEntry::new("zebra.txt", 100, false),
        ArchiveEntry::new("apple.txt", 500, false),
        ArchiveEntry::new("folder_a", 0, true),
    ];

    file_view.set_entries(entries);

    let first = file_view.get_entry_at_index(0).unwrap();
    assert_eq!(first.file_name(), "folder_a");
    assert!(first.is_dir);

    let second = file_view.get_entry_at_index(1).unwrap();
    assert_eq!(second.file_name(), "apple.txt");

    let third = file_view.get_entry_at_index(2).unwrap();
    assert_eq!(third.file_name(), "zebra.txt");

    file_view.select_all();
    let selected = file_view.get_selected_entry_paths();
    assert_eq!(selected.len(), 3);

    file_view.deselect_all();
    let deselected = file_view.get_selected_entry_paths();
    assert_eq!(deselected.len(), 0);
}
