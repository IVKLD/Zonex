#[cfg(test)]
mod tests {
    use crate::archive::ArchiveEntry;
    use crate::fuse::archive_fs::ArchiveFuseFs;
    use fuser::FileType;
    use std::path::PathBuf;

    const FUSE_ROOT_ID: u64 = 1;

    #[test]
    fn test_fuse_tree_construction_and_permissions() {
        let entries = vec![
            ArchiveEntry::new("docs/readme.md", 1024, false),
            ArchiveEntry::new("docs/images/logo.png", 2048, false),
            ArchiveEntry::new("src/main.rs", 512, false),
            ArchiveEntry::new("empty_dir", 0, true),
        ];

        let archive_path = PathBuf::from("/tmp/dummy_archive.zip");
        let fs = ArchiveFuseFs::new(archive_path, &entries);

        // Root directory permissions check (0o755)
        let root_attr = fs.make_attr(FUSE_ROOT_ID).expect("Root node must exist");
        assert_eq!(root_attr.kind, FileType::Directory);
        assert_eq!(root_attr.perm, 0o755);

        // Verify nested directory creation
        let docs_ino = fs.find_node_ino(FUSE_ROOT_ID, "docs");
        assert!(docs_ino.is_some(), "'docs' directory inode must exist");
        let docs_attr = fs.make_attr(docs_ino.unwrap()).unwrap();
        assert_eq!(docs_attr.kind, FileType::Directory);
        assert_eq!(docs_attr.perm, 0o755);

        // Verify leaf file permissions (0o644 - Read-Write for owner so copied files can be edited!)
        let readme_ino = fs.find_node_ino(docs_ino.unwrap(), "readme.md");
        assert!(readme_ino.is_some(), "'readme.md' inode must exist inside 'docs'");
        let readme_attr = fs.make_attr(readme_ino.unwrap()).unwrap();
        assert_eq!(readme_attr.kind, FileType::RegularFile);
        assert_eq!(readme_attr.size, 1024);
        assert_eq!(
            readme_attr.perm, 0o644,
            "File permissions must be 0o644 (rw-r--r--) so extracted files are editable"
        );
    }

    #[test]
    fn test_fuse_node_lookup() {
        let entries = vec![
            ArchiveEntry::new("src/lib.rs", 256, false),
        ];

        let archive_path = PathBuf::from("/tmp/dummy.zip");
        let fs = ArchiveFuseFs::new(archive_path, &entries);

        let src_ino = fs.find_node_ino(FUSE_ROOT_ID, "src").expect("src dir should exist");
        let lib_ino = fs.find_node_ino(src_ino, "lib.rs").expect("lib.rs should exist");

        assert!(fs.make_attr(lib_ino).is_some());
        assert!(fs.find_node_ino(src_ino, "non_existent.rs").is_none());
    }
}
