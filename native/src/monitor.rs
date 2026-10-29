//! Passive telemetry. No stress test, administrator rights or health score.
use serde::Serialize;
use std::time::{Duration, Instant};
use sysinfo::{Disks, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Debug, Clone, Serialize, Default)]
pub struct ProcessSample {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: Option<u64>,
    pub memory_percent: Option<f64>,
    pub cpu_percent: Option<f64>,
    pub read_bytes_per_second: Option<f64>,
    pub written_bytes_per_second: Option<f64>,
    pub io_percent: Option<f64>,
    pub gpu_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DiskSample {
    pub id: String,
    pub name: String,
    pub used_percent: Option<f64>,
    pub read_bytes_per_second: Option<f64>,
    pub written_bytes_per_second: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct Snapshot {
    pub captured_at: u64,
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub gpu_percent: Option<f64>,
    pub gpu_source: String,
    pub disks: Vec<DiskSample>,
    pub processes: Vec<ProcessSample>,
}
pub struct Sampler {
    system: System,
    disks: Disks,
    sampled: Option<Instant>,
    gpu_sampled: Option<Instant>,
    snapshot: Snapshot,
    gpu_processes: std::collections::BTreeMap<(u32, u64), u64>,
}
impl Default for Sampler {
    fn default() -> Self {
        Self {
            system: System::new(),
            disks: Disks::new(),
            sampled: None,
            gpu_sampled: None,
            snapshot: Snapshot::default(),
            gpu_processes: Default::default(),
        }
    }
}
impl Sampler {
    pub fn sample(&mut self, gpu_enabled: bool) -> Snapshot {
        let now = Instant::now();
        if self
            .sampled
            .is_some_and(|last| now.duration_since(last) < Duration::from_secs(1))
        {
            return self.snapshot.clone();
        }
        let elapsed = self
            .sampled
            .map(|last| now.duration_since(last).as_secs_f64());
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .without_tasks()
                .with_cpu()
                .with_memory()
                .with_disk_usage(),
        );
        let cores = self.system.cpus().len().max(1) as f64;
        self.snapshot.processes = self
            .system
            .processes()
            .values()
            .map(|process| {
                let usage = process.disk_usage();
                #[cfg(target_os = "linux")]
                let io_readable =
                    std::fs::File::open(format!("/proc/{}/io", process.pid())).is_ok();
                #[cfg(not(target_os = "linux"))]
                let io_readable = process.memory() > 0;
                ProcessSample {
                    pid: process.pid().as_u32(),
                    name: process.name().to_string_lossy().into_owned(),
                    memory_bytes: (process.memory() > 0).then_some(process.memory()),
                    memory_percent: (process.memory() > 0)
                        .then(|| ratio(process.memory(), self.system.total_memory()))
                        .flatten(),
                    cpu_percent: elapsed.and_then(|_| {
                        valid_percent((f64::from(process.cpu_usage()) / cores).min(100.0))
                    }),
                    read_bytes_per_second: elapsed
                        .filter(|_| io_readable)
                        .map(|seconds| usage.read_bytes as f64 / seconds),
                    written_bytes_per_second: elapsed
                        .filter(|_| io_readable)
                        .map(|seconds| usage.written_bytes as f64 / seconds),
                    gpu_memory_bytes: self
                        .gpu_processes
                        .get(&(process.pid().as_u32(), process.start_time()))
                        .copied(),
                    ..ProcessSample::default()
                }
            })
            .collect();
        let total_io: f64 = self
            .snapshot
            .processes
            .iter()
            .filter_map(|p| Some(p.read_bytes_per_second? + p.written_bytes_per_second?))
            .sum();
        for process in &mut self.snapshot.processes {
            process.io_percent = process
                .read_bytes_per_second
                .zip(process.written_bytes_per_second)
                .map(|(r, w)| {
                    if total_io > 0.0 {
                        (r + w) / total_io * 100.0
                    } else {
                        0.0
                    }
                });
        }
        self.disks.refresh(true);
        self.snapshot.cpu_percent =
            elapsed.and_then(|_| valid_percent(f64::from(self.system.global_cpu_usage())));
        self.snapshot.memory_percent = ratio(self.system.used_memory(), self.system.total_memory());
        self.snapshot.disks = self
            .disks
            .list()
            .iter()
            .filter(|disk| disk.total_space() > 0)
            .map(|disk| {
                let usage = disk.usage();
                DiskSample {
                    id: disk.mount_point().to_string_lossy().into_owned(),
                    name: disk.name().to_string_lossy().into_owned(),
                    used_percent: ratio(
                        disk.total_space().saturating_sub(disk.available_space()),
                        disk.total_space(),
                    ),
                    read_bytes_per_second: elapsed.map(|seconds| usage.read_bytes as f64 / seconds),
                    written_bytes_per_second: elapsed
                        .map(|seconds| usage.written_bytes as f64 / seconds),
                }
            })
            .collect();
        // External driver queries are bounded and run at most once every six seconds.
        if gpu_enabled
            && self
                .gpu_sampled
                .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(6))
        {
            let (value, source) = gpu_usage();
            self.snapshot.gpu_percent = value;
            self.snapshot.gpu_source = source.into();
            self.gpu_processes.clear();
            for process in &mut self.snapshot.processes {
                process.gpu_memory_bytes = None;
            }
            // NVIDIA exposes compute-process VRAM separately from GPU activity.
            // Never present VRAM as a GPU utilization percentage.
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                #[cfg(target_os = "linux")]
                let nvidia = "/usr/bin/nvidia-smi".to_owned();
                #[cfg(target_os = "windows")]
                let nvidia = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into())
                    + "\\System32\\nvidia-smi.exe";
                if let Some(text) = query(
                    &nvidia,
                    &[
                        "--query-compute-apps=pid,used_gpu_memory",
                        "--format=csv,noheader,nounits",
                    ],
                ) {
                    for (pid, bytes) in nvidia_process_memory(&text) {
                        if let Some(process) =
                            self.snapshot.processes.iter_mut().find(|p| p.pid == pid)
                        {
                            process.gpu_memory_bytes = Some(bytes);
                            if let Some(native) = self.system.process(sysinfo::Pid::from_u32(pid)) {
                                self.gpu_processes.insert((pid, native.start_time()), bytes);
                            }
                        }
                    }
                }
            }
            self.gpu_sampled = Some(now);
        } else if !gpu_enabled {
            self.gpu_processes.clear();
            for process in &mut self.snapshot.processes {
                process.gpu_memory_bytes = None;
            }
            self.snapshot.gpu_percent = None;
            self.snapshot.gpu_source.clear();
            self.gpu_sampled = None;
        }
        self.snapshot.captured_at = crate::domain::timestamp_ms();
        self.sampled = Some(now);
        self.snapshot.clone()
    }
}
fn valid_percent(value: f64) -> Option<f64> {
    (value.is_finite() && (0.0..=100.0).contains(&value)).then_some(value)
}
pub fn process_value(process: &ProcessSample, metric: &str) -> Option<f64> {
    match metric {
        "memory" => process.memory_bytes.map(|n| n as f64),
        "cpu" => process.cpu_percent,
        "gpu" => process.gpu_memory_bytes.map(|n| n as f64),
        "storage" => Some(process.read_bytes_per_second? + process.written_bytes_per_second?),
        _ => None,
    }
}
pub fn top_processes<'a>(snapshot: &'a Snapshot, metric: &str) -> Vec<&'a ProcessSample> {
    let mut processes: Vec<_> = snapshot
        .processes
        .iter()
        .filter(|p| process_value(p, metric).is_some())
        .collect();
    processes.sort_by(|a, b| {
        process_value(b, metric)
            .unwrap()
            .total_cmp(&process_value(a, metric).unwrap())
            .then_with(|| a.pid.cmp(&b.pid))
    });
    processes.truncate(10);
    processes
}
fn ratio(used: u64, total: u64) -> Option<f64> {
    (total > 0).then(|| (used.min(total) as f64 / total as f64) * 100.0)
}
pub fn parse_percent(value: &str) -> Option<f64> {
    valid_percent(value.trim().parse().ok()?)
}
/// Local desktop icon lookup only. Executable paths never enter telemetry or MCP payloads.
pub fn process_executable(pid: u32) -> Option<std::path::PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_link(format!("/proc/{pid}/exe")).ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut system = System::new();
        let pid = sysinfo::Pid::from_u32(pid);
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing()
                .without_tasks()
                .with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
        );
        system.process(pid)?.exe().map(std::path::Path::to_path_buf)
    }
}
pub fn nvidia_percent(value: &str) -> Option<f64> {
    value
        .lines()
        .filter_map(parse_percent)
        .max_by(f64::total_cmp)
}
pub fn nvidia_process_memory(text: &str) -> std::collections::BTreeMap<u32, u64> {
    let mut values = std::collections::BTreeMap::<u32, u64>::new();
    for line in text.lines() {
        if let Some((pid, mib)) = line.split_once(',')
            && let (Ok(pid), Ok(mib)) = (pid.trim().parse::<u32>(), mib.trim().parse::<u64>())
            && let Some(bytes) = mib.checked_mul(1024 * 1024)
            && let Some(total) = values.get(&pid).copied().unwrap_or(0).checked_add(bytes)
        {
            values.insert(pid, total);
        }
    }
    values
}
fn query(program: &str, args: &[&str]) -> Option<String> {
    crate::inventory::command(program, args, Duration::from_millis(1500))
        .ok()
        .filter(|out| out.code == 0)
        .map(|out| out.stdout)
}
fn gpu_usage() -> (Option<f64>, &'static str) {
    #[cfg(target_os = "linux")]
    {
        let mut values = vec![];
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name
                    .strip_prefix("card")
                    .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                    && let Ok(text) =
                        std::fs::read_to_string(entry.path().join("device/gpu_busy_percent"))
                    && let Some(value) = parse_percent(&text)
                {
                    values.push(value);
                }
            }
        }
        // Fixed executable locations; no shell, sudo, downloads or user-provided command.
        if let Some(text) = query(
            "/usr/bin/nvidia-smi",
            &[
                "--query-gpu=utilization.gpu",
                "--format=csv,noheader,nounits",
            ],
        ) && let Some(value) = nvidia_percent(&text)
        {
            values.push(value);
        }
        (
            values.into_iter().max_by(f64::total_cmp),
            "GPU · sysfs / NVIDIA",
        )
    }
    #[cfg(target_os = "windows")]
    {
        let root = std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "C:\\Windows".into());
        let program = root.join("System32/WindowsPowerShell/v1.0/powershell.exe");
        // Sum processes per engine, then select the busiest engine, never sum engines.
        let script = "$v=Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine -ErrorAction Stop | Group-Object { $_.Name -replace '^pid_\\d+_','' } | ForEach-Object { ($_.Group | Measure-Object UtilizationPercentage -Sum).Sum } | Measure-Object -Maximum; if($null -ne $v.Maximum){[Console]::Write([Math]::Min(100,$v.Maximum).ToString([Globalization.CultureInfo]::InvariantCulture))}";
        (
            program
                .to_str()
                .and_then(|p| query(p, &["-NoProfile", "-NonInteractive", "-Command", script]))
                .and_then(|text| parse_percent(&text)),
            "GPU · Windows performance counters",
        )
    }
    #[cfg(target_os = "macos")]
    {
        let value = query(
            "/usr/sbin/ioreg",
            &["-r", "-d", "1", "-l", "-c", "IOAccelerator"],
        )
        .and_then(|text| mac_gpu_percent(&text));
        (value, "GPU · IOAccelerator")
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        (None, "")
    }
}
#[cfg(any(test, target_os = "macos"))]
pub fn mac_gpu_percent(text: &str) -> Option<f64> {
    // Driver-specific counters are optional. Frequency and VRAM usage are not load.
    ["\"Device Utilization %\"=", "\"GPU Activity(%)\"="]
        .iter()
        .flat_map(|key| text.split(key).skip(1))
        .filter_map(|part| {
            let number: String = part
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            parse_percent(&number)
        })
        .max_by(f64::total_cmp)
}
