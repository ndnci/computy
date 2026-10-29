use computy_engine::clean::{self, Policy, Rule};
use std::fs;
fn fixture() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "computy-clean-{}",
        computy_engine::domain::new_id()
    ));
    fs::create_dir_all(p.join("cache")).unwrap();
    fs::write(p.join("cache/old.log"), b"old temporary content").unwrap();
    fs::write(p.join("cache/keep.json"), b"{\"important\":true}").unwrap();
    p.canonicalize().unwrap()
}
#[test]
fn preview_is_read_only_and_execution_requires_the_saved_plan_and_confirmation() {
    let p = fixture();
    let policy = Policy {
        roots: vec![p.join("cache")],
        rule: Rule::All {
            rules: vec![
                Rule::Extension {
                    value: "log".into(),
                },
                Rule::MinSize { value: 4 },
            ],
        },
        ..Policy::default()
    };
    let plan = clean::preview(&p.join("state"), policy).unwrap();
    assert_eq!(plan.files.len(), 1);
    assert_eq!(plan.total_bytes, 21);
    assert!(p.join("cache/old.log").exists());
    assert!(clean::execute(&p.join("state"), &plan.id, &[], false).is_err());
    let result = clean::execute(&p.join("state"), &plan.id, &[], true).unwrap();
    assert_eq!(result.deleted, 1);
    assert!(p.join("cache/keep.json").exists());
    assert!(clean::execute(&p.join("state"), &plan.id, &[], true).is_err());
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn changed_files_exclusions_and_nested_boolean_rules_are_respected() {
    let p = fixture();
    let plan = clean::preview(
        &p.join("state"),
        Policy {
            roots: vec![p.join("cache")],
            exclude: vec![p.join("cache/keep.json")],
            rule: Rule::Any {
                rules: vec![
                    Rule::Name {
                        value: "old.log".into(),
                    },
                    Rule::Not {
                        rule: Box::new(Rule::MinSize { value: 0 }),
                    },
                ],
            },
            ..Policy::default()
        },
    )
    .unwrap();
    fs::write(p.join("cache/old.log"), b"changed after preview").unwrap();
    let result = clean::execute(&p.join("state"), &plan.id, &[], true).unwrap();
    assert_eq!(result.deleted, 0);
    assert_eq!(result.skipped, 1);
    assert!(p.join("cache/old.log").exists());
    fs::remove_dir_all(p).unwrap();
}
#[cfg(unix)]
#[test]
fn symlinks_are_never_followed_even_if_a_parent_is_replaced_after_preview() {
    use std::os::unix::fs::symlink;
    let p = fixture();
    fs::create_dir(p.join("outside")).unwrap();
    fs::write(p.join("outside/valuable"), b"save me").unwrap();
    symlink(p.join("outside"), p.join("cache/link")).unwrap();
    let plan = clean::preview(
        &p.join("state"),
        Policy {
            roots: vec![p.join("cache")],
            rule: Rule::All { rules: vec![] },
            ..Policy::default()
        },
    )
    .unwrap();
    assert_eq!(plan.files.len(), 2);
    fs::rename(p.join("cache"), p.join("moved")).unwrap();
    symlink(p.join("outside"), p.join("cache")).unwrap();
    let result = clean::execute(&p.join("state"), &plan.id, &[], true).unwrap();
    assert_eq!(result.deleted, 0);
    assert!(p.join("outside/valuable").exists());
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn automation_is_bound_to_a_preview_and_never_accepts_root_or_invalid_regex() {
    let p = fixture();
    assert!(
        clean::preview(
            &p.join("state"),
            Policy {
                roots: vec![std::path::PathBuf::from("/")],
                ..Policy::default()
            }
        )
        .is_err()
    );
    assert!(
        clean::preview(
            &p.join("state"),
            Policy {
                roots: vec![p.join("cache")],
                rule: Rule::PathRegex { value: "[".into() },
                ..Policy::default()
            }
        )
        .is_err()
    );
    assert!(clean::schedule(&p.join("state"), "missing", 24, 1024, true).is_err());
    let plan = clean::preview(
        &p.join("state"),
        Policy {
            roots: vec![p.join("cache")],
            ..Policy::default()
        },
    )
    .unwrap();
    assert!(clean::schedule(&p.join("state"), &plan.id, 24, 1024, false).is_err());
    let task = clean::schedule(&p.join("state"), &plan.id, 24, 1024, true).unwrap();
    assert_eq!(task.policy.roots, plan.policy.roots);
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn protected_system_subdirectories_and_filesystem_scope_cannot_be_authorized() {
    let p = fixture();
    let existing = if cfg!(target_os = "linux") {
        std::path::PathBuf::from("/var/lib")
    } else if cfg!(target_os = "macos") {
        std::path::PathBuf::from("/System/Library")
    } else {
        std::path::PathBuf::from("C:\\Windows\\System32")
    };
    if existing.is_dir() {
        assert!(
            clean::preview(
                &p.join("state"),
                Policy {
                    roots: vec![existing],
                    ..Policy::default()
                }
            )
            .is_err()
        );
    }
    fs::remove_dir_all(p).unwrap();
}
#[cfg(unix)]
#[test]
fn a_symlinked_state_directory_is_rejected_before_creating_any_file() {
    let p = fixture();
    fs::create_dir_all(p.join("elsewhere")).unwrap();
    fs::create_dir_all(p.join("state")).unwrap();
    std::os::unix::fs::symlink(p.join("elsewhere"), p.join("state/clean")).unwrap();
    assert!(
        clean::preview(
            &p.join("state"),
            Policy {
                roots: vec![p.join("cache")],
                ..Policy::default()
            }
        )
        .is_err()
    );
    assert_eq!(fs::read_dir(p.join("elsewhere")).unwrap().count(), 0);
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn automation_has_a_fresh_dry_run_and_stops_above_its_approved_byte_limit() {
    let p = fixture();
    let plan = clean::preview(
        &p.join("state"),
        Policy {
            roots: vec![p.join("cache")],
            rule: Rule::All { rules: vec![] },
            ..Policy::default()
        },
    )
    .unwrap();
    let task = clean::schedule(&p.join("state"), &plan.id, 24, 1, true).unwrap();
    let schedule_path = p.join("state/clean/schedules.json");
    let mut list = clean::schedules(&p.join("state")).unwrap();
    list[0].next_run = 0;
    fs::write(&schedule_path, serde_json::to_vec(&list).unwrap()).unwrap();
    let result = clean::run_due(&p.join("state")).unwrap();
    assert_eq!(result[0].deleted, 0);
    assert!(!result[0].errors.is_empty());
    assert!(p.join("cache/old.log").exists());
    let mut list = clean::schedules(&p.join("state")).unwrap();
    assert_eq!(list[0].id, task.id);
    list[0].max_bytes = 1024;
    list[0].next_run = 0;
    fs::write(&schedule_path, serde_json::to_vec(&list).unwrap()).unwrap();
    let result = clean::run_due(&p.join("state")).unwrap();
    assert_eq!(result[0].deleted, 2);
    assert!(!p.join("cache/old.log").exists());
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn active_diagnostic_prevents_cleanup_and_policy_creation() {
    let p = fixture();
    fs::create_dir_all(p.join("state")).unwrap();
    let lock = fs::File::create(p.join("state/diagnostic.lock")).unwrap();
    fs2::FileExt::lock_exclusive(&lock).unwrap();
    assert!(
        clean::preview(
            &p.join("state"),
            Policy {
                roots: vec![p.join("cache")],
                ..Policy::default()
            }
        )
        .is_err()
    );
    drop(lock);
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn an_expired_preview_cannot_delete_or_create_an_automation() {
    let p = fixture();
    let state = p.join("state");
    let mut plan = clean::preview(
        &state,
        Policy {
            roots: vec![p.join("cache")],
            rule: Rule::All { rules: vec![] },
            ..Policy::default()
        },
    )
    .unwrap();
    plan.created_at = 0;
    fs::write(
        state.join("clean").join(format!("plan-{}.json", plan.id)),
        serde_json::to_vec(&plan).unwrap(),
    )
    .unwrap();
    assert!(clean::execute(&state, &plan.id, &[], true).is_err());
    assert!(clean::schedule(&state, &plan.id, 24, 1024, true).is_err());
    assert!(p.join("cache/old.log").exists());
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn preview_normalizes_exclusions_including_future_children() {
    let p = fixture();
    let mut exclusion = p.join("cache/keep.json").to_string_lossy().into_owned();
    #[cfg(windows)]
    {
        exclusion = exclusion.trim_start_matches(r"\\?\").to_owned();
    }
    // Windows exercises a normal path against verbatim canonical scan roots.
    #[cfg(not(windows))]
    {
        exclusion = exclusion.replace("/cache/", "/cache//");
    }
    let plan = clean::preview(
        &p.join("state"),
        Policy {
            roots: vec![p.join("cache")],
            exclude: vec![exclusion.into(), p.join("cache/future/subfolder")],
            rule: Rule::All { rules: vec![] },
            ..Policy::default()
        },
    )
    .unwrap();
    assert_eq!(plan.files.len(), 1);
    assert!(plan.files[0].path.ends_with("old.log"));
    assert!(plan.policy.exclude[1].ends_with("future/subfolder"));
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn cleanup_state_is_protected_when_the_scope_contains_its_parent() {
    let p = fixture();
    let state = p.join("state");
    fs::create_dir_all(&state).unwrap();
    fs::write(state.join("preferences.state"), "private state").unwrap();
    #[cfg(windows)]
    let state = std::path::PathBuf::from(state.to_string_lossy().trim_start_matches(r"\\?\"));
    let plan = clean::preview(
        &state,
        Policy {
            roots: vec![p.clone()],
            rule: Rule::All { rules: vec![] },
            ..Policy::default()
        },
    )
    .unwrap();
    assert_eq!(plan.files.len(), 2);
    assert!(
        plan.files
            .iter()
            .all(|f| f.path.starts_with(p.join("cache")))
    );
    fs::remove_dir_all(p).unwrap();
}
