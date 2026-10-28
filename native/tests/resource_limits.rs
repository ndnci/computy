use computy_engine::{domain::safe_memory_budget, inventory::parse_windows_memory};
use serde_json::json;
#[test]
fn memory_test_budget_respects_a_smaller_container_or_process_limit() {
    let gib = 1024u64.pow(3);
    assert_eq!(
        safe_memory_budget(16 * gib, 8 * gib, Some(96 * 1024 * 1024)),
        32 * 1024 * 1024
    );
    assert_eq!(safe_memory_budget(16 * gib, 8 * gib, None), 8 * gib / 3);
    assert_eq!(safe_memory_budget(gib, gib, Some(0)), 0);
}
#[test]
fn legacy_firmware_capacity_sentinel_is_not_a_claim_of_two_tebibytes() {
    let memory = parse_windows_memory(
        &json!({"arrays":[{"MaxCapacity":2147483648u64,"MaxCapacityEx":0}],"modules":[]}),
    );
    assert_eq!(memory.max_capacity_bytes, None);
}
