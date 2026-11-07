use crate::archive::detector::{detect, DetectedFormat};
use crate::archive::libarchive_engine::LibarchiveEngine;
use crate::archive::zip_engine::ZipEngine;
use crate::archive::{ArchiveEngine, ArchiveManager};
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

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

    let format = detect(&zip_path);
    assert_eq!(format, Some(DetectedFormat::Zip));

    let mut mgr = ArchiveManager::new();
    let entries = mgr.open_archive(&zip_path).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].file_name(), "hello.txt");
}

#[test]
fn test_corrupt_or_truncated_zip_resilience() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("truncated.zip");

    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file("docs/readme.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"Important Readme Content").unwrap();
        zip.finish().unwrap();
    }

    let file_bytes = fs::read(&zip_path).unwrap();
    let truncated_bytes = &file_bytes[..file_bytes.len().saturating_sub(15)];
    fs::write(&zip_path, truncated_bytes).unwrap();

    let engine = ZipEngine;
    let entries = engine.list_entries(&zip_path).unwrap();
    assert!(!entries.is_empty());
    assert!(entries
        .iter()
        .any(|e| e.path.to_string_lossy().contains("readme.txt")));
}

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
        .extract_entries(&zip_path, std::slice::from_ref(&target), &extract_dir)
        .unwrap();

    assert!(!extracted.is_empty());
    assert!(extract_dir.join(&target).exists() || extract_dir.join("info.json").exists());
}
