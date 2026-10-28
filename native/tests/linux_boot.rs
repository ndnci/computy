#![cfg(target_os = "linux")]
use computy_engine::linux_boot::{Facts, Snapshot, assess, memtest_entry, validate_snapshot};
#[test]
fn an_unconfirmed_or_older_request_cannot_authorize_reboot() {
    let request: computy_engine::linux_boot::Request = serde_json::from_value(
        serde_json::json!({"owner_uid":1000,"requested_at":1,"boot_id":"test"}),
    )
    .unwrap();
    assert!(!request.scheduled);
}

fn facts() -> Facts {
    Facts {
        architecture: "x86_64".into(),
        container: false,
        uefi: true,
        secure_boot: Some(false),
        helper_installed: true,
        payload_installed: true,
    }
}
fn snapshot() -> Snapshot {
    Snapshot { facts: facts(), grub_config: "load_env\nset default=\"${next_entry}\"\nset next_entry=\nsave_env next_entry\nmenuentry 'Memory test' --class computy $menuentry_id_option 'computy-memory-test' {\n linux /computy-memory.efi\n}\n".into(),
        filesystem: "ext2".into(), abstractions: String::new(), environment: String::new() }
}
#[test]
fn secure_boot_and_unknown_firmware_block_automatic_scheduling() {
    let mut input = facts();
    input.secure_boot = Some(true);
    assert_eq!(assess(&input), "secure_boot");
    input.secure_boot = None;
    assert_eq!(assess(&input), "secure_boot_unknown");
    input.secure_boot = Some(false);
    assert_eq!(assess(&input), "ready");
    input.container = true;
    assert_eq!(assess(&input), "container");
}
#[test]
fn only_the_normal_top_level_distribution_memtest_entry_is_selected() {
    assert_eq!(
        memtest_entry(&snapshot().grub_config),
        Some("computy-memory-test")
    );
    assert_eq!(
        memtest_entry(
            "menuentry 'serial' --class computy $menuentry_id_option 'computy-memory-test-serial' {\n}"
        ),
        None
    );
    assert_eq!(
        memtest_entry(
            "submenu 'tools' {\nmenuentry 'memory' --class computy $menuentry_id_option 'computy-memory-test' {\n}\n}"
        ),
        None
    );
    assert_eq!(
        memtest_entry("menuentry 'Linux' --class gnu-linux 'computy-memory-test' {\n}"),
        None
    );
}
#[test]
fn one_shot_requires_a_writable_supported_boot_environment_and_never_overrides_another_plan() {
    let mut state = snapshot();
    assert!(validate_snapshot(&state).is_ok());
    state.filesystem = "btrfs".into();
    assert!(validate_snapshot(&state).is_err());
    state.filesystem = "ext2".into();
    state.abstractions = "lvm".into();
    assert!(validate_snapshot(&state).is_err());
    state.abstractions.clear();
    state.environment = "next_entry=another-os\n".into();
    assert!(validate_snapshot(&state).is_err());
    state.environment.clear();
    state.grub_config = state
        .grub_config
        .replace("save_env next_entry", "# unsupported");
    assert!(validate_snapshot(&state).is_err());
}
