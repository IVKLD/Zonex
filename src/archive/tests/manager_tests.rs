use crate::archive::ArchiveManager;
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn test_zip_add_entries_fast() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("archive.zip");
    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");

    fs::write(&file1, "Content 1").unwrap();
    fs::write(&file2, "Content 2").unwrap();

    let mut mgr = ArchiveManager::new();
    mgr.create_archive(&zip_path, std::slice::from_ref(&file1))
        .unwrap();
    assert_eq!(mgr.visible_entries().len(), 1);

    mgr.add_files(std::slice::from_ref(&file2)).unwrap();
    assert_eq!(mgr.visible_entries().len(), 2);
}

#[test]
fn test_selective_drag_out_extraction() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("nested.zip");

    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("nested/dir/target.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"Target File Data").unwrap();
        zip.finish().unwrap();
    }

    let mut mgr = ArchiveManager::new();
    mgr.open_archive(&zip_path).unwrap();

    let stage_dir = dir.path().join("stage");
    let target_path = std::path::PathBuf::from("nested/dir/target.txt");

    let extracted = mgr
        .extract_selected(std::slice::from_ref(&target_path), &stage_dir)
        .unwrap();

    assert!(!extracted.is_empty());
    let expected_extracted_file = stage_dir.join("target.txt");
    assert!(expected_extracted_file.exists());
    assert_eq!(
        fs::read_to_string(&expected_extracted_file).unwrap(),
        "Target File Data"
    );
}

#[test]
fn test_manager_navigation_and_visible_entries() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("hierarchy.zip");

    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("root_file.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"Root").unwrap();

        zip.start_file("folder/sub_file.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"Sub").unwrap();

        zip.finish().unwrap();
    }

    let mut mgr = ArchiveManager::new();
    mgr.open_archive(&zip_path).unwrap();

    let root_visible = mgr.visible_entries();
    let root_names: Vec<String> = root_visible.iter().map(|e| e.file_name()).collect();
    assert!(root_names.contains(&"folder".to_string()));
    assert!(root_names.contains(&"root_file.txt".to_string()));

    mgr.enter_folder(std::path::PathBuf::from("folder"));
    let sub_visible = mgr.visible_entries();
    let sub_names: Vec<String> = sub_visible.iter().map(|e| e.file_name()).collect();
    assert_eq!(sub_names, vec!["sub_file.txt"]);

    assert!(mgr.navigate_back());
    assert_eq!(mgr.current_folder(), std::path::Path::new(""));
}

#[test]
fn test_add_archive_into_open_archive() {
    let dir = tempdir().unwrap();
    let parent_zip = dir.path().join("parent.zip");
    let child_zip = dir.path().join("child.zip");

    {
        let file = File::create(&parent_zip).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("existing.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"Parent content").unwrap();
        zip.finish().unwrap();
    }

    {
        let file = File::create(&child_zip).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("child_data.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"Child content").unwrap();
        zip.finish().unwrap();
    }

    let mut mgr = ArchiveManager::new();
    mgr.open_archive(&parent_zip).unwrap();
    assert_eq!(mgr.visible_entries().len(), 1);

    mgr.add_files(std::slice::from_ref(&child_zip)).unwrap();
    assert_eq!(mgr.visible_entries().len(), 2);

    let names: Vec<String> = mgr
        .visible_entries()
        .iter()
        .map(|e| e.file_name())
        .collect();
    assert!(names.contains(&"child.zip".to_string()));
}
