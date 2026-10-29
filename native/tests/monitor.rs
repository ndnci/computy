use computy_engine::{monitor, preferences::Preferences};

#[test]
fn old_preferences_enable_only_the_three_default_tray_indicators() {
    let value: Preferences = serde_json::from_str(r#"{"language":"fr"}"#).unwrap();
    assert!(value.tray_enabled);
    assert_eq!(value.tray_style, "icon_text");
    assert_eq!(value.tray_components, ["memory", "cpu", "gpu"]);
    assert!(value.tray_disks.is_empty());
    assert!(computy_engine::preferences::validate(&value).is_ok());
    let invalid = Preferences {
        tray_components: vec!["unknown".into()],
        ..value
    };
    assert!(computy_engine::preferences::validate(&invalid).is_err());
    assert!(
        computy_engine::preferences::validate(&Preferences {
            tray_style: "unknown".into(),
            ..Default::default()
        })
        .is_err()
    );
    let oversized = Preferences {
        tray_disks: (0..16)
            .map(|n| format!("/{}{n}", "\"".repeat(500)))
            .collect(),
        ..Default::default()
    };
    assert!(serde_json::to_vec(&oversized).unwrap().len() > 8192);
    assert!(computy_engine::preferences::validate(&oversized).is_err());
}

#[test]
fn gpu_values_preserve_unknown_and_reject_invalid_percentages() {
    assert_eq!(monitor::parse_percent("0\n"), Some(0.0));
    assert_eq!(monitor::parse_percent("100"), Some(100.0));
    for value in ["N/A", "", "NaN", "inf", "101", "-1", "40 % garbage"] {
        assert_eq!(monitor::parse_percent(value), None, "{value}");
    }
    assert_eq!(monitor::nvidia_percent("N/A\n27\n81\n"), Some(81.0));
    assert_eq!(monitor::nvidia_percent("N/A"), None);
}

#[test]
fn telemetry_uses_real_samples_and_never_records_a_diagnostic() {
    let mut sampler = monitor::Sampler::default();
    let first = sampler.sample(true);
    assert!(first.cpu_percent.is_none(), "CPU needs two spaced samples");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let next = sampler.sample(true);
    assert!(
        next.memory_percent
            .is_some_and(|n| (0.0..=100.0).contains(&n))
    );
    assert!(next.cpu_percent.is_some_and(|n| (0.0..=100.0).contains(&n)));
    for disk in next.disks {
        assert!(disk.used_percent.is_none_or(|n| (0.0..=100.0).contains(&n)));
    }
    assert!(!next.processes.is_empty());
    for process in &next.processes {
        assert!(
            process
                .cpu_percent
                .is_none_or(|n| (0.0..=100.0).contains(&n))
        );
        #[cfg(target_os = "linux")]
        if let Ok(status) = std::fs::read_to_string(format!("/proc/{}/status", process.pid))
            && let Some(tgid) = status.lines().find_map(|line| {
                line.strip_prefix("Tgid:")
                    .and_then(|value| value.trim().parse::<u32>().ok())
            })
        {
            assert_eq!(
                process.pid, tgid,
                "Linux thread must not duplicate its process memory"
            );
        }
        assert!(
            process
                .memory_percent
                .is_none_or(|n| (0.0..=100.0).contains(&n))
        );
    }
    let serialized = serde_json::to_string(&next.processes).unwrap();
    assert!(!serialized.contains("\"cmd\":"));
    assert!(!serialized.contains("\"environ\":"));
}

#[test]
fn top_ten_ranks_known_values_and_keeps_the_complete_snapshot() {
    use monitor::{ProcessSample, Snapshot};
    let snapshot = Snapshot {
        processes: (0..24)
            .map(|pid| ProcessSample {
                pid,
                memory_bytes: Some(u64::from(pid) * 1024),
                cpu_percent: Some(f64::from(24 - pid)),
                ..ProcessSample::default()
            })
            .chain(std::iter::once(ProcessSample {
                pid: 999,
                ..Default::default()
            }))
            .collect(),
        ..Default::default()
    };
    let memory = monitor::top_processes(&snapshot, "memory");
    assert_eq!(memory.len(), 10);
    assert_eq!(memory[0].pid, 23);
    assert_eq!(monitor::top_processes(&snapshot, "cpu")[0].pid, 0);
    assert!(monitor::top_processes(&snapshot, "gpu").is_empty());
    assert!(monitor::top_processes(&snapshot, "storage").is_empty());
    assert_eq!(snapshot.processes.len(), 25);
}

#[test]
fn gpu_process_memory_sums_devices_and_ignores_unknown_rows() {
    let values = monitor::nvidia_process_memory("42, 10\n42, 20\n17, N/A\n8, 0\ninvalid\n");
    assert_eq!(values.get(&42), Some(&(30 * 1024 * 1024)));
    assert_eq!(values.get(&8), Some(&0));
    assert!(!values.contains_key(&17));
}
