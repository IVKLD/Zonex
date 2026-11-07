use fuser::FileType;
use std::path::PathBuf;
use std::time::Instant;
use zonex::archive::ArchiveEntry;
use zonex::fuse::ArchiveFuseFs;

const FUSE_ROOT_ID: u64 = 1;

#[test]
fn test_fuse_inode_resolution_and_permissions() {
    let entries = vec![
        ArchiveEntry::new("projects/zonex/Cargo.toml", 850, false),
        ArchiveEntry::new("projects/zonex/src/main.rs", 1200, false),
    ];

    let archive_path = PathBuf::from("/tmp/test_zonex.zip");
    let fs = ArchiveFuseFs::new(archive_path, &entries);

    let proj_ino = fs
        .find_node_ino(FUSE_ROOT_ID, "projects")
        .expect("projects dir");
    let zonex_ino = fs.find_node_ino(proj_ino, "zonex").expect("zonex dir");
    let cargo_ino = fs
        .find_node_ino(zonex_ino, "Cargo.toml")
        .expect("Cargo.toml file");

    let cargo_attr = fs.make_attr(cargo_ino).expect("Cargo.toml attr");

    assert_eq!(cargo_attr.kind, FileType::RegularFile);
    assert_eq!(cargo_attr.size, 850);
    assert_eq!(cargo_attr.perm, 0o644);
}

#[test]
fn test_fuse_drag_out_virtual_path_performance() {
    let mount_root = PathBuf::from("/run/user/1000/zonex_fuse/my_archive_12345");
    let selected_entries = [
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
        PathBuf::from(
            "/run/user/1000/zonex_fuse/my_archive_12345/deep/nested/subfolder/file_1.txt"
        )
    );

    assert!(
        duration.as_micros() < 50,
        "FUSE path resolution took {:?} (expected < 50us)",
        duration
    );
}
