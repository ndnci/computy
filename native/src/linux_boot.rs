//! Conservative GRUB one-shot integration. No firmware changes, shell, raw disk I/O,
//! bootloader installation, permanent default changes or inferred Memtest results.
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use std::{path::Path, time::Duration};

pub const HELPER: &str = "/usr/libexec/computy-boot-helper";
#[cfg(target_os = "linux")]
const REQUEST: &str = "/var/lib/computy/boot-request.json";
#[cfg(all(target_os = "linux", feature = "boot-helper"))]
const ENVIRONMENT: &str = "/boot/grub/grubenv";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Facts {
    pub architecture: String,
    pub container: bool,
    pub uefi: bool,
    pub secure_boot: Option<bool>,
    pub helper_installed: bool,
    pub payload_installed: bool,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub facts: Facts,
    pub grub_config: String,
    pub filesystem: String,
    pub abstractions: String,
    pub environment: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    pub owner_uid: u32,
    pub requested_at: u64,
    pub boot_id: String,
    #[serde(default)]
    pub scheduled: bool,
}
pub fn assess(facts: &Facts) -> &'static str {
    if facts.container {
        "container"
    } else if facts.architecture != "x86_64" {
        "architecture"
    } else if facts.uefi && facts.secure_boot == Some(true) {
        "secure_boot"
    } else if facts.uefi && facts.secure_boot.is_none() {
        "secure_boot_unknown"
    } else if !facts.helper_installed {
        "helper_missing"
    } else if !facts.payload_installed {
        "payload_missing"
    } else {
        "ready"
    }
}
fn environment_value<'a>(environment: &'a str, key: &str) -> Option<&'a str> {
    environment
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .filter(|value| !value.is_empty())
}
pub fn memtest_entry(config: &str) -> Option<&'static str> {
    let mut depth = 0usize;
    for line in config.lines() {
        let line = line.trim();
        if depth == 0
            && line.starts_with("menuentry ")
            && line.contains(" --class computy ")
            && (line.ends_with("'computy-memory-test' {")
                || line.ends_with("\"computy-memory-test\" {"))
        {
            return Some("computy-memory-test");
        }
        // Count only syntactic braces; ignore comments, quoted text and ${variables}.
        let mut quote = None;
        let mut escaped = false;
        let mut variable = false;
        for character in line.chars() {
            if escaped {
                escaped = false;
                continue;
            }
            if character == '\\' {
                escaped = true;
                continue;
            }
            if let Some(active) = quote {
                if character == active {
                    quote = None;
                }
                continue;
            }
            if character == '#' {
                break;
            }
            if character == '\'' || character == '"' {
                quote = Some(character);
                continue;
            }
            if character == '$' {
                variable = true;
                continue;
            }
            if character == '{' {
                if !variable {
                    depth += 1;
                }
            } else if character == '}' {
                if !variable {
                    depth = depth.saturating_sub(1);
                }
                variable = false;
            } else if !character.is_alphanumeric() && character != '_' {
                variable = false;
            }
        }
    }
    None
}
pub fn validate_snapshot(snapshot: &Snapshot) -> Result<(), String> {
    let code = assess(&snapshot.facts);
    if code != "ready" {
        return Err(format!("boot:{code}"));
    }
    if snapshot.filesystem.trim() != "ext2" || !snapshot.abstractions.trim().is_empty() {
        return Err("boot:unsupported_boot_storage".into());
    }
    let active: String = snapshot
        .grub_config
        .lines()
        .map(|line| line.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    if ![
        "load_env",
        "${next_entry}",
        "set next_entry=",
        "save_env next_entry",
    ]
    .iter()
    .all(|fragment| active.contains(fragment))
    {
        return Err("boot:unsupported_grub_header".into());
    }
    if memtest_entry(&snapshot.grub_config).is_none() {
        return Err("boot:entry_missing".into());
    }
    if environment_value(&snapshot.environment, "next_entry").is_some() {
        return Err("boot:another_boot_pending".into());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn trusted(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    path.ancestors().all(|part| {
        std::fs::symlink_metadata(part)
            .is_ok_and(|m| m.uid() == 0 && m.mode() & 0o022 == 0 && !m.file_type().is_symlink())
    })
}
#[cfg(target_os = "linux")]
pub fn facts() -> Facts {
    let uefi = Path::new("/sys/firmware/efi").is_dir();
    let mut secure_boot = if uefi {
        std::fs::read("/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c")
            .ok()
            .filter(|bytes| bytes.len() == 5)
            .and_then(|bytes| match bytes[4] {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            })
    } else {
        Some(false)
    };
    if uefi && secure_boot.is_none() {
        secure_boot =
            crate::inventory::command("/usr/bin/mokutil", &["--sb-state"], Duration::from_secs(3))
                .ok()
                .and_then(|output| {
                    if output.code != 0 {
                        None
                    } else if output.stdout.contains("SecureBoot enabled") {
                        Some(true)
                    } else if output.stdout.contains("SecureBoot disabled") {
                        Some(false)
                    } else {
                        None
                    }
                });
    }
    Facts {
        architecture: std::env::consts::ARCH.into(),
        container: Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists(),
        uefi,
        secure_boot,
        helper_installed: Path::new(HELPER).is_file() && trusted(Path::new(HELPER)),
        payload_installed: Path::new("/boot/mt86+x64").is_file()
            || Path::new("/boot/computy-memory-testx64.bin").is_file(),
    }
}
#[cfg(target_os = "linux")]
fn boot_id() -> Result<String, String> {
    let id =
        std::fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(|e| e.to_string())?;
    let id = id.trim();
    if id.len() != 36 || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return Err("boot:invalid_boot_id".into());
    }
    Ok(id.into())
}
#[cfg(target_os = "linux")]
fn read_request() -> Result<Option<Request>, String> {
    let path = Path::new(REQUEST);
    if !path.try_exists().map_err(|e| e.to_string())? {
        return Ok(None);
    }
    if !trusted(path) || std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 8192 {
        return Err("boot:invalid_request".into());
    }
    serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map(Some)
        .map_err(|_| "boot:invalid_request".into())
}
#[cfg(target_os = "linux")]
pub fn status() -> Result<serde_json::Value, String> {
    let facts = facts();
    let pending = read_request()?.filter(|request| request.owner_uid == unsafe { libc::getuid() });
    let returned = pending
        .as_ref()
        .is_some_and(|request| boot_id().is_ok_and(|id| id != request.boot_id));
    let confirmed = pending.as_ref().is_some_and(|request| request.scheduled);
    Ok(
        serde_json::json!({"code":assess(&facts),"facts":facts,"pending":pending.is_some(),"confirmed":confirmed,"requested_at":pending.map(|request|request.requested_at),"returned":returned,"automatic_results":false}),
    )
}
#[cfg(target_os = "linux")]
pub fn request(action: &str) -> Result<serde_json::Value, String> {
    if !["verify", "schedule", "cancel"].contains(&action) {
        return Err("boot:invalid_action".into());
    }
    if !trusted(Path::new(HELPER)) {
        return Err("boot:helper_missing".into());
    }
    let output = crate::inventory::command(
        "/usr/bin/pkexec",
        &[HELPER, action],
        Duration::from_secs(180),
    )?;
    let response: serde_json::Value = serde_json::from_str(&output.stdout)
        .map_err(|_| "boot:authorization_or_helper_failed".to_owned())?;
    if output.code != 0 {
        return Err(response["error"]
            .as_str()
            .unwrap_or("boot:helper_failed")
            .into());
    }
    Ok(response)
}
#[cfg(target_os = "linux")]
pub fn reboot() -> Result<(), String> {
    let state = status()?;
    if state["pending"] != true
        || state["confirmed"] != true
        || state["returned"] == true
        || state["code"] != "ready"
    {
        return Err("boot:no_pending_request".into());
    }
    let output =
        crate::inventory::command("/usr/bin/systemctl", &["reboot"], Duration::from_secs(30))?;
    if output.code != 0 {
        return Err("boot:reboot_refused".into());
    }
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "boot-helper"))]
pub mod privileged {
    use super::*;
    use std::{
        io::{Read, Write},
        os::unix::fs::OpenOptionsExt,
        process::{Command, Stdio},
        time::Instant,
    };
    fn command(program: &str, args: &[&str]) -> Result<String, String> {
        if !trusted(Path::new(program)) {
            return Err("boot:untrusted_system_tool".into());
        }
        let mut child = Command::new(program)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stdout = child.stdout.take().unwrap();
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout
                .take(1024 * 1024)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        });
        let deadline = Instant::now() + Duration::from_secs(15);
        let code = loop {
            match child.try_wait() {
                Ok(Some(code)) => break code,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("boot:system_tool_timeout".into());
                }
            }
        };
        let bytes = reader
            .join()
            .map_err(|_| "boot:system_tool_failed")?
            .map_err(|e| e.to_string())?;
        if !code.success() || bytes.len() >= 1024 * 1024 {
            return Err("boot:system_tool_failed".into());
        }
        String::from_utf8(bytes).map_err(|_| "boot:invalid_system_output".into())
    }
    fn snapshot() -> Result<Snapshot, String> {
        let facts = facts();
        let code = assess(&facts);
        if code != "ready" {
            return Err(format!("boot:{code}"));
        }
        let config = Path::new("/boot/grub/grub.cfg");
        if !trusted(config) || !trusted(Path::new(ENVIRONMENT)) {
            return Err("boot:untrusted_grub_files".into());
        }
        if std::fs::metadata(config).map_err(|e| e.to_string())?.len() > 1024 * 1024
            || std::fs::metadata(ENVIRONMENT)
                .map_err(|e| e.to_string())?
                .len()
                != 1024
        {
            return Err("boot:unsupported_grub_files".into());
        }
        Ok(Snapshot {
            facts,
            grub_config: std::fs::read_to_string(config).map_err(|e| e.to_string())?,
            filesystem: command("/usr/sbin/grub-probe", &["--target=fs", ENVIRONMENT])?,
            abstractions: command(
                "/usr/sbin/grub-probe",
                &["--target=abstraction", ENVIRONMENT],
            )?,
            environment: command("/usr/bin/grub-editenv", &[ENVIRONMENT, "list"])?,
        })
    }
    fn save_request(request: &Request) -> Result<(), String> {
        let temporary = format!("/var/lib/computy/boot-{}.tmp", crate::domain::new_id());
        let result = (|| {
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o644)
                .open(&temporary)
                .map_err(|e| e.to_string())?;
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o644))
                .map_err(|e| e.to_string())?;
            file.write_all(&serde_json::to_vec(request).map_err(|e| e.to_string())?)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            std::fs::rename(&temporary, REQUEST).map_err(|e| e.to_string())?;
            std::fs::File::open("/var/lib/computy")
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temporary);
        }
        result
    }
    pub fn run(action: &str, uid: u32) -> Result<serde_json::Value, String> {
        if unsafe { libc::geteuid() } != 0
            || uid == 0
            || !["verify", "schedule", "cancel"].contains(&action)
        {
            return Err("boot:authorization_required".into());
        }
        std::fs::create_dir_all("/var/lib/computy").map_err(|e| e.to_string())?;
        if !trusted(Path::new("/var/lib/computy")) {
            return Err("boot:untrusted_state_directory".into());
        }
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions("/var/lib/computy", std::fs::Permissions::from_mode(0o755))
            .map_err(|error| error.to_string())?;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open("/var/lib/computy/boot.lock")
            .map_err(|e| e.to_string())?;
        if !trusted(Path::new("/var/lib/computy/boot.lock")) {
            return Err("boot:untrusted_lock".into());
        }
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| "boot:busy")?;
        if action == "cancel" {
            let pending = read_request()?.ok_or("boot:no_pending_request")?;
            if pending.owner_uid != uid {
                return Err("boot:another_user_pending".into());
            }
            let environment = command("/usr/bin/grub-editenv", &[ENVIRONMENT, "list"])?;
            match environment_value(&environment, "next_entry") {
                Some("computy-memory-test") => {
                    command(
                        "/usr/bin/grub-editenv",
                        &[ENVIRONMENT, "unset", "next_entry"],
                    )?;
                }
                Some(_) => return Err("boot:another_boot_pending".into()),
                None => {}
            }
            std::fs::remove_file(REQUEST).map_err(|e| e.to_string())?;
            return Ok(serde_json::json!({"cancelled":true}));
        }
        if read_request()?.is_some() {
            return Err("boot:request_already_pending".into());
        }
        let snapshot = snapshot()?;
        validate_snapshot(&snapshot)?;
        if action == "verify" {
            return Ok(
                serde_json::json!({"verified":true,"entry":"computy-memory-test","automatic_results":false}),
            );
        }
        let mut request = Request {
            owner_uid: uid,
            requested_at: crate::domain::timestamp_ms(),
            boot_id: boot_id()?,
            scheduled: false,
        };
        save_request(&request)?;
        // The OS default is never changed. The existing GRUB header consumes next_entry once.
        command("/usr/sbin/grub-reboot", &["computy-memory-test"])?;
        let environment = command("/usr/bin/grub-editenv", &[ENVIRONMENT, "list"])?;
        if environment_value(&environment, "next_entry") != Some("computy-memory-test") {
            return Err("boot:schedule_not_confirmed".into());
        }
        request.scheduled = true;
        save_request(&request)?;
        Ok(
            serde_json::json!({"scheduled":true,"entry":"computy-memory-test","automatic_results":false}),
        )
    }
}
