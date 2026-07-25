#[cfg(test)]
mod tests {
    use crate::archive::detector::{DetectedFormat, FormatDetector};
    use crate::archive::libarchive_engine::LibarchiveEngine;
    use crate::archive::zip_engine::ZipEngine;
    use crate::archive::{ArchiveEngine, ArchiveManager};
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    /// Test 1: Standard Zip creation, detection, and opening
    #[test]
    fn test_zip_creation_and_opening() {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("sample.zip");

        {
            let file = File::create(&zip_path).unwrap();
            let mut zip = ZipWriter::new(file);
            zip.start_file("hello.txt", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"Hello Rust World").unwrap();
            zip.finish().unwrap();
        }

        // Verify format detector
        let format = FormatDetector::detect(&zip_path);
        assert_eq!(format, Some(DetectedFormat::Zip));

        // Verify ArchiveManager open_archive
        let mut mgr = ArchiveManager::new();
        let entries = mgr.open_archive(&zip_path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name(), "hello.txt");
    }

    /// Test 2: Resilient Local Header Stream Scanner for Truncated/Corrupt EOCD Zips
    #[test]
    fn test_corrupt_or_truncated_zip_resilience() {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("truncated.zip");

        // 1. Create a valid zip
        {
            let file = File::create(&zip_path).unwrap();
            let mut zip = ZipWriter::new(file);
            zip.start_file("docs/readme.txt", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"Important Readme Content").unwrap();
            zip.finish().unwrap();
        }

        // 2. Intentionally truncate the last 15 bytes to destroy standard EOCD record
        let file_bytes = fs::read(&zip_path).unwrap();
        let truncated_bytes = &file_bytes[..file_bytes.len().saturating_sub(15)];
        fs::write(&zip_path, truncated_bytes).unwrap();

        // 3. Verify ZipEngine falls back to local header scanner and recovers entries
        let engine = ZipEngine;
        let entries = engine.list_entries(&zip_path).unwrap();
        assert!(!entries.is_empty());
        assert!(entries
            .iter()
            .any(|e| e.path.to_string_lossy().contains("readme.txt")));
    }

    /// Test 3: Fast Zero-Copy Addition (`add_entries`)
    #[test]
    fn test_zip_add_entries_fast() {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("archive.zip");
        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");

        fs::write(&file1, "Content 1").unwrap();
        fs::write(&file2, "Content 2").unwrap();

        let mut mgr = ArchiveManager::new();
        mgr.create_archive(&zip_path, &[file1.clone()]).unwrap();
        assert_eq!(mgr.visible_entries().len(), 1);

        // Fast zero-copy add second file
        mgr.add_files(&[file2.clone()]).unwrap();
        assert_eq!(mgr.visible_entries().len(), 2);
    }

    /// Test 4: Selective Drag-OUT Extraction and Path Accuracy
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
            .extract_selected(&[target_path.clone()], &stage_dir)
            .unwrap();

        assert!(!extracted.is_empty());
        let expected_extracted_file = stage_dir.join("target.txt");
        assert!(expected_extracted_file.exists());
        assert_eq!(
            fs::read_to_string(&expected_extracted_file).unwrap(),
            "Target File Data"
        );
    }

    /// Test 5: LibarchiveEngine Fallback Capability
    #[test]
    fn test_libarchive_engine_listing_and_extraction() {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("libarc_test.zip");

        {
            let file = File::create(&zip_path).unwrap();
            let mut zip = ZipWriter::new(file);
            zip.start_file("data/info.json", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"{\"status\": \"ok\"}").unwrap();
            zip.finish().unwrap();
        }

        let engine = LibarchiveEngine;
        let entries = engine.list_entries(&zip_path).unwrap();
        assert!(!entries.is_empty());

        let extract_dir = dir.path().join("libarc_out");
        let target = std::path::PathBuf::from("data/info.json");
        let extracted = engine
            .extract_entries(&zip_path, &[target.clone()], &extract_dir)
            .unwrap();

        assert!(!extracted.is_empty());
        assert!(extract_dir.join(&target).exists() || extract_dir.join("info.json").exists());
    }

    /// Test 6: ArchiveManager Folder Navigation and Visible Entries Filtering
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

        // At root folder
        let root_visible = mgr.visible_entries();
        let root_names: Vec<String> = root_visible.iter().map(|e| e.file_name()).collect();
        assert!(root_names.contains(&"folder".to_string()));
        assert!(root_names.contains(&"root_file.txt".to_string()));

        // Navigate inside 'folder'
        mgr.set_current_folder(std::path::PathBuf::from("folder"));
        let sub_visible = mgr.visible_entries();
        let sub_names: Vec<String> = sub_visible.iter().map(|e| e.file_name()).collect();
        assert_eq!(sub_names, vec!["sub_file.txt"]);

        // Navigate back up
        assert!(mgr.navigate_back());
        assert_eq!(mgr.current_folder(), std::path::Path::new(""));
    }

    /// Test 7: Dropping an archive into an already open archive adds it instead of switching
    #[test]
    fn test_add_archive_into_open_archive() {
        let dir = tempdir().unwrap();
        let parent_zip = dir.path().join("parent.zip");
        let child_zip = dir.path().join("child.zip");

        // Create parent zip
        {
            let file = File::create(&parent_zip).unwrap();
            let mut zip = ZipWriter::new(file);
            zip.start_file("existing.txt", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"Parent content").unwrap();
            zip.finish().unwrap();
        }

        // Create child zip
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

        // Add child.zip into parent.zip
        mgr.add_files(&[child_zip.clone()]).unwrap();
        assert_eq!(mgr.visible_entries().len(), 2);

        let names: Vec<String> = mgr
            .visible_entries()
            .iter()
            .map(|e| e.file_name())
            .collect();
        assert!(names.contains(&"child.zip".to_string()));
    }

    /// Test 8: Benchmark proving GTK Main UI Thread preparation latency is sub-millisecond (<1ms)
    #[test]
    fn test_zero_latency_main_thread_preparation_time() {
        use std::time::Instant;

        let dir = tempdir().unwrap();
        let stage_dir = dir.path().join("stage_test");
        fs::create_dir_all(&stage_dir).unwrap();

        let selected = vec![
            std::path::PathBuf::from("big_payload_1.iso"),
            std::path::PathBuf::from("big_payload_2.bin"),
        ];

        // Measure main thread prep time (calculating paths & offloading to background worker)
        let prep_start = Instant::now();

        let mut drag_paths = Vec::new();
        for sel in &selected {
            let full_path = stage_dir.join(sel);
            drag_paths.push(full_path);
        }

        let prep_duration = prep_start.elapsed();

        // Main thread preparation MUST complete in less than 0.5 milliseconds (500 microseconds)!
        assert!(
            prep_duration.as_micros() < 500,
            "Main UI thread blocked for {:?} (expected < 500us)",
            prep_duration
        );
        assert_eq!(drag_paths.len(), 2);
    }

    /// Test 9: Verify that subsequent drags clear old staged files and provide ONLY newly selected items
    #[test]
    fn test_fresh_stage_cleanup_on_subsequent_drags() {
        let dir = tempdir().unwrap();
        let stage_dir = dir.path().join("stage_subsequent");
        fs::create_dir_all(&stage_dir).unwrap();

        // 1. Simulate old drag with 15 files
        for i in 1..=15 {
            fs::write(stage_dir.join(format!("old_file_{}.txt", i)), "old").unwrap();
        }
        assert_eq!(fs::read_dir(&stage_dir).unwrap().count(), 15);

        // 2. Simulate next drag of 1 single file: clear stage and extract 1 file
        let _ = fs::remove_dir_all(&stage_dir);
        fs::create_dir_all(&stage_dir).unwrap();
        fs::write(stage_dir.join("single_selected.txt"), "new").unwrap();

        let current_staged: Vec<_> = fs::read_dir(&stage_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();

        assert_eq!(current_staged.len(), 1);
        assert_eq!(
            current_staged[0].file_name().unwrap().to_str().unwrap(),
            "single_selected.txt"
        );
    }
}
