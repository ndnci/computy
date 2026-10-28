use computy_engine::{
    diagnostics::{cpu, disk, memory},
    domain::{Report, RunOptions, Status, summarize},
    inventory::{Inventory, parse_windows_memory},
    repository::{FileRepository, ReportRepository},
};
use serde_json::json;
use std::sync::atomic::AtomicBool;

fn temporary(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "computy-{}-{name}",
        computy_engine::domain::new_id()
    ))
}

#[test]
fn disk_verification_detects_corruption_including_truncated_reads() {
    assert_eq!(disk::verify_chunk(&[1, 2, 9, 4], &[1, 2, 3, 4]), 1);
    assert_eq!(disk::verify_chunk(&[1, 2], &[1, 2, 3, 4]), 2);
}

#[test]
fn real_disk_roundtrip_cleans_only_its_own_temporary_file() {
    let root = temporary("disk");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("user-file.txt"), b"precious").unwrap();
    let check = disk::run(&root, 1, &AtomicBool::new(false), &|_, _| {});
    assert_eq!(check.status, Status::Passed);
    assert_eq!(check.metrics["verified_bytes"], 1024 * 1024);
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(
        std::fs::read(root.join("user-file.txt")).unwrap(),
        b"precious"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancelled_disk_test_never_claims_success() {
    let root = temporary("cancelled");
    let check = disk::run(&root, 1, &AtomicBool::new(true), &|_, _| {});
    assert_eq!(check.status, Status::Cancelled);
    assert_eq!(check.score(), None);
    assert!(!root.exists());
}

#[test]
fn real_memory_patterns_complete_and_pre_cancelled_run_has_no_score() {
    let check = memory::run(1, 1, &AtomicBool::new(false), &|_, _| {});
    assert_eq!(check.status, Status::Passed);
    assert_eq!(check.metrics["errors"], 0);
    assert_eq!(check.metrics["patterns_completed"], 5);
    let cancelled = memory::run(1, 1, &AtomicBool::new(true), &|_, _| {});
    assert_eq!(cancelled.status, Status::Cancelled);
    assert_eq!(cancelled.score(), None);
}

#[test]
fn cpu_cancel_does_not_turn_into_a_pass() {
    assert_eq!(
        cpu::run(1, &AtomicBool::new(true), &|_, _| {}).status,
        Status::Cancelled
    );
}

#[test]
fn cim_max_capacity_ex_is_in_kib_not_bytes() {
    let memory = parse_windows_memory(
        &json!({"arrays":[{"MaxCapacityEx":134217728,"MemoryDevices":2}],"modules":[{"Capacity":17179869184u64,"FormFactor":12,"Speed":5600,"ConfiguredClockSpeed":4800,"SMBIOSMemoryType":34}]}),
    );
    assert_eq!(memory.max_capacity_bytes, Some(128 * 1024u64.pow(3)));
    assert_eq!(memory.modules[0].replaceable, Some(true));
    assert_eq!(memory.modules[0].kind, "DDR5");
}

#[test]
fn history_roundtrip_rename_and_tamper_detection() {
    let root = temporary("repository");
    let repository = FileRepository::new(root.clone()).unwrap();
    let id = computy_engine::domain::new_id();
    let report = Report {
        source: "computy_live".into(),
        schema_version: 1,
        id: id.clone(),
        label: "Avant remplacement".into(),
        created_at: 1234,
        duration_ms: 5,
        completed: true,
        options: RunOptions::default(),
        inventory: Inventory::default(),
        checks: vec![],
        summary: summarize(&[]),
    };
    repository.save(&report).unwrap();
    assert_eq!(repository.get(&id).unwrap()["label"], "Avant remplacement");
    repository.rename(&id, "Après remplacement").unwrap();
    assert_eq!(
        repository.list().unwrap().reports[0].label,
        "Après remplacement"
    );
    let path = root.join(format!("{id}.json"));
    let content = std::fs::read_to_string(&path)
        .unwrap()
        .replace("Après remplacement", "Données modifiées");
    std::fs::write(path, content).unwrap();
    assert!(repository.get(&id).is_err());
    assert_eq!(repository.list().unwrap().unreadable, 1);
    std::fs::remove_dir_all(root).unwrap();
}
