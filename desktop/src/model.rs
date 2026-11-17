//! Interface data model, mirroring the engine's JSON contract.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Passed,
    Warning,
    Failed,
    #[default]
    Unavailable,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Check {
    pub id: String,
    pub component: String,
    pub name: String,
    pub status: Status,
    pub message: String,
    pub duration_ms: f64,
    pub metrics: Map<String, Value>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Summary {
    pub status: Status,
    pub score: Option<f64>,
    pub coverage_percent: f64,
    pub checked: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MemoryModule {
    pub locator: String,
    pub capacity_bytes: f64,
    pub kind: String,
    pub form_factor: String,
    pub manufacturer: String,
    pub part_number: String,
    pub speed_mts: Option<f64>,
    pub configured_speed_mts: Option<f64>,
    pub replaceable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Memory {
    pub total_bytes: f64,
    pub available_bytes: f64,
    pub test_budget_bytes: Option<f64>,
    pub max_capacity_bytes: Option<f64>,
    pub max_supported_speed_mts: Option<f64>,
    pub slots: Option<u64>,
    pub modules: Vec<MemoryModule>,
    pub source: String,
    pub notes: Vec<String>,
    pub ecc_corrected: Option<f64>,
    pub ecc_uncorrected: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Cpu {
    pub name: String,
    pub vendor: String,
    pub physical_cores: Option<u64>,
    pub logical_cores: u64,
    pub frequency_mhz: f64,
    pub usage_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Disk {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub kind: String,
    pub total_bytes: f64,
    pub available_bytes: f64,
    pub removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Temperature {
    pub name: String,
    pub celsius: Option<f64>,
    pub critical_celsius: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Inventory {
    pub runtime: Option<String>,
    pub captured_at: f64,
    pub machine_id: String,
    pub host_name: String,
    pub model: String,
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub kernel: String,
    pub uptime_seconds: f64,
    pub memory: Memory,
    pub cpu: Cpu,
    pub disks: Vec<Disk>,
    pub temperatures: Vec<Temperature>,
    pub smart_available: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct RunOptions {
    pub components: Vec<String>,
    pub memory_mib: u64,
    pub passes: u64,
    pub disk_mib: u64,
    pub cpu_seconds: u64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Report {
    pub source: Option<String>,
    pub schema_version: u64,
    pub id: String,
    pub label: String,
    pub created_at: f64,
    pub duration_ms: f64,
    pub completed: bool,
    pub options: RunOptions,
    pub inventory: Inventory,
    pub checks: Vec<Check>,
    pub summary: Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct History {
    pub reports: Vec<Report>,
    pub unreadable: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Preferences {
    pub memory_mib: u64,
    pub passes: u64,
    pub disk_mib: u64,
    pub cpu_seconds: u64,
    pub theme: String,
    pub language: String,
    pub tray_enabled: bool,
    pub keep_running_on_close: bool,
    pub tray_style: String,
    pub tray_components: Vec<String>,
    pub tray_disks: Vec<String>,
    pub battery_enabled: bool,
    pub battery_devices: Vec<BatteryDevice>,
}

/// Tray choice for one wireless peripheral, remembered while disconnected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct BatteryDevice {
    pub id: String,
    pub name: String,
    pub shown: bool,
    pub icon: String,
}

/// Battery of a connected wireless peripheral (engine `peripherals`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Peripheral {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub percent: Option<f64>,
    pub level: String,
    pub approximate: bool,
    pub charging: Option<bool>,
    pub connected: bool,
}

impl Peripheral {
    /// Gauge value; a coarse level maps to the middle of its range.
    pub fn gauge(&self) -> Option<f64> {
        self.percent.or(match self.level.as_str() {
            "critical" => Some(5.),
            "low" => Some(20.),
            "normal" => Some(55.),
            "high" => Some(80.),
            "full" => Some(100.),
            _ => None,
        })
    }

    /// Icon kind: the user's choice, otherwise the detected kind.
    pub fn icon_kind<'a>(&'a self, preferences: &'a Preferences) -> &'a str {
        preferences
            .battery_devices
            .iter()
            .find(|d| d.id == self.id && d.icon != "auto")
            .map_or(self.kind.as_str(), |d| d.icon.as_str())
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            memory_mib: 256,
            passes: 2,
            disk_mib: 64,
            cpu_seconds: 5,
            theme: "system".into(),
            language: "system".into(),
            tray_enabled: true,
            keep_running_on_close: true,
            tray_style: "icon_text".into(),
            tray_components: vec!["memory".into(), "cpu".into(), "gpu".into()],
            tray_disks: vec![],
            battery_enabled: false,
            battery_devices: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProcessSample {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: Option<f64>,
    pub memory_percent: Option<f64>,
    pub cpu_percent: Option<f64>,
    pub read_bytes_per_second: Option<f64>,
    pub written_bytes_per_second: Option<f64>,
    pub io_percent: Option<f64>,
    pub gpu_memory_bytes: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DiskSample {
    pub id: String,
    pub name: String,
    pub used_percent: Option<f64>,
    pub read_bytes_per_second: Option<f64>,
    pub written_bytes_per_second: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MonitorSnapshot {
    pub captured_at: f64,
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub gpu_percent: Option<f64>,
    pub gpu_source: String,
    pub processes: Vec<ProcessSample>,
    pub disks: Vec<DiskSample>,
}

impl MonitorSnapshot {
    pub fn metric(&self, metric: &str, volume: &str) -> Option<f64> {
        match metric {
            "memory" => self.memory_percent,
            "cpu" => self.cpu_percent,
            "gpu" => self.gpu_percent,
            _ => self.disks.iter().find(|d| d.id == volume)?.used_percent,
        }
    }
}

impl ProcessSample {
    pub fn value(&self, metric: &str) -> Option<f64> {
        match metric {
            "memory" => self.memory_bytes,
            "cpu" => self.cpu_percent,
            "gpu" => self.gpu_memory_bytes,
            _ => Some(self.read_bytes_per_second? + self.written_bytes_per_second?),
        }
    }
}

pub fn ranked_processes(processes: &[ProcessSample], metric: &str) -> Vec<ProcessSample> {
    let mut sorted = processes.to_vec();
    sorted.sort_by(|a, b| {
        let av = a.value(metric).unwrap_or(-1.0);
        let bv = b.value(metric).unwrap_or(-1.0);
        bv.partial_cmp(&av)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.pid.cmp(&b.pid))
    });
    sorted
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Fan {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub rpm: Option<f64>,
    pub pwm_percent: Option<f64>,
    pub mode: Option<String>,
    pub manual_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FanInventory {
    pub fans: Vec<Fan>,
    pub control_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub manager: String,
    pub package: Option<String>,
    pub location: Option<String>,
    pub launcher: Option<String>,
    pub launch_target: Option<String>,
    pub system: bool,
    pub can_uninstall: bool,
    pub can_remove_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ApplicationCatalog {
    pub apps: Vec<Application>,
    pub warnings: Vec<String>,
}
