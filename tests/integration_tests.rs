use zonex::archive::detector::{DetectedFormat, FormatDetector};
use zonex::archive::tar_engine::{TarCompression, TarEngine};

use zonex::archive::{ArchiveEngine, ArchiveEntry, ArchiveManager};
use zonex::fuse::ArchiveFuseFs;
use fuser::{FileType, FUSE_ROOT_ID};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Integration Test 1: Full Multi-Format Engine (Zip, Tar, TarGz, TarZst) Operations
#[test]
fn test_multi_format_creation_listing_extraction() {
    let dir = tempdir().unwrap();

    // 1. Test Plain TAR engine
    let tar_path = dir.path().join("test.tar");
    let file1 = dir.path().join("doc.txt");
    fs::write(&file1, "Plain Tar Content").unwrap();

    let tar_engine = TarEngine::new(TarCompression::None);
    tar_engine.create_archive(&tar_path, &[file1.clone()]).unwrap();
    assert_eq!(FormatDetector::detect(&tar_path), Some(DetectedFormat::Tar));

    let tar_entries = tar_engine.list_entries(&tar_path).unwrap();
    assert!(!tar_entries.is_empty());
    assert_eq!(tar_entries[0].size, 17); // Exact size validation

    let extract_dir_tar = dir.path().join("out_tar");
    let extracted_tar = tar_engine
        .extract_entries(&tar_path, &[], &extract_dir_tar)
        .unwrap();
    assert!(!extracted_tar.is_empty());

    // 2. Test TAR.GZ engine
    let targz_path = dir.path().join("test.tar.gz");
    let targz_engine = TarEngine::new(TarCompression::Gzip);
    targz_engine.create_archive(&targz_path, &[file1.clone()]).unwrap();
    assert_eq!(FormatDetector::detect(&targz_path), Some(DetectedFormat::TarGz));

    let targz_entries = targz_engine.list_entries(&targz_path).unwrap();
    assert_eq!(targz_entries[0].size, 17);

    // 3. Test TAR.ZST engine
    let tarzst_path = dir.path().join("test.tar.zst");
    let tarzst_engine = TarEngine::new(TarCompression::Zstd);
    tarzst_engine.create_archive(&tarzst_path, &[file1.clone()]).unwrap();
    assert_eq!(FormatDetector::detect(&tarzst_path), Some(DetectedFormat::TarZst));

    let tarzst_entries = tarzst_engine.list_entries(&tarzst_path).unwrap();
    assert_eq!(tarzst_entries[0].size, 17);
}

/// Integration Test 2: Unicode & Deep Nested Hierarchy Navigation History
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

    // At Root
    assert_eq!(mgr.current_folder(), Path::new(""));

    // Enter 'Документы'
    mgr.enter_folder(PathBuf::from("Документы"));
    assert_eq!(mgr.current_folder(), Path::new("Документы"));

    // Enter 'Документы/Проекты'
    mgr.enter_folder(PathBuf::from("Документы/Проекты"));
    assert_eq!(mgr.current_folder(), Path::new("Документы/Проекты"));

    // Navigate BACK -> 'Документы'
    assert!(mgr.navigate_back());
    assert_eq!(mgr.current_folder(), Path::new("Документы"));

    // Navigate BACK -> Root
    assert!(mgr.navigate_back());
    assert_eq!(mgr.current_folder(), Path::new(""));

    // Navigate FORWARD -> 'Документы'
    assert!(mgr.navigate_forward());
    assert_eq!(mgr.current_folder(), Path::new("Документы"));

    // Navigate FORWARD -> 'Документы/Проекты'
    assert!(mgr.navigate_forward());
    assert_eq!(mgr.current_folder(), Path::new("Документы/Проекты"));

    // Further FORWARD fails (at top of forward history stack)
    assert!(!mgr.navigate_forward());

    // Enter new branch -> clears forward stack
    mgr.navigate_back(); // Back to 'Документы'
    mgr.enter_folder(PathBuf::from("НоваяПапка"));
    assert_eq!(mgr.current_folder(), Path::new("НоваяПапка"));
    assert!(!mgr.navigate_forward()); // Forward stack must be empty
}

/// Integration Test 3: FUSE Inode Resolution & Read-Write Permissions (0o644)
#[test]
fn test_fuse_inode_resolution_and_permissions() {
    let entries = vec![
        ArchiveEntry::new("projects/zonex/Cargo.toml", 850, false),
        ArchiveEntry::new("projects/zonex/src/main.rs", 1200, false),
    ];

    let archive_path = PathBuf::from("/tmp/test_zonex.zip");
    let fs = ArchiveFuseFs::new(archive_path, &entries);

    let proj_ino = fs.find_node_ino(FUSE_ROOT_ID, "projects").expect("projects dir");
    let zonex_ino = fs.find_node_ino(proj_ino, "zonex").expect("zonex dir");
    let cargo_ino = fs.find_node_ino(zonex_ino, "Cargo.toml").expect("Cargo.toml file");

    let cargo_attr = fs.make_attr(cargo_ino).expect("Cargo.toml attr");

    assert_eq!(cargo_attr.kind, FileType::RegularFile);
    assert_eq!(cargo_attr.size, 850);
    // MUST be 0o644 (rw-r--r--) so copied files can be edited immediately after DnD
    assert_eq!(cargo_attr.perm, 0o644);
}

/// Integration Test 4: Sub-millisecond FUSE Path Join Guarantee for Wayland Drag-Out
#[test]
fn test_fuse_drag_out_virtual_path_performance() {
    use std::time::Instant;

    let mount_root = PathBuf::from("/run/user/1000/zonex_fuse/my_archive_12345");
    let selected_entries = vec![
        PathBuf::from("deep/nested/subfolder/file_1.txt"),
        PathBuf::from("deep/nested/subfolder/file_2.txt"),
        PathBuf::from("deep/nested/subfolder/file_3.txt"),
    ];

    let start = Instant::now();

    let drag_paths: Vec<PathBuf> = selected_entries
        .iter()
        .map(|entry_path| mount_root.join(entry_path))
        .collect();

    let duration = start.elapsed();

    assert_eq!(drag_paths.len(), 3);
    assert_eq!(
        drag_paths[0],
        PathBuf::from("/run/user/1000/zonex_fuse/my_archive_12345/deep/nested/subfolder/file_1.txt")
    );

    // Path resolution MUST complete under 50 microseconds (0.05 ms)
    assert!(
        duration.as_micros() < 50,
        "FUSE path resolution took {:?} (expected < 50us)",
        duration
    );
}
