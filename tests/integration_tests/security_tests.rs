use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;
use zonex::archive::ArchiveManager;

#[test]
fn test_encrypt_open_archive_with_password() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("plain.zip");

    let file1 = dir.path().join("secret.txt");
    fs::write(&file1, "Secret Content 12345").unwrap();

    let mut mgr = ArchiveManager::new();
    mgr.create_archive(&zip_path, std::slice::from_ref(&file1))
        .unwrap();

    mgr.encrypt_archive("my_secret_pass").unwrap();

    let open_no_pwd = mgr.open_archive_with_password(&zip_path, None);
    assert!(open_no_pwd.is_err());

    let open_with_pwd = mgr
        .open_archive_with_password(&zip_path, Some("my_secret_pass"))
        .unwrap();
    assert_eq!(open_with_pwd.len(), 1);

    let report = mgr.verify_integrity(Some("my_secret_pass")).unwrap();
    assert_eq!(report.ok_files, 1);
    assert!(report.failed_files.is_empty());
}

#[test]
fn test_zip_slip_path_traversal_prevention() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("malicious.zip");

    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);

        zip.start_file("../../evil_escape.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"Malicious Payload").unwrap();

        zip.finish().unwrap();
    }

    let extract_dir = dir.path().join("safe_output");
    fs::create_dir_all(&extract_dir).unwrap();

    let mut mgr = ArchiveManager::new();
    mgr.open_archive(&zip_path).unwrap();

    let extracted = mgr.extract_selected(&[], &extract_dir).unwrap();
    for path in extracted {
        assert!(
            path.starts_with(&extract_dir),
            "Extracted file escaped destination directory: {}",
            path.display()
        );
    }

    assert!(!dir.path().join("evil_escape.txt").exists());
}

#[test]
fn test_corrupt_file_integrity_report() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("corrupt.zip");

    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);

        zip.start_file(
            "good.txt",
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(b"Good content").unwrap();

        zip.start_file(
            "bad.txt",
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(b"Original content before byte flip").unwrap();

        zip.finish().unwrap();
    }

    let mut bytes = fs::read(&zip_path).unwrap();
    let pos = bytes
        .windows(8)
        .position(|w| w == b"Original")
        .expect("Original not found in zip");
    bytes[pos] ^= 0xFF;
    bytes[pos + 1] ^= 0xFF;
    fs::write(&zip_path, bytes).unwrap();

    let mut mgr = ArchiveManager::new();
    mgr.open_archive(&zip_path).unwrap();

    let report = mgr.verify_integrity(None).unwrap();
    assert_eq!(report.total_files, 2);
    assert_eq!(report.ok_files, 1);
    assert_eq!(report.failed_files.len(), 1);
    assert_eq!(report.failed_files[0].0, "bad.txt");
}

#[test]
fn test_multi_algorithm_encryption() {
    use zonex::archive::EncryptionMethod;

    for method in [
        EncryptionMethod::Aes256,
        EncryptionMethod::Aes192,
        EncryptionMethod::Aes128,
        EncryptionMethod::ZipCrypto,
    ] {
        let dir = tempdir().unwrap();
        let zip_path = dir.path().join("test_enc.zip");
        let file1 = dir.path().join("payload.txt");
        fs::write(&file1, format!("Sensitive data encrypted with {:?}", method)).unwrap();

        let mut mgr = ArchiveManager::new();
        mgr.create_archive(&zip_path, std::slice::from_ref(&file1))
            .unwrap();

        mgr.encrypt_archive_with_method("algorithm_test_pass", method)
            .unwrap();

        let open_no_pwd = mgr.open_archive_with_password(&zip_path, None);
        assert!(open_no_pwd.is_err());

        let open_with_pwd = mgr
            .open_archive_with_password(&zip_path, Some("algorithm_test_pass"))
            .unwrap();
        assert_eq!(open_with_pwd.len(), 1);

        let report = mgr.verify_integrity(Some("algorithm_test_pass")).unwrap();
        assert_eq!(report.ok_files, 1);
        assert!(report.failed_files.is_empty());
    }
}
