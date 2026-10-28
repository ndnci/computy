use computy_engine::{
    domain::{RunOptions, Status, new_id},
    inventory::Inventory,
    preferences::{Preferences, save},
    repository::FileRepository,
    service::{AppService, HardwareProvider},
    transport,
};
use serde_json::json;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

struct Hardware;
impl HardwareProvider for Hardware {
    fn collect(&self) -> Inventory {
        let mut inventory = Inventory::default();
        inventory.memory.ecc_uncorrected = Some(2);
        inventory
    }
}
fn service() -> (Arc<AppService>, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("computy-defaults-{}", new_id()));
    let repository = Arc::new(FileRepository::new(root.join("reports")).unwrap());
    (
        AppService::new(repository, Arc::new(Hardware), root.join("scratch")),
        root,
    )
}
fn finished(service: &AppService, id: &str) -> computy_engine::service::Job {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let job = service.job(id).unwrap();
        if !job.running {
            return job;
        }
        assert!(Instant::now() < deadline, "diagnostic did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn transports_use_saved_defaults_and_allow_explicit_overrides() {
    let (service, root) = service();
    save(
        &root,
        &Preferences {
            memory_mib: 32,
            passes: 3,
            ..Preferences::default()
        },
    )
    .unwrap();
    let started = transport::dispatch(
        &service,
        "start",
        &json!({"components":["memory"],"passes":1}),
    )
    .unwrap();
    let report = finished(&service, started["id"].as_str().unwrap())
        .report
        .unwrap();
    assert_eq!(report.options.memory_mib, 32);
    assert_eq!(report.options.passes, 1);
    service.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn partial_ecc_counters_keep_an_uncorrected_error_visible() {
    let (service, root) = service();
    let id = service
        .start(RunOptions {
            components: vec!["memory".into()],
            ..RunOptions::default()
        })
        .unwrap();
    let report = finished(&service, &id).report.unwrap();
    let ecc = report
        .checks
        .iter()
        .find(|check| check.id == "memory-ecc")
        .unwrap();
    assert_eq!(ecc.status, Status::Failed);
    assert_eq!(ecc.metrics["uncorrected"], 2);
    assert_eq!(report.summary.status, Status::Failed);
    service.shutdown();
    std::fs::remove_dir_all(root).unwrap();
}
