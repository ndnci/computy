use computy_engine::{diagnostics::smart::parse_smart, domain::Status};
use serde_json::json;
#[test]
fn a_transport_problem_must_not_hide_an_explicit_hardware_failure() {
    let check =
        parse_smart(&json!({"smartctl":{"exit_status":10},"smart_status":{"passed":false}}));
    assert_eq!(check.status, Status::Failed);
}
