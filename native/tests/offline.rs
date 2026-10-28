use computy_engine::{domain::Status, offline::parse_windows_event};
use serde_json::json;
fn event(id: u64) -> serde_json::Value {
    json!({"provider":"Microsoft-Windows-MemoryDiagnostics-Results","event_id":id,"record_id":123,"created_at":2000,"completion_type":"Pass","num_bad_pages":0})
}
#[test]
fn boot_results_require_a_matching_provider_and_a_fresh_event() {
    assert!(parse_windows_event(&event(1201), 3000).is_none());
    let mut forged = event(1201);
    forged["provider"] = json!("Unrelated-Provider");
    assert!(parse_windows_event(&forged, 1000).is_none());
    assert!(parse_windows_event(&event(9999), 1000).is_none());
}
#[test]
fn known_windows_event_ids_preserve_failures() {
    assert_eq!(
        parse_windows_event(&event(1201), 1000).unwrap().status,
        Status::Passed
    );
    assert_eq!(
        parse_windows_event(&event(1202), 1000).unwrap().status,
        Status::Failed
    );
    let mut bad = event(1101);
    bad["num_bad_pages"] = json!(3);
    assert_eq!(
        parse_windows_event(&bad, 1000).unwrap().status,
        Status::Failed
    );
    let mut incomplete = event(1201);
    incomplete["completion_type"] = json!("Aborted");
    assert_eq!(
        parse_windows_event(&incomplete, 1000).unwrap().status,
        Status::Unavailable
    );
}
