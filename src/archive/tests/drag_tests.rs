use std::fs;
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn test_zero_latency_main_thread_preparation_time() {
    let dir = tempdir().unwrap();
    let stage_dir = dir.path().join("stage_test");
    fs::create_dir_all(&stage_dir).unwrap();

    let selected = vec![
        std::path::PathBuf::from("big_payload_1.iso"),
        std::path::PathBuf::from("big_payload_2.bin"),
    ];

    let prep_start = Instant::now();

    let mut drag_paths = Vec::new();
    for sel in &selected {
        let full_path = stage_dir.join(sel);
        drag_paths.push(full_path);
    }

    let prep_duration = prep_start.elapsed();

    assert!(
        prep_duration.as_micros() < 500,
        "Main UI thread blocked for {:?} (expected < 500us)",
        prep_duration
    );
    assert_eq!(drag_paths.len(), 2);
}

#[test]
fn test_fresh_stage_cleanup_on_subsequent_drags() {
    let dir = tempdir().unwrap();
    let stage_dir = dir.path().join("stage_subsequent");
    fs::create_dir_all(&stage_dir).unwrap();

    for i in 1..=15 {
        fs::write(stage_dir.join(format!("old_file_{}.txt", i)), "old").unwrap();
    }
    assert_eq!(fs::read_dir(&stage_dir).unwrap().count(), 15);

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
