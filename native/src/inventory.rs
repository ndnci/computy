use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use sysinfo::{Components, Disks, System};
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryModule {
    pub locator: String,
    pub capacity_bytes: u64,
    pub kind: String,
    pub form_factor: String,
    pub manufacturer: String,
    pub part_number: String,
    pub speed_mts: Option<u64>,
    pub configured_speed_mts: Option<u64>,
    pub replaceable: Option<bool>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryInventory {
    pub total_bytes: u64,
    pub available_bytes: u64,
    #[serde(default)]
    pub test_budget_bytes: u64,
    pub max_capacity_bytes: Option<u64>,
    pub slots: Option<u64>,
    pub max_supported_speed_mts: Option<u64>,
    pub modules: Vec<MemoryModule>,
    pub source: String,
    pub notes: Vec<String>,
    pub ecc_corrected: Option<u64>,
    pub ecc_uncorrected: Option<u64>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskInventory {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub kind: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub removable: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuInventory {
    pub name: String,
    pub vendor: String,
    pub physical_cores: Option<usize>,
    pub logical_cores: usize,
    pub frequency_mhz: u64,
    pub usage_percent: Option<f32>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Temperature {
    pub name: String,
    pub celsius: Option<f32>,
    pub critical_celsius: Option<f32>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    #[serde(default)]
    pub runtime: String,
    pub captured_at: u64,
    pub machine_id: String,
    pub host_name: String,
    pub model: String,
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub kernel: String,
    pub uptime_seconds: u64,
    pub memory: MemoryInventory,
    pub cpu: CpuInventory,
    pub disks: Vec<DiskInventory>,
    pub temperatures: Vec<Temperature>,
    pub smart_available: bool,
    pub notes: Vec<String>,
}

pub struct CommandOutput {
    pub code: i32,
    pub stdout: String,
}
pub fn command(program: &str, args: &[&str], timeout: Duration) -> Result<CommandOutput, String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd
        .spawn()
        .map_err(|_| format!("{program} indisponible."))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Sortie de commande indisponible.")?;
    let reader = thread::spawn(move || {
        let mut data = Vec::new();
        stdout
            .take(4 * 1024 * 1024)
            .read_to_end(&mut data)
            .map(|_| data)
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status.code().unwrap_or(-1)),
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!("{program} : délai dépassé."));
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| "Erreur de lecture de commande.")?
        .map_err(|e| e.to_string())?;
    Ok(CommandOutput {
        code: status?,
        stdout: String::from_utf8_lossy(&bytes).into_owned(),
    })
}

fn capacity(text: &str) -> Option<u64> {
    let mut parts = text.split_whitespace();
    let value = parts.next()?.parse::<u64>().ok()?;
    let multiplier = match parts.next()?.to_uppercase().as_str() {
        "KB" => 1024,
        "MB" => 1024u64.pow(2),
        "GB" => 1024u64.pow(3),
        "TB" => 1024u64.pow(4),
        _ => return None,
    };
    value.checked_mul(multiplier)
}
fn speed(text: &str) -> Option<u64> {
    text.split_whitespace()
        .next()?
        .parse()
        .ok()
        .filter(|v| *v > 0)
}
fn field<'a>(block: &'a str, key: &str) -> &'a str {
    block
        .lines()
        .filter_map(|line| line.trim().split_once(':'))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| value.trim())
        .unwrap_or("")
}
fn replacement(form: &str) -> Option<bool> {
    match form {
        "DIMM" | "SODIMM" | "SO-DIMM" => Some(true),
        "Chip" | "Die" | "Row Of Chips" => Some(false),
        _ => None,
    }
}
pub fn parse_dmidecode(text: &str) -> MemoryInventory {
    let mut memory = MemoryInventory {
        source: "SMBIOS / dmidecode".into(),
        ..Default::default()
    };
    for block in text.split("\n\n") {
        if block.lines().any(|l| l.trim() == "Physical Memory Array")
            && field(block, "Use") == "System Memory"
        {
            if let Some(value) = capacity(field(block, "Maximum Capacity")) {
                memory.max_capacity_bytes =
                    Some(memory.max_capacity_bytes.unwrap_or(0).saturating_add(value));
            }
            if let Ok(value) = field(block, "Number Of Devices").parse::<u64>() {
                memory.slots = Some(memory.slots.unwrap_or(0) + value);
            }
        } else if block.lines().any(|l| l.trim() == "Memory Device") {
            let Some(size) = capacity(field(block, "Size")) else {
                continue;
            };
            let form = field(block, "Form Factor");
            memory.modules.push(MemoryModule {
                locator: field(block, "Locator").into(),
                capacity_bytes: size,
                kind: field(block, "Type").into(),
                form_factor: form.into(),
                manufacturer: field(block, "Manufacturer").into(),
                part_number: field(block, "Part Number").into(),
                speed_mts: speed(field(block, "Speed")),
                configured_speed_mts: speed(field(block, "Configured Memory Speed")),
                replaceable: replacement(form),
            });
        }
    }
    memory
}

#[cfg(target_os = "linux")]
fn platform_memory() -> MemoryInventory {
    let mut memory = match command("dmidecode", &["--type", "16,17"], Duration::from_secs(3)) {
        Ok(output) if output.code == 0 => parse_dmidecode(&output.stdout),
        _ => MemoryInventory { source: "OS Linux".into(), notes: vec!["Détails SMBIOS non accessibles : dmidecode requiert généralement des droits administrateur.".into()], ..Default::default() },
    };
    let counters = std::fs::read_dir("/sys/devices/system/edac/mc")
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with("mc"))
                .map(|entry| {
                    let read = |name: &str| {
                        std::fs::read_to_string(entry.path().join(name))
                            .ok()
                            .and_then(|s| s.trim().parse::<u64>().ok())
                    };
                    (read("ce_count"), read("ue_count"))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !counters.is_empty() {
        memory.ecc_corrected = counters
            .iter()
            .map(|c| c.0)
            .collect::<Option<Vec<_>>>()
            .map(|v| v.iter().sum());
        memory.ecc_uncorrected = counters
            .iter()
            .map(|c| c.1)
            .collect::<Option<Vec<_>>>()
            .map(|v| v.iter().sum());
    }
    memory
}

#[cfg(target_os = "windows")]
fn platform_memory() -> MemoryInventory {
    // Only fixed scripts are executed. User input is never interpolated into PowerShell.
    let script = "[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new(); @{modules=@(Get-CimInstance Win32_PhysicalMemory | Select-Object Capacity,DeviceLocator,Manufacturer,PartNumber,FormFactor,SMBIOSMemoryType,Speed,ConfiguredClockSpeed); arrays=@(Get-CimInstance Win32_PhysicalMemoryArray | Where-Object Use -eq 3 | Select-Object MaxCapacity,MaxCapacityEx,MemoryDevices)} | ConvertTo-Json -Depth 5 -Compress";
    match command(
        "powershell.exe",
        &[
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ],
        Duration::from_secs(10),
    )
    .ok()
    .filter(|o| o.code == 0)
    .and_then(|o| serde_json::from_str::<Value>(&o.stdout).ok())
    {
        Some(value) => parse_windows_memory(&value),
        None => MemoryInventory {
            source: "OS Windows".into(),
            notes: vec!["Les détails CIM/SMBIOS ne sont pas accessibles.".into()],
            ..Default::default()
        },
    }
}

pub fn parse_windows_memory(value: &Value) -> MemoryInventory {
    let mut memory = MemoryInventory {
        source: "Windows CIM / SMBIOS".into(),
        ..Default::default()
    };
    if let Some(arrays) = value["arrays"].as_array() {
        for array in arrays {
            // Both MaxCapacity and MaxCapacityEx are documented in KiB, not bytes.
            let cap = array["MaxCapacityEx"]
                .as_u64()
                .filter(|c| *c > 0)
                .or_else(|| {
                    array["MaxCapacity"]
                        .as_u64()
                        .filter(|capacity| *capacity != 2147483648)
                });
            if let Some(cap) = cap.filter(|c| *c > 0).and_then(|c| c.checked_mul(1024)) {
                memory.max_capacity_bytes =
                    Some(memory.max_capacity_bytes.unwrap_or(0).saturating_add(cap));
            }
            if let Some(slots) = array["MemoryDevices"].as_u64() {
                memory.slots = Some(memory.slots.unwrap_or(0) + slots);
            }
        }
    }
    if let Some(modules) = value["modules"].as_array() {
        for module in modules {
            let text = |key: &str| module[key].as_str().unwrap_or("").trim().to_owned();
            let form = match module["FormFactor"].as_u64() {
                Some(8) => "DIMM",
                Some(12) => "SODIMM",
                Some(5) => "Chip",
                Some(4) => "Row Of Chips",
                Some(11) => "Die",
                _ => "Unknown",
            };
            let kind = match module["SMBIOSMemoryType"].as_u64() {
                Some(24) => "DDR3",
                Some(26) => "DDR4",
                Some(30) => "LPDDR4",
                Some(34) => "DDR5",
                Some(35) => "LPDDR5",
                _ => "Inconnu",
            };
            memory.modules.push(MemoryModule {
                locator: text("DeviceLocator"),
                capacity_bytes: module["Capacity"].as_u64().unwrap_or(0),
                kind: kind.into(),
                form_factor: form.into(),
                manufacturer: text("Manufacturer"),
                part_number: text("PartNumber"),
                speed_mts: module["Speed"].as_u64().filter(|s| *s > 0),
                configured_speed_mts: module["ConfiguredClockSpeed"].as_u64().filter(|s| *s > 0),
                replaceable: replacement(form),
            });
        }
    }
    memory
}

#[cfg(target_os = "macos")]
fn platform_memory() -> MemoryInventory {
    let output = command(
        "/usr/sbin/system_profiler",
        &["SPMemoryDataType", "-json", "-detailLevel", "mini"],
        Duration::from_secs(10),
    );
    let mut memory = MemoryInventory {
        source: "macOS system_profiler".into(),
        ..Default::default()
    };
    if let Ok(output) = output
        && let Ok(value) = serde_json::from_str::<Value>(&output.stdout)
    {
        fn visit(value: &Value, modules: &mut Vec<MemoryModule>) {
            if let Some(size) = value
                .get("dimm_size")
                .and_then(Value::as_str)
                .and_then(capacity)
            {
                let text = |key: &str| value[key].as_str().unwrap_or("").to_owned();
                modules.push(MemoryModule {
                    locator: text("_name"),
                    capacity_bytes: size,
                    kind: text("dimm_type"),
                    manufacturer: text("dimm_manufacturer"),
                    part_number: text("dimm_part_number"),
                    speed_mts: value["dimm_speed"].as_str().and_then(speed),
                    ..Default::default()
                });
            }
            match value {
                Value::Array(items) => {
                    for item in items {
                        visit(item, modules);
                    }
                }
                Value::Object(map) => {
                    for item in map.values() {
                        visit(item, modules);
                    }
                }
                _ => {}
            }
        }
        visit(&value, &mut memory.modules);
    }
    #[cfg(target_arch = "aarch64")]
    memory.notes.push("Apple Silicon : mémoire unifiée intégrée, non remplaçable. La capacité maximale dépend du modèle ; elle n’est pas déduite de la capacité installée.".into());
    memory
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn platform_memory() -> MemoryInventory {
    MemoryInventory::default()
}

pub fn collect() -> Inventory {
    let mut system = System::new();
    system.refresh_memory();
    system.refresh_cpu_all();
    let mut memory = platform_memory();
    memory.total_bytes = system.total_memory();
    memory.available_bytes = system.available_memory();
    memory.test_budget_bytes = crate::domain::safe_memory_budget(
        memory.total_bytes,
        memory.available_bytes,
        system.cgroup_limits().map(|limits| limits.free_memory),
    );
    memory.notes.push("Capacité maximale : déclaration du firmware, à confirmer avec la documentation du fabricant.".into());
    memory.notes.push("La vitesse nominale d’une barrette ne prouve pas la vitesse maximale prise en charge par la carte mère et le CPU. Les débits DDR sont exprimés en MT/s.".into());
    memory.notes.push("Le format DIMM / SODIMM indique un module généralement remplaçable ; confirmez l’accès physique dans le manuel du modèle.".into());
    let cpu = system.cpus().first();
    let cpu = CpuInventory {
        name: cpu.map(|c| c.brand().into()).unwrap_or_default(),
        vendor: cpu.map(|c| c.vendor_id().into()).unwrap_or_default(),
        frequency_mhz: cpu.map(|c| c.frequency()).unwrap_or(0),
        logical_cores: system.cpus().len(),
        physical_cores: System::physical_core_count(),
        usage_percent: None,
    };
    let disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|d| DiskInventory {
            name: d.name().to_string_lossy().into_owned(),
            mount_point: d.mount_point().to_string_lossy().into_owned(),
            file_system: d.file_system().to_string_lossy().into_owned(),
            kind: format!("{:?}", d.kind()),
            total_bytes: d.total_space(),
            available_bytes: d.available_space(),
            removable: d.is_removable(),
        })
        .filter(|d| d.total_bytes > 0)
        .collect();
    let temperatures = Components::new_with_refreshed_list()
        .iter()
        .map(|c| Temperature {
            name: c.label().into(),
            celsius: c.temperature(),
            critical_celsius: c.critical(),
        })
        .collect();
    let model = {
        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/sys/class/dmi/id/product_name")
                .unwrap_or_default()
                .trim()
                .to_owned()
        }
        #[cfg(target_os = "macos")]
        {
            command(
                "/usr/sbin/sysctl",
                &["-n", "hw.model"],
                Duration::from_secs(2),
            )
            .map(|o| o.stdout.trim().into())
            .unwrap_or_default()
        }
        #[cfg(target_os = "windows")]
        {
            command(
                "powershell.exe",
                &[
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "(Get-CimInstance Win32_ComputerSystem).Model",
                ],
                Duration::from_secs(5),
            )
            .map(|o| o.stdout.trim().into())
            .unwrap_or_default()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            String::new()
        }
    };
    let host_name = System::host_name().unwrap_or_else(|| "Cet ordinateur".into());
    let machine_id = format!(
        "{:x}",
        Sha256::digest(format!("{host_name}|{model}|{}", cpu.name).as_bytes())
    )[..16]
        .to_owned();
    let runtime = if std::env::var_os("COMPUTY_CONTAINER").is_some()
        || std::path::Path::new("/.dockerenv").exists()
    {
        "container"
    } else {
        "native"
    }
    .to_owned();
    Inventory { runtime, captured_at: crate::domain::timestamp_ms(), machine_id, host_name, model, os: System::name().unwrap_or_default(), os_version: System::os_version().unwrap_or_default(),
        arch: std::env::consts::ARCH.into(), kernel: System::kernel_version().unwrap_or_default(), uptime_seconds: System::uptime(), memory, cpu, disks, temperatures,
        smart_available: command("smartctl", &["--version"], Duration::from_secs(2)).is_ok_and(|o| o.code == 0),
        notes: vec!["Les informations décrivent la machine qui exécute le moteur natif. Dans une VM ou un conteneur, le matériel visible peut être virtuel.".into()] }
}
