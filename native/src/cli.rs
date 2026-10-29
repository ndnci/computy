use crate::{
    domain::{Report, RunOptions, Status},
    mcp::{self, McpServer, Permissions},
    preferences,
    repository::FileRepository,
    service::{AppService, SystemHardware},
    transport,
};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Read, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

#[derive(Debug, Parser)]
#[command(
    name = "computy-cli",
    version,
    about = "Computy — native diagnostics, local history and MCP. NDNCI · https://www.ndnci.com",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Shared reports/preferences directory (defaults to the desktop app's OS data directory).
    #[arg(long, global = true)]
    pub data_dir: Option<PathBuf>,
    /// Machine-readable JSON. Progress is always written to stderr.
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Commands,
}
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Read actual hardware, firmware information and visible sensors.
    Inventory,
    /// Passive CPU/RAM/GPU/disk telemetry. --watch streams JSON until Ctrl+C.
    Monitor {
        #[arg(long)]
        watch: bool,
    },
    /// Run bounded RAM/storage/CPU checks. Ctrl+C cancels and saves partial results.
    Diagnose {
        #[arg(long, value_delimiter = ',', default_value = "memory,storage,cpu")]
        components: Vec<String>,
        #[arg(long)]
        memory_mib: Option<usize>,
        #[arg(long)]
        passes: Option<usize>,
        #[arg(long)]
        disk_mib: Option<usize>,
        #[arg(long)]
        cpu_seconds: Option<u64>,
        #[arg(long, default_value = "")]
        label: String,
        #[arg(long, value_enum, default_value = "standard")]
        profile: Profile,
    },
    /// List saved reports.
    History,
    /// Show all measurements and limitations of a report.
    Show {
        id: String,
    },
    /// Compare 2–4 snapshots. The first ID is the baseline.
    Compare {
        #[arg(num_args=2..=4)]
        ids: Vec<String>,
    },
    /// Export JSON to stdout or a file. Existing files require --force.
    Export {
        id: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    Rename {
        id: String,
        label: String,
    },
    /// Permanently delete a report (requires --yes).
    Delete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
    /// Read/update settings shared with the UI.
    Settings {
        #[command(subcommand)]
        action: SettingsAction,
    },
    /// Post-boot diagnostic preparation/import (Windows), or platform status.
    Boot {
        #[command(subcommand)]
        action: BootAction,
    },
    /// Manage the desktop app's login startup entry without opening its UI.
    Startup {
        #[arg(value_enum)]
        action: StartupAction,
        #[arg(long)]
        desktop_path: Option<PathBuf>,
    },
    /// MCP server. Read-only by default; HTTP access is temporary and authenticated.
    Mcp {
        #[arg(long)]
        http: bool,
        #[arg(long, default_value_t = 8765)]
        port: u16,
        #[arg(long, default_value_t = 900)]
        ttl: u64,
        #[arg(long)]
        token_file: Option<PathBuf>,
        #[arg(long)]
        allow_diagnostics: bool,
        #[arg(long)]
        allow_history_write: bool,
        #[arg(long)]
        allow_origin: Vec<String>,
        /// Publish via an installed cloudflared. Explicit opt-in; no download or account creation.
        #[arg(long, requires = "http")]
        cloudflare_tunnel: bool,
    },
    #[command(hide = true)]
    Rpc,
}
#[derive(Debug, Clone, ValueEnum)]
pub enum Profile {
    Quick,
    Standard,
    Extended,
}
#[derive(Debug, Subcommand)]
pub enum SettingsAction {
    Show,
    Set {
        #[arg(long)]
        memory_mib: Option<usize>,
        #[arg(long)]
        passes: Option<usize>,
        #[arg(long)]
        disk_mib: Option<usize>,
        #[arg(long)]
        cpu_seconds: Option<u64>,
        #[arg(long)]
        theme: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        tray_enabled: Option<bool>,
        /// Comma-separated memory,cpu,gpu; an empty string hides all three.
        #[arg(long)]
        tray_components: Option<String>,
        /// Comma-separated mount points; an empty string clears selected volumes.
        #[arg(long)]
        tray_disks: Option<String>,
    },
}
#[derive(Debug, Subcommand)]
pub enum BootAction {
    Status,
    Prepare,
    Collect,
    Forget,
    Verify,
    Cancel,
    Reboot {
        #[arg(long)]
        yes: bool,
    },
    UsbList,
    UsbPrepare {
        #[arg(long)]
        id: String,
        #[arg(long)]
        yes: bool,
    },
}
#[derive(Debug, Clone, ValueEnum)]
pub enum StartupAction {
    Status,
    Enable,
    Disable,
}

pub fn run(cli: Cli) -> Result<i32, String> {
    let root = cli.data_dir.unwrap_or_else(preferences::default_data_dir);
    let repository = Arc::new(FileRepository::new(root.join("reports"))?);
    let service = AppService::new(repository, Arc::new(SystemHardware), root.join("scratch"));
    let result = match cli.command {
        Commands::Monitor { watch } => {
            let running = Arc::new(AtomicBool::new(true));
            let signal = running.clone();
            if watch {
                ctrlc::set_handler(move || {
                    signal.store(false, Ordering::Relaxed);
                })
                .map_err(|e| e.to_string())?;
            }
            // Seed the counters; the second sample has an actual CPU interval.
            service.monitor(true);
            thread::sleep(Duration::from_secs(1));
            loop {
                print_json(&service.monitor(true))?;
                if !watch || !running.load(Ordering::Relaxed) {
                    break;
                }
                thread::sleep(Duration::from_secs(2));
                if !running.load(Ordering::Relaxed) {
                    break;
                }
            }
            0
        }
        Commands::Inventory => {
            let value = service.inventory(true);
            if cli.json {
                print_json(&value)?;
            } else {
                println!(
                    "Computy · {} · {} {}",
                    value.host_name, value.os, value.os_version
                );
                println!(
                    "CPU    {} · {} logical cores",
                    value.cpu.name, value.cpu.logical_cores
                );
                println!(
                    "RAM    {} visible · {} available",
                    bytes(value.memory.total_bytes),
                    bytes(value.memory.available_bytes)
                );
                println!(
                    "MAX    {}",
                    value
                        .memory
                        .max_capacity_bytes
                        .map(bytes)
                        .unwrap_or_else(|| "Not reported by firmware".into())
                );
                for disk in value.disks {
                    println!(
                        "DISK   {} · {} · {} free",
                        disk.mount_point,
                        bytes(disk.total_bytes),
                        bytes(disk.available_bytes)
                    );
                }
                println!("Use --json for modules, sources, limits and sensors.");
            }
            0
        }
        Commands::Diagnose {
            components,
            memory_mib,
            passes,
            disk_mib,
            cpu_seconds,
            label,
            profile,
        } => {
            let settings = preferences::load(&root)?;
            let base = match profile {
                Profile::Quick => RunOptions {
                    memory_mib: 64,
                    passes: 1,
                    disk_mib: 16,
                    cpu_seconds: 2,
                    ..Default::default()
                },
                Profile::Extended => RunOptions {
                    memory_mib: 512,
                    passes: 4,
                    disk_mib: 128,
                    cpu_seconds: 15,
                    ..Default::default()
                },
                Profile::Standard => RunOptions {
                    memory_mib: settings.memory_mib,
                    passes: settings.passes,
                    disk_mib: settings.disk_mib,
                    cpu_seconds: settings.cpu_seconds,
                    ..Default::default()
                },
            };
            let options = RunOptions {
                components,
                memory_mib: memory_mib.unwrap_or(base.memory_mib),
                passes: passes.unwrap_or(base.passes),
                disk_mib: disk_mib.unwrap_or(base.disk_mib),
                cpu_seconds: cpu_seconds.unwrap_or(base.cpu_seconds),
                label,
            };
            let interrupted = Arc::new(AtomicBool::new(false));
            let flag = interrupted.clone();
            ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed))
                .map_err(|e| e.to_string())?;
            let id = service.start(options)?;
            let mut previous = String::new();
            let mut cancelled = false;
            let report = loop {
                let job = service.job(&id)?;
                if interrupted.load(Ordering::Relaxed) && job.running && !cancelled {
                    let _ = service.cancel(&id);
                    cancelled = true;
                }
                let progress = format!("{:3}% · {}", (job.progress * 100.0) as u8, job.phase);
                if progress != previous {
                    eprintln!("{progress}");
                    previous = progress;
                }
                if !job.running {
                    if let Some(error) = job.error {
                        return Err(error);
                    }
                    break job.report.ok_or("No report was generated")?;
                }
                thread::sleep(Duration::from_millis(150));
            };
            if cli.json {
                print_json(&report)?;
            } else {
                print_report(&report);
            }
            match report.summary.status {
                Status::Failed => 2,
                Status::Cancelled => 130,
                Status::Unavailable => 3,
                _ => 0,
            }
        }
        Commands::History => {
            let list = service.repository.list()?;
            if cli.json {
                print_json(&list)?;
            } else {
                println!("ID                               SCORE   COVERAGE   LABEL");
                for r in list.reports {
                    println!(
                        "{:<32} {:<7} {}/{}         {}",
                        r.id,
                        r.summary
                            .score
                            .map(|s| format!("{s}/10"))
                            .unwrap_or_else(|| "—".into()),
                        r.summary.checked,
                        r.summary.total,
                        r.label
                    );
                }
                if list.unreadable > 0 {
                    eprintln!("{} unreadable/altered report(s) excluded", list.unreadable);
                }
            }
            0
        }
        Commands::Show { id } => {
            let report: Report =
                serde_json::from_value(service.repository.get(&id)?).map_err(|e| e.to_string())?;
            if cli.json {
                print_json(&report)?;
            } else {
                print_report(&report);
            }
            0
        }
        Commands::Compare { ids } => {
            let value = crate::comparison::compare(service.repository.as_ref(), &ids)?;
            if cli.json {
                print_json(&value)?;
            } else {
                for warning in value["warnings"].as_array().unwrap() {
                    eprintln!("Warning: {}", warning.as_str().unwrap_or(""));
                }
                for r in value["reports"].as_array().unwrap() {
                    println!(
                        "{} · {} · RAM {} · score {}",
                        r["id"].as_str().unwrap_or(""),
                        r["label"].as_str().unwrap_or(""),
                        bytes(
                            r["inventory"]["memory"]["total_bytes"]
                                .as_u64()
                                .unwrap_or(0)
                        ),
                        r["summary"]["score"]
                    );
                }
                println!(
                    "Capacity and speed changes do not prove better health. Use --json for all snapshots and measurements."
                );
            }
            0
        }
        Commands::Export { id, output, force } => {
            let report = service.repository.get(&id)?;
            if let Some(path) = output {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(force)
                    .create_new(!force)
                    .open(path)
                    .map_err(|e| e.to_string())?;
                file.write_all(&serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            } else {
                print_json(&report)?;
            }
            0
        }
        Commands::Rename { id, label } => {
            service.repository.rename(&id, &label)?;
            print_json(&json!({"renamed":id,"label":label}))?;
            0
        }
        Commands::Delete { id, yes } => {
            if !yes {
                return Err(
                    "Deletion requires explicit --yes. Export a copy first if needed.".into(),
                );
            }
            service.repository.delete(&id)?;
            print_json(&json!({"deleted":id}))?;
            0
        }
        Commands::Settings { action } => {
            match action {
                SettingsAction::Show => print_json(&preferences::load(&root)?)?,
                SettingsAction::Set {
                    memory_mib,
                    passes,
                    disk_mib,
                    cpu_seconds,
                    theme,
                    language,
                    tray_enabled,
                    tray_components,
                    tray_disks,
                } => {
                    let mut value = serde_json::to_value(preferences::load(&root)?)
                        .map_err(|e| e.to_string())?;
                    for (key, item) in [
                        ("memory_mib", memory_mib.map(|v| json!(v))),
                        ("passes", passes.map(|v| json!(v))),
                        ("disk_mib", disk_mib.map(|v| json!(v))),
                        ("cpu_seconds", cpu_seconds.map(|v| json!(v))),
                        ("theme", theme.map(|v| json!(v))),
                        ("language", language.map(|v| json!(v))),
                        ("tray_enabled", tray_enabled.map(|v| json!(v))),
                        (
                            "tray_components",
                            tray_components.map(|v| {
                                json!(v.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>())
                            }),
                        ),
                        (
                            "tray_disks",
                            tray_disks.map(|v| {
                                json!(v.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>())
                            }),
                        ),
                    ] {
                        if let Some(item) = item {
                            value[key] = item;
                        }
                    }
                    let settings = serde_json::from_value(value).map_err(|e| e.to_string())?;
                    preferences::save(&root, &settings)?;
                    print_json(&settings)?;
                }
            }
            0
        }
        Commands::Boot { action } => {
            match action {
                #[cfg(target_os = "linux")]
                BootAction::Verify => print_json(&crate::linux_boot::request("verify")?)?,
                #[cfg(target_os = "linux")]
                BootAction::Cancel => print_json(&crate::linux_boot::request("cancel")?)?,
                #[cfg(target_os = "linux")]
                BootAction::UsbList => print_json(&json!(crate::usb_boot::targets()?))?,
                #[cfg(target_os = "linux")]
                BootAction::UsbPrepare { id, yes } => {
                    if !yes {
                        return Err("USB preparation requires explicit --yes after selecting a target from `boot usb-list`.".into());
                    }
                    print_json(&json!(crate::usb_boot::prepare(&id)?))?;
                }
                #[cfg(target_os = "linux")]
                BootAction::Reboot { yes } => {
                    if !yes {
                        return Err("Reboot requires explicit --yes. Save your work first.".into());
                    }
                    crate::linux_boot::reboot()?;
                }
                #[cfg(not(target_os = "linux"))]
                BootAction::Verify
                | BootAction::Cancel
                | BootAction::UsbList
                | BootAction::UsbPrepare { .. }
                | BootAction::Reboot { .. } => {
                    return Err("This boot integration is available on Linux.".into());
                }
                BootAction::Status => print_json(&crate::offline::status(&root)?)?,
                BootAction::Collect => print_json(&crate::offline::collect(&service)?)?,
                #[cfg(target_os = "linux")]
                BootAction::Forget => print_json(&crate::linux_boot::request("cancel")?)?,
                #[cfg(not(target_os = "linux"))]
                BootAction::Forget => print_json(&crate::offline::forget(&root)?)?,
                BootAction::Prepare => {
                    crate::offline::prepare(&service)?;
                    #[cfg(target_os = "windows")]
                    {
                        std::process::Command::new("mdsched.exe")
                            .spawn()
                            .map_err(|e| e.to_string())?;
                    }
                    eprintln!(
                        "Boot diagnostic prepared. On Windows, choose when to restart in the system dialog. On Linux, the next boot selects Computy once; the on-screen result is not automatically imported."
                    );
                }
            }
            0
        }
        Commands::Mcp {
            http,
            port,
            ttl,
            token_file,
            allow_diagnostics,
            allow_history_write,
            allow_origin,
            cloudflare_tunnel,
        } => {
            let server = McpServer::new(
                service.clone(),
                Permissions {
                    diagnostics: allow_diagnostics,
                    history_write: allow_history_write,
                },
            );
            if http {
                let token_file = token_file
                    .unwrap_or_else(|| root.join(format!("mcp-{}.token", crate::domain::new_id())));
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?
                    .block_on(mcp::serve_http(
                        server,
                        mcp::HttpConfig {
                            port,
                            ttl_seconds: ttl,
                            token_file,
                            allowed_origins: allow_origin,
                            cloudflare_tunnel,
                        },
                    ))?;
            } else {
                mcp::serve_stdio(server)?;
            }
            0
        }
        Commands::Startup {
            action,
            desktop_path,
        } => {
            if matches!(action, StartupAction::Enable) && desktop_path.is_none() {
                return Err("Enabling login startup requires --desktop-path pointing to the installed Computy application.".into());
            }
            let executable =
                desktop_path.unwrap_or(std::env::current_exe().map_err(|e| e.to_string())?);
            if !executable.is_absolute() || !executable.exists() {
                return Err("The application path must exist and be absolute.".into());
            }
            let entry = auto_launch::AutoLaunchBuilder::new()
                .set_app_name("Computy")
                .set_app_path(&executable.to_string_lossy())
                .set_use_launch_agent(true)
                .set_args(&["--return-from-diagnostic"])
                .build()
                .map_err(|e| e.to_string())?;
            match action {
                StartupAction::Enable => entry.enable().map_err(|e| e.to_string())?,
                StartupAction::Disable => entry.disable().map_err(|e| e.to_string())?,
                StartupAction::Status => {}
            }
            print_json(&json!({"enabled":entry.is_enabled().map_err(|e|e.to_string())?}))?;
            0
        }
        Commands::Rpc => {
            serve_rpc(&service)?;
            0
        }
    };
    service.shutdown();
    Ok(result)
}
fn print_json(value: &impl serde::Serialize) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
    );
    Ok(())
}
fn print_report(report: &Report) {
    println!("\n{} · {}", report.label, report.id);
    println!(
        "Score: {} · {}/{} controls usable · completed: {}",
        report
            .summary
            .score
            .map(|s| format!("{s}/10"))
            .unwrap_or_else(|| "not scored".into()),
        report.summary.checked,
        report.summary.total,
        report.completed
    );
    for c in &report.checks {
        println!(
            "\n{:?} · {}\n  {}\n  Measurements: {}",
            c.status, c.name, c.message, c.metrics
        );
        for limit in &c.limitations {
            println!("  Scope: {limit}");
        }
    }
    println!("\nNo error observed is not a guarantee against future or intermittent faults.");
}
fn bytes(value: u64) -> String {
    format!("{:.1} GiB", value as f64 / 1024f64.powi(3))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: String,
    method: String,
    #[serde(default)]
    params: Value,
}
fn serve_rpc(service: &Arc<AppService>) -> Result<(), String> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    loop {
        let mut line = String::new();
        if input
            .by_ref()
            .take(65537)
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            break;
        }
        if line.len() > 65536 {
            return Err("Request too large".into());
        }
        let response = match serde_json::from_str::<Request>(&line) {
            Ok(r) => match transport::dispatch(service, &r.method, &r.params) {
                Ok(result) => json!({"id":r.id,"result":result}),
                Err(error) => json!({"id":r.id,"error":error}),
            },
            Err(_) => json!({"id":null,"error":"Invalid JSON request"}),
        };
        writeln!(output, "{response}")
            .and_then(|_| output.flush())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
