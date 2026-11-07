use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;
use zonex::archive::ArchiveManager;

#[test]
fn test_deep_navigation_history_back_forward_unicode() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("unicode_nested.zip");

    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);

        zip.start_file("Документы/Проекты/отчет.pdf", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"PDF DATA").unwrap();

        zip.start_file("Документы/фото.jpg", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"JPG DATA").unwrap();

        zip.finish().unwrap();
    }

    let mut mgr = ArchiveManager::new();
    mgr.open_archive(&zip_path).unwrap();

    assert_eq!(mgr.current_folder(), Path::new(""));

    mgr.enter_folder(PathBuf::from("Документы"));
    assert_eq!(mgr.current_folder(), Path::new("Документы"));

    mgr.enter_folder(PathBuf::from("Документы/Проекты"));
    assert_eq!(mgr.current_folder(), Path::new("Документы/Проекты"));

    assert!(mgr.navigate_back());
    assert_eq!(mgr.current_folder(), Path::new("Документы"));

    assert!(mgr.navigate_back());
    assert_eq!(mgr.current_folder(), Path::new(""));

    assert!(mgr.navigate_forward());
    assert_eq!(mgr.current_folder(), Path::new("Документы"));

    assert!(mgr.navigate_forward());
    assert_eq!(mgr.current_folder(), Path::new("Документы/Проекты"));

    assert!(!mgr.navigate_forward());

    mgr.navigate_back();
    mgr.enter_folder(PathBuf::from("НоваяПапка"));
    assert_eq!(mgr.current_folder(), Path::new("НоваяПапка"));
    assert!(!mgr.navigate_forward());
}

#[test]
fn test_archive_deletion_and_integrity_check() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("deletion_test.zip");

    let file1 = dir.path().join("keep.txt");
    let file2 = dir.path().join("remove.txt");
    fs::write(&file1, "Keep this file").unwrap();
    fs::write(&file2, "Remove this file").unwrap();

    let mut mgr = ArchiveManager::new();
    mgr.create_archive(&zip_path, &[file1, file2]).unwrap();

    let initial_entries = mgr.visible_entries();
    assert_eq!(initial_entries.len(), 2);

    mgr.delete_selected_entries(&[PathBuf::from("remove.txt")])
        .unwrap();

    let remaining_entries = mgr.visible_entries();
    assert_eq!(remaining_entries.len(), 1);
    assert_eq!(remaining_entries[0].path, Path::new("keep.txt"));

    let report = mgr.verify_integrity(None).unwrap();
    assert_eq!(report.total_files, 1);
    assert_eq!(report.ok_files, 1);
    assert!(report.failed_files.is_empty());
}

#[test]
fn test_nested_deletion_preserves_current_folder() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("nested_del.zip");

    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);

        zip.start_file("folder/file1.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"1").unwrap();
        zip.start_file("folder/file2.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"2").unwrap();
        zip.finish().unwrap();
    }

    let mut mgr = ArchiveManager::new();
    mgr.open_archive(&zip_path).unwrap();
    mgr.enter_folder(PathBuf::from("folder"));
    assert_eq!(mgr.current_folder(), Path::new("folder"));

    mgr.delete_selected_entries(&[PathBuf::from("folder/file1.txt")])
        .unwrap();

    assert_eq!(mgr.current_folder(), Path::new("folder"));
    let visible = mgr.visible_entries();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].path, Path::new("folder/file2.txt"));
}

#[test]
fn test_large_hierarchy_navigation_cache() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("large_tree.zip");

    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);

        for i in 0..500 {
            let path_str = format!("sub_{}/file_{}.txt", i % 10, i);
            zip.start_file(path_str, SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"sample data").unwrap();
        }
        zip.finish().unwrap();
    }

    let mut mgr = ArchiveManager::new();
    mgr.open_archive(&zip_path).unwrap();

    let root = mgr.visible_entries();
    assert_eq!(root.len(), 10);

    // Enter subfolder and test cached navigation
    mgr.enter_folder(PathBuf::from("sub_0"));
    let sub = mgr.visible_entries();
    assert_eq!(sub.len(), 50);

    // Second call must return instantly from cache
    let sub_cached = mgr.visible_entries();
    assert_eq!(sub_cached.len(), 50);

    assert!(mgr.navigate_back());
    let root_cached = mgr.visible_entries();
    assert_eq!(root_cached.len(), 10);
}
