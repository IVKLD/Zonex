use std::fs;
use tempfile::tempdir;
use zonex::archive::detector::{detect, DetectedFormat};
use zonex::archive::tar_engine::{TarCompression, TarEngine};
use zonex::archive::ArchiveEngine;

#[test]
fn test_multi_format_creation_listing_extraction() {
    let dir = tempdir().unwrap();

    let tar_path = dir.path().join("test.tar");
    let file1 = dir.path().join("doc.txt");
    fs::write(&file1, "Plain Tar Content").unwrap();

    let tar_engine = TarEngine::new(TarCompression::None);
    tar_engine
        .create_archive(&tar_path, std::slice::from_ref(&file1))
        .unwrap();
    assert_eq!(detect(&tar_path), Some(DetectedFormat::Tar));

    let tar_entries = tar_engine.list_entries(&tar_path).unwrap();
    assert!(!tar_entries.is_empty());
    assert_eq!(tar_entries[0].size, 17);

    let extract_dir_tar = dir.path().join("out_tar");
    let extracted_tar = tar_engine
        .extract_entries(&tar_path, &[], &extract_dir_tar)
        .unwrap();
    assert!(!extracted_tar.is_empty());

    let targz_path = dir.path().join("test.tar.gz");
    let targz_engine = TarEngine::new(TarCompression::Gzip);
    targz_engine
        .create_archive(&targz_path, std::slice::from_ref(&file1))
        .unwrap();
    assert_eq!(detect(&targz_path), Some(DetectedFormat::TarGz));

    let targz_entries = targz_engine.list_entries(&targz_path).unwrap();
    assert_eq!(targz_entries[0].size, 17);

    let tarzst_path = dir.path().join("test.tar.zst");
    let tarzst_engine = TarEngine::new(TarCompression::Zstd);
    tarzst_engine
        .create_archive(&tarzst_path, std::slice::from_ref(&file1))
        .unwrap();
    assert_eq!(detect(&tarzst_path), Some(DetectedFormat::TarZst));

    let tarzst_entries = tarzst_engine.list_entries(&tarzst_path).unwrap();
    assert_eq!(tarzst_entries[0].size, 17);
}
