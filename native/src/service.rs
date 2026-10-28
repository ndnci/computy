use crate::{
    domain::{
        Check, Report, RunOptions, Status, new_id, summarize, timestamp_ms, validate_options,
    },
    inventory::{self, Inventory},
    repository::ReportRepository,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub trait HardwareProvider: Send + Sync {
    fn collect(&self) -> Inventory;
}
pub struct SystemHardware;
type StageOperation<'a> = dyn Fn(&dyn Fn(f64, &str)) -> Vec<Check> + 'a;
impl HardwareProvider for SystemHardware {
    fn collect(&self) -> Inventory {
        inventory::collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Job {
    pub id: String,
    pub running: bool,
    pub progress: f64,
    pub phase: String,
    pub component: String,
    pub checks: Vec<Check>,
    pub report: Option<Report>,
    pub error: Option<String>,
    pub cancel_requested: bool,
}
pub struct AppService {
    pub repository: Arc<dyn ReportRepository>,
    hardware: Arc<dyn HardwareProvider>,
    scratch: PathBuf,
    job: Mutex<Option<Job>>,
    cancel: AtomicBool,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
    cached_inventory: Mutex<Option<Inventory>>,
}
impl AppService {
    pub fn data_root(&self) -> PathBuf {
        self.scratch.parent().unwrap_or(&self.scratch).to_path_buf()
    }
    pub fn new(
        repository: Arc<dyn ReportRepository>,
        hardware: Arc<dyn HardwareProvider>,
        scratch: PathBuf,
    ) -> Arc<Self> {
        Arc::new(Self {
            repository,
            hardware,
            scratch,
            job: Mutex::new(None),
            cancel: AtomicBool::new(false),
            worker: Mutex::new(None),
            cached_inventory: Mutex::new(None),
        })
    }
    pub fn inventory(&self, refresh: bool) -> Inventory {
        let mut cache = self
            .cached_inventory
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if refresh || cache.is_none() {
            *cache = Some(self.hardware.collect());
        }
        cache.as_ref().unwrap().clone()
    }
    pub fn job(&self, id: &str) -> Result<Job, String> {
        self.job
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .filter(|j| j.id == id)
            .cloned()
            .ok_or_else(|| "Diagnostic introuvable.".into())
    }
    pub fn active_job(&self) -> Option<Job> {
        self.job.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        let mut guard = self.job.lock().unwrap_or_else(|e| e.into_inner());
        let job = guard
            .as_mut()
            .filter(|j| j.id == id && j.running)
            .ok_or("Aucun diagnostic actif.")?;
        self.cancel.store(true, Ordering::Relaxed);
        job.cancel_requested = true;
        job.phase = "Arrêt en cours…".into();
        Ok(())
    }
    pub fn start(self: &Arc<Self>, options: RunOptions) -> Result<String, String> {
        validate_options(&options)?;
        let mut guard = self.job.lock().unwrap_or_else(|e| e.into_inner());
        if guard.as_ref().is_some_and(|j| j.running) {
            return Err("Un diagnostic est déjà en cours.".into());
        }
        if let Some(previous) = self.worker.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = previous.join();
        }
        let id = new_id();
        *guard = Some(Job {
            id: id.clone(),
            running: true,
            progress: 0.0,
            phase: "Lecture du matériel…".into(),
            component: "system".into(),
            checks: vec![],
            report: None,
            error: None,
            cancel_requested: false,
        });
        self.cancel.store(false, Ordering::Relaxed);
        drop(guard);
        let service = Arc::clone(self);
        let handle = thread::spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| service.run(options)));
            if result.is_err() {
                service.update(|job| {
                    job.running = false;
                    job.error = Some(
                        "Le moteur a interrompu le diagnostic. Aucun succès n’a été enregistré."
                            .into(),
                    );
                });
            }
        });
        *self.worker.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
        Ok(id)
    }
    fn update(&self, action: impl FnOnce(&mut Job)) {
        if let Some(job) = self.job.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            action(job);
        }
    }
    fn run(&self, options: RunOptions) {
        // Cross-process lock: the UI, CLI and MCP must not stress the same store concurrently.
        let lock_path = self.data_root().join("diagnostic.lock");
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path);
        let lock = match lock.and_then(|file| {
            fs2::FileExt::try_lock_exclusive(&file)?;
            Ok(file)
        }) {
            Ok(file) => file,
            Err(_) => {
                self.update(|job|{job.running=false;job.error=Some("Un autre processus Computy utilise déjà le moteur, ou son verrou est inaccessible.".into());});
                return;
            }
        };
        let _lock = lock;
        let started = Instant::now();
        let inventory = self.hardware.collect();
        let has = |component: &str| options.components.iter().any(|c| c == component);
        let total = (if has("memory") { 2 } else { 0 })
            + (if has("storage") { 2 } else { 0 })
            + (if has("cpu") { 1 } else { 0 });
        let mut stage = 0;
        let mut checks = vec![];
        let mut execute = |component: &str, run: &StageOperation<'_>| {
            if !self.cancel.load(Ordering::Relaxed) {
                let progress = |fraction: f64, phase: &str| {
                    self.update(|job| {
                        if !job.cancel_requested {
                            job.phase = phase.into();
                        }
                        job.component = component.into();
                        job.progress = (stage as f64 + fraction.clamp(0.0, 1.0)) / total as f64;
                    })
                };
                checks.extend(run(&progress));
                self.update(|job| job.checks = checks.clone());
            } else {
                let mut check = Check::new(
                    &format!("{component}-skipped-{stage}"),
                    Status::Cancelled,
                    "Contrôle non exécuté après l’annulation.",
                );
                check.component = component.into();
                checks.push(check);
            }
            stage += 1;
        };
        if has("memory") {
            execute("memory", &|progress| {
                // Preserve responsiveness and avoid consuming most of the host's available RAM.
                let budget = inventory.memory.test_budget_bytes;
                if options.memory_mib as u64 * 1024 * 1024 > budget {
                    let mut check = Check::new(
                        "memory-patterns",
                        Status::Unavailable,
                        "Mémoire disponible insuffisante pour l’allocation demandée. Réduisez la taille du test.",
                    );
                    check.component = "memory".into();
                    check.name = "Intégrité mémoire".into();
                    vec![check]
                } else {
                    vec![crate::diagnostics::memory::run(
                        options.memory_mib,
                        options.passes,
                        &self.cancel,
                        progress,
                    )]
                }
            });
            execute("memory", &|progress| {
                let mut check = Check::new(
                    "memory-ecc",
                    Status::Unavailable,
                    "Compteurs ECC non exposés par ce système.",
                );
                check.component = "memory".into();
                check.name = "Compteurs d’erreurs ECC".into();
                let corrected = inventory.memory.ecc_corrected;
                let uncorrected = inventory.memory.ecc_uncorrected;
                check.metrics = json!({"corrected":corrected,"uncorrected":uncorrected});
                if corrected.is_some_and(|count| count > 0)
                    || uncorrected.is_some_and(|count| count > 0)
                    || (corrected.is_some() && uncorrected.is_some())
                {
                    check.status = if uncorrected.is_some_and(|count| count > 0) {
                        Status::Failed
                    } else if corrected.is_some_and(|count| count > 0) {
                        Status::Warning
                    } else {
                        Status::Passed
                    };
                    let corrected = corrected
                        .map(|count| count.to_string())
                        .unwrap_or_else(|| "—".into());
                    let uncorrected = uncorrected
                        .map(|count| count.to_string())
                        .unwrap_or_else(|| "—".into());
                    check.message = format!(
                        "{corrected} erreur(s) corrigée(s), {uncorrected} non corrigée(s)."
                    );
                }
                check.limitations = vec!["Compteurs EDAC depuis le démarrage du système, pas uniquement pendant ce diagnostic.".into()];
                progress(1.0, "Compteurs ECC lus");
                vec![check]
            });
        }
        if has("storage") {
            execute("storage", &|progress| {
                self.smart_checks(&inventory, progress)
            });
            execute("storage", &|progress| {
                let volume = inventory
                    .disks
                    .iter()
                    .filter(|d| self.scratch.starts_with(&d.mount_point))
                    .max_by_key(|d| d.mount_point.len());
                if volume.is_some_and(|d| {
                    d.available_bytes < options.disk_mib as u64 * 1024 * 1024 + 256 * 1024 * 1024
                }) {
                    let mut check = Check::new(
                        "disk-roundtrip",
                        Status::Unavailable,
                        "Espace libre insuffisant ; une réserve de 256 Mio est conservée.",
                    );
                    check.component = "storage".into();
                    vec![check]
                } else {
                    vec![crate::diagnostics::disk::run(
                        &self.scratch,
                        options.disk_mib,
                        &self.cancel,
                        progress,
                    )]
                }
            });
        }
        if has("cpu") {
            execute("cpu", &|progress| {
                vec![crate::diagnostics::cpu::run(
                    options.cpu_seconds,
                    &self.cancel,
                    progress,
                )]
            });
        }
        let completed = !self.cancel.load(Ordering::Relaxed);
        let mut summary = summarize(&checks);
        if !completed && summary.status != Status::Failed {
            summary.status = Status::Cancelled;
            summary.score = None;
        }
        let report = Report {
            source: "computy_live".into(),
            schema_version: 1,
            id: self.active_job().unwrap().id,
            label: if options.label.trim().is_empty() {
                "Diagnostic de l’ordinateur".into()
            } else {
                options.label.trim().into()
            },
            created_at: timestamp_ms() - started.elapsed().as_millis() as u64,
            duration_ms: started.elapsed().as_millis() as u64,
            completed,
            inventory,
            checks,
            summary,
            options,
        };
        let error = self
            .repository
            .save(&report)
            .err()
            .map(|error| format!("Rapport non sauvegardé : {error}"));
        self.update(|job| {
            job.progress = if completed { 1.0 } else { job.progress };
            job.phase = if completed {
                "Diagnostic terminé".into()
            } else {
                "Diagnostic interrompu".into()
            };
            job.checks = report.checks.clone();
            job.report = Some(report);
            job.error = error;
            job.running = false;
        });
    }
    fn smart_checks(&self, inventory: &Inventory, progress: &dyn Fn(f64, &str)) -> Vec<Check> {
        let unavailable = |message: &str| {
            let mut c = Check::new("smart", Status::Unavailable, message);
            c.component = "storage".into();
            c.name = "Télémétrie SMART / NVMe".into();
            vec![c]
        };
        if !inventory.smart_available {
            return unavailable(
                "smartctl absent : installez smartmontools pour lire les indicateurs des disques.",
            );
        }
        let scan = inventory::command("smartctl", &["--scan", "--json"], Duration::from_secs(4))
            .ok()
            .and_then(|o| serde_json::from_str::<Value>(&o.stdout).ok());
        let Some(devices) = scan
            .as_ref()
            .and_then(|v| v["devices"].as_array())
            .filter(|d| !d.is_empty())
        else {
            return unavailable(
                "Aucun disque SMART accessible. Vérifiez les droits et le contrôleur de stockage.",
            );
        };
        let mut checks = vec![];
        for (index, device) in devices.iter().take(16).enumerate() {
            if self.cancel.load(Ordering::Relaxed) {
                break;
            }
            let Some(name) = device["name"]
                .as_str()
                .filter(|s| s.len() < 256 && !s.starts_with('-'))
            else {
                continue;
            };
            progress(
                index as f64 / devices.len().min(16) as f64,
                &format!("Lecture SMART · {name}"),
            );
            let mut args = vec!["--all", "--json", "--nocheck=standby,0"];
            if let Some(kind) = device["type"].as_str() {
                args.extend(["--device", kind]);
            }
            args.push(name);
            let mut check = match inventory::command("smartctl", &args, Duration::from_secs(5)) {
                Ok(output) => match serde_json::from_str::<Value>(&output.stdout) {
                    Ok(mut value) => {
                        // A terminated subprocess is an incomplete read, never exit status 0.
                        value["smartctl"]["exit_status"] =
                            json!(if output.code < 0 { 7 } else { output.code });
                        crate::diagnostics::smart::parse_smart(&value)
                    }
                    Err(_) => unavailable("Sortie smartctl illisible.").remove(0),
                },
                Err(message) => unavailable(&message).remove(0),
            };
            check.id = format!("smart-{index}");
            check.name = format!("SMART · {name}");
            check.metrics["device"] = json!(name);
            checks.push(check);
        }
        if devices.len() > 16 {
            checks.extend(unavailable(
                "Plus de 16 périphériques détectés ; seuls les 16 premiers ont été interrogés.",
            ));
        }
        if checks.is_empty() {
            unavailable("Lecture SMART interrompue ou périphériques indisponibles.")
        } else {
            checks
        }
    }
    pub fn shutdown(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        let handle = self.worker.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}
