use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Passed,
    Warning,
    Failed,
    Unavailable,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub id: String,
    pub component: String,
    pub name: String,
    pub status: Status,
    pub message: String,
    pub duration_ms: u64,
    pub metrics: Value,
    pub limitations: Vec<String>,
}
impl Check {
    pub fn new(id: &str, status: Status, message: &str) -> Self {
        Self {
            id: id.into(),
            component: id.into(),
            name: id.into(),
            status,
            message: message.into(),
            duration_ms: 0,
            metrics: json!({}),
            limitations: vec![],
        }
    }
    pub fn score(&self) -> Option<u8> {
        match self.status {
            Status::Passed => Some(10),
            Status::Warning => Some(5),
            Status::Failed => Some(0),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub status: Status,
    pub score: Option<u8>,
    pub coverage_percent: u8,
    pub checked: usize,
    pub total: usize,
}
pub fn summarize(checks: &[Check]) -> Summary {
    let scores: Vec<_> = checks.iter().filter_map(Check::score).collect();
    let failed = checks.iter().any(|c| c.status == Status::Failed);
    let warning = checks.iter().any(|c| c.status == Status::Warning);
    let incomplete = scores.len() < checks.len();
    Summary {
        status: if failed {
            Status::Failed
        } else if scores.is_empty() {
            Status::Unavailable
        } else if warning || incomplete {
            Status::Warning
        } else {
            Status::Passed
        },
        score: scores.into_iter().min(),
        coverage_percent: if checks.is_empty() {
            0
        } else {
            (checks.iter().filter(|c| c.score().is_some()).count() * 100 / checks.len()) as u8
        },
        checked: checks.iter().filter(|c| c.score().is_some()).count(),
        total: checks.len(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunOptions {
    pub components: Vec<String>,
    pub memory_mib: usize,
    pub passes: usize,
    pub disk_mib: usize,
    pub cpu_seconds: u64,
    pub label: String,
}
impl Default for RunOptions {
    fn default() -> Self {
        Self {
            components: vec!["memory".into(), "storage".into(), "cpu".into()],
            memory_mib: 256,
            passes: 2,
            disk_mib: 64,
            cpu_seconds: 5,
            label: String::new(),
        }
    }
}
pub fn validate_options(options: &RunOptions) -> Result<(), String> {
    if options.components.is_empty()
        || options.components.len() > 3
        || options
            .components
            .iter()
            .any(|c| !["memory", "storage", "cpu"].contains(&c.as_str()))
    {
        return Err("Sélectionnez au moins un composant valide.".into());
    }
    let mut unique = options.components.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != options.components.len() {
        return Err("Composants en double.".into());
    }
    if !(16..=2048).contains(&options.memory_mib)
        || !(1..=8).contains(&options.passes)
        || !(16..=512).contains(&options.disk_mib)
        || !(1..=60).contains(&options.cpu_seconds)
    {
        return Err(
            "Limites : RAM 16–2048 Mio, 1–8 passes, fichier 16–512 Mio, CPU 1–60 s.".into(),
        );
    }
    if options.label.chars().count() > 100 {
        return Err("Le nom doit contenir au maximum 100 caractères.".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    #[serde(default = "live_source")]
    pub source: String,
    pub schema_version: u32,
    pub id: String,
    pub label: String,
    pub created_at: u64,
    pub duration_ms: u64,
    pub completed: bool,
    pub options: RunOptions,
    pub inventory: crate::inventory::Inventory,
    pub checks: Vec<Check>,
    pub summary: Summary,
}
fn live_source() -> String {
    "computy_live".into()
}

pub fn timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn safe_memory_budget(total: u64, available: u64, process_free: Option<u64>) -> u64 {
    let available = process_free.map_or(available, |limit| limit.min(available));
    (available / 3).min(total / 4)
}

pub fn new_id() -> String {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        timestamp_ms(),
        std::process::id(),
        SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}
