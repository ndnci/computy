use computy_engine::diagnostics::{memory::verify_words, smart::parse_smart};
use computy_engine::domain::{Check, RunOptions, Status, summarize, validate_options};
use computy_engine::inventory::parse_dmidecode;
use computy_engine::repository::{FileRepository, ReportRepository};
use serde_json::json;

#[test]
fn missing_evidence_is_never_a_healthy_score() {
    let summary = summarize(&[Check::new("smart", Status::Unavailable, "Access denied")]);
    assert_eq!(summary.score, None);
    assert_eq!(summary.status, Status::Unavailable);
}

#[test]
fn one_failed_check_cannot_be_averaged_away() {
    let summary = summarize(&[
        Check::new("ram", Status::Passed, "No mismatch"),
        Check::new("disk", Status::Failed, "Data mismatch"),
    ]);
    assert_eq!(summary.score, Some(0));
    assert_eq!(summary.status, Status::Failed);
}

#[test]
fn incomplete_coverage_remains_visible() {
    let summary = summarize(&[
        Check::new("ram", Status::Passed, "Sample checked"),
        Check::new("smart", Status::Unavailable, "Not installed"),
    ]);
    assert_eq!(summary.score, Some(10));
    assert_eq!(summary.coverage_percent, 50);
    assert_eq!(summary.status, Status::Warning);
}

#[test]
fn memory_verifier_detects_injected_corruption_and_keeps_first_offset() {
    let mut words: Vec<u64> = (0..1024).map(|i| i as u64 ^ 0x55aa).collect();
    words[19] ^= 1;
    words[900] ^= 8;
    let result = verify_words(&words, |i| i as u64 ^ 0x55aa);
    assert_eq!(result.errors, 2);
    assert_eq!(result.first_error.unwrap().offset_bytes, 19 * 8);
}

#[test]
fn smart_permission_error_is_unavailable_even_when_json_says_passed() {
    let result = parse_smart(&json!({"smartctl":{"exit_status":2},"smart_status":{"passed":true}}));
    assert_eq!(result.status, Status::Unavailable);
    assert_eq!(result.score(), None);
}

#[test]
fn nvme_critical_warning_and_media_errors_are_failures() {
    for health in [json!({"critical_warning":4}), json!({"media_errors":1})] {
        let result = parse_smart(
            &json!({"smartctl":{"exit_status":0},"smart_status":{"passed":true},"nvme_smart_health_information_log":health}),
        );
        assert_eq!(result.status, Status::Failed);
    }
}

#[test]
fn smart_exit_status_bitmask_is_not_a_boolean() {
    let result =
        parse_smart(&json!({"smartctl":{"exit_status":64},"smart_status":{"passed":true}}));
    assert_eq!(result.status, Status::Warning);
}

#[test]
fn absent_smart_health_must_not_be_promoted_to_pass() {
    assert_eq!(
        parse_smart(&json!({"smartctl":{"exit_status":0}})).status,
        Status::Unavailable
    );
}

#[test]
fn firmware_capacity_does_not_invent_a_maximum_frequency() {
    let text = "Physical Memory Array\n\tLocation: System Board Or Motherboard\n\tUse: System Memory\n\tMaximum Capacity: 128 GB\n\tNumber Of Devices: 4\n\nMemory Device\n\tSize: 16 GB\n\tForm Factor: DIMM\n\tLocator: DIMM_A1\n\tType: DDR5\n\tSpeed: 5600 MT/s\n\tConfigured Memory Speed: 4800 MT/s\n\tManufacturer: Example\n\tPart Number: TEST-16\n";
    let memory = parse_dmidecode(text);
    assert_eq!(memory.max_capacity_bytes, Some(128 * 1024u64.pow(3)));
    assert_eq!(memory.slots, Some(4));
    assert_eq!(memory.modules[0].speed_mts, Some(5600));
    assert_eq!(memory.modules[0].configured_speed_mts, Some(4800));
    assert_eq!(memory.max_supported_speed_mts, None);
    assert_eq!(memory.modules[0].replaceable, Some(true));
}

#[test]
fn unknown_form_factor_is_not_assumed_replaceable() {
    let memory = parse_dmidecode("Memory Device\n\tSize: 8 GB\n\tForm Factor: Unknown\n");
    assert_eq!(memory.modules[0].replaceable, None);
}

#[test]
fn destructive_or_unbounded_requests_are_rejected() {
    let mut options = RunOptions {
        memory_mib: 0,
        ..RunOptions::default()
    };
    assert!(validate_options(&options).is_err());
    options.memory_mib = 65536;
    assert!(validate_options(&options).is_err());
    options.memory_mib = 64;
    options.passes = 1000;
    assert!(validate_options(&options).is_err());
}

#[test]
fn repository_rejects_path_traversal() {
    let root = std::env::temp_dir().join(format!("computy-contract-{}", std::process::id()));
    let repository = FileRepository::new(root.clone()).unwrap();
    assert!(repository.get("../private").is_err());
    assert!(repository.delete("../private").is_err());
    std::fs::remove_dir_all(root).unwrap();
}
