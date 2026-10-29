#[test]
fn desktop_fields_do_not_confuse_localized_names_or_actions_with_the_main_entry() {
    let fields = computy_engine::applications::desktop_fields(
        "[Desktop Entry]\nName=Example\nName[fr]=Exemple\nType=Application\nExec=example %U\n[Desktop Action Exit]\nName=Exit\nExec=exit",
    );
    assert_eq!(fields["Name"], "Example");
    assert_eq!(fields["Exec"], "example %U");
}
#[test]
fn fan_readings_do_not_grant_unvalidated_manual_control() {
    let inventory = computy_engine::fans::inventory();
    assert!(!inventory.control_available);
    assert!(inventory.fans.iter().all(|f| !f.manual_supported));
}
