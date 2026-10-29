//! OS inventory and manager-owned removal. No guessed data directories or shell commands.
use crate::commands::run;
use serde::{Deserialize, Serialize};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs;
use std::{collections::HashMap, path::PathBuf};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub version: String,
    pub manager: String,
    pub package: String,
    pub location: Option<PathBuf>,
    pub launcher: Option<PathBuf>,
    pub launch_target: Option<String>,
    pub can_uninstall: bool,
    pub can_remove_data: bool,
    pub system: bool,
}
#[derive(Debug, Serialize)]
pub struct Catalog {
    pub apps: Vec<Application>,
    pub warnings: Vec<String>,
}
fn app(id: &str, name: &str, manager: &str, package: &str) -> Application {
    Application {
        id: format!("{manager}:{id}"),
        name: name.into(),
        version: String::new(),
        manager: manager.into(),
        package: package.into(),
        location: None,
        launcher: None,
        launch_target: None,
        can_uninstall: false,
        can_remove_data: false,
        system: false,
    }
}
pub fn desktop_fields(content: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    let mut active = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            active = line == "[Desktop Entry]";
            continue;
        }
        if active && let Some((key, value)) = line.split_once('=') {
            values.insert(key.to_owned(), value.to_owned());
        }
    }
    values
}
/// Resolve an uncomplicated desktop executable for folder display, never execution.
/// Wrappers/field codes/arguments are deliberately not guessed.
#[cfg(target_os = "linux")]
fn desktop_location(fields: &HashMap<String, String>) -> Option<PathBuf> {
    let value = fields.get("TryExec").or_else(|| fields.get("Exec"))?;
    let token = if let Some(rest) = value.strip_prefix('"') {
        rest.split_once('"')?.0
    } else {
        value.split_whitespace().next()?
    };
    if token.contains('%') || ["env", "sh", "bash", "flatpak", "snap"].contains(&token) {
        return None;
    }
    let path = PathBuf::from(token);
    let executable = if path.is_absolute() {
        path
    } else {
        std::env::split_paths(&std::env::var_os("PATH")?)
            .map(|p| p.join(token))
            .find(|p| p.is_file())?
    };
    executable
        .canonicalize()
        .ok()?
        .parent()
        .map(|p| p.to_path_buf())
}
pub fn list() -> Catalog {
    let mut apps = vec![];
    let mut warnings = vec![];
    #[cfg(target_os = "linux")]
    {
        let packages = run(
            "/usr/bin/dpkg-query",
            &[
                "-W",
                "-f=${binary:Package}\t${Version}\t${db:Status-Status}\n",
            ],
            15,
        );
        if let Ok(output) = packages {
            for line in output.lines() {
                let fields = line.split('\t').collect::<Vec<_>>();
                if fields.len() == 3 && fields[2] == "installed" {
                    let mut a = app(fields[0], fields[0], "deb", fields[0]);
                    a.version = fields[1].into();
                    a.can_uninstall = true;
                    a.system = true;
                    apps.push(a);
                }
            }
        }
        let mut directories = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
        ];
        if let Some(home) = crate::clean::home() {
            directories.push(home.join(".local/share/applications"));
            directories.push(home.join(".local/share/flatpak/exports/share/applications"));
        }
        directories.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
        directories.push(PathBuf::from("/var/lib/snapd/desktop/applications"));
        let owners = run(
            "/usr/bin/dpkg-query",
            &["-S", "/usr/share/applications/*.desktop"],
            15,
        )
        .unwrap_or_default();
        let owners = owners
            .lines()
            .filter_map(|l| l.split_once(": "))
            .map(|(p, f)| (f.to_owned(), p.to_owned()))
            .collect::<HashMap<_, _>>();
        for dir in directories {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_none_or(|e| e != "desktop") {
                    continue;
                }
                let Ok(content) = fs::read_to_string(&path) else {
                    continue;
                };
                let fields = desktop_fields(&content);
                if fields.get("Type").is_none_or(|s| s != "Application")
                    || fields.get("Hidden").is_some_and(|s| s == "true")
                    || fields.get("NoDisplay").is_some_and(|s| s == "true")
                {
                    continue;
                }
                let Some(name) = fields.get("Name") else {
                    continue;
                };
                let id = path.to_string_lossy();
                let mut a = app(&id, name, "desktop", "");
                a.location = desktop_location(&fields);
                a.launcher = Some(path.clone());
                if let Some(package) = owners.get(id.as_ref()) {
                    a.manager = "deb".into();
                    a.package = package.clone();
                    a.can_uninstall = true;
                    if let Some(existing) = apps.iter_mut().find(|a| &a.package == package) {
                        existing.name = name.into();
                        existing.location = a.location;
                        existing.launcher = a.launcher;
                        existing.system = false;
                        continue;
                    }
                }
                apps.push(a);
            }
        }
        if let Ok(output) = run(
            "flatpak",
            &[
                "list",
                "--app",
                "--columns=application,name,version,installation",
            ],
            10,
        ) {
            for line in output.lines() {
                let f = line.split('\t').collect::<Vec<_>>();
                if f.len() >= 4 {
                    let mut a = app(&format!("{}:{}", f[3], f[0]), f[1], "flatpak", f[0]);
                    a.version = f[2].into();
                    a.can_uninstall = true;
                    a.can_remove_data = true;
                    let user = f[3] == "user";
                    let base = if user {
                        crate::clean::home().map(|h| h.join(".local/share/flatpak"))
                    } else {
                        Some(PathBuf::from("/var/lib/flatpak"))
                    };
                    a.location = base.map(|p| p.join("app").join(f[0]));
                    a.system = false;
                    apps.push(a);
                }
            }
        }
        if let Ok(output) = run("snap", &["list"], 10) {
            for line in output.lines().skip(1) {
                let f = line.split_whitespace().collect::<Vec<_>>();
                if f.len() >= 2 {
                    let mut a = app(f[0], f[0], "snap", f[0]);
                    a.version = f[1].into();
                    a.location = Some(PathBuf::from("/snap").join(f[0]));
                    a.can_uninstall = true;
                    a.can_remove_data = true;
                    apps.push(a);
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let mut roots = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
        ];
        if let Some(h) = crate::clean::home() {
            roots.push(h.join("Applications"));
        }
        for root in roots {
            let Ok(entries) = fs::read_dir(&root) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_none_or(|e| e != "app") {
                    continue;
                }
                let system = path.starts_with("/System");
                let id = path.to_string_lossy();
                let name = path.file_stem().unwrap_or_default().to_string_lossy();
                let mut a = app(&id, &name, "bundle", "");
                a.launcher = Some(path.clone());
                a.location = Some(path);
                a.can_uninstall = !system;
                a.system = system;
                apps.push(a);
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        // Read registry data as JSON; never execute an UninstallString supplied by a registry entry.
        let script = r#"$p='HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*','HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'; @(Get-ItemProperty $p -ErrorAction SilentlyContinue | Where-Object {$_.DisplayName} | Select-Object PSChildName,DisplayName,DisplayVersion,InstallLocation,SystemComponent) | ConvertTo-Json -Compress"#;
        match run(
            "powershell.exe",
            &["-NoProfile", "-NonInteractive", "-Command", script],
            20,
        ) {
            Ok(output) => match serde_json::from_str::<Vec<serde_json::Value>>(&output) {
                Ok(rows) => {
                    for r in rows {
                        let Some(id) = r["PSChildName"].as_str() else {
                            continue;
                        };
                        let mut a = app(id, r["DisplayName"].as_str().unwrap_or(id), "winget", id);
                        a.version = r["DisplayVersion"].as_str().unwrap_or("").into();
                        a.location = r["InstallLocation"]
                            .as_str()
                            .filter(|p| !p.is_empty())
                            .map(PathBuf::from);
                        a.system = r["SystemComponent"].as_u64() == Some(1);
                        a.can_uninstall = !a.system;
                        apps.push(a);
                    }
                }
                Err(e) => warnings.push(e.to_string()),
            },
            Err(e) => warnings.push(e),
        }
        // Store apps use their registered AppsFolder entry for launching and exact package identity for removal.
        let script = "@(Get-AppxPackage | Select-Object Name,PackageFullName,PackageFamilyName,InstallLocation,NonRemovable) | ConvertTo-Json -Compress";
        if let Ok(output) = run(
            "powershell.exe",
            &["-NoProfile", "-NonInteractive", "-Command", script],
            20,
        ) {
            if let Ok(rows) = serde_json::from_str::<Vec<serde_json::Value>>(&output) {
                for r in rows {
                    let Some(id) = r["PackageFullName"].as_str() else {
                        continue;
                    };
                    let mut a = app(id, r["Name"].as_str().unwrap_or(id), "appx", id);
                    a.location = r["InstallLocation"].as_str().map(PathBuf::from);
                    a.system = r["NonRemovable"].as_bool().unwrap_or(true);
                    a.can_uninstall = false;
                    apps.push(a);
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = run(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "@(Get-StartApps | Select-Object Name,AppID) | ConvertTo-Json -Compress",
            ],
            15,
        ) {
            if let Ok(rows) = serde_json::from_str::<Vec<serde_json::Value>>(&output) {
                for a in &mut apps {
                    let found = rows
                        .iter()
                        .filter(|r| r["Name"].as_str() == Some(a.name.as_str()))
                        .collect::<Vec<_>>();
                    if found.len() == 1 {
                        a.launch_target = found[0]["AppID"].as_str().map(str::to_owned);
                    }
                }
            }
        }
    }
    apps.sort_by_cached_key(|a| a.name.to_lowercase());
    apps.dedup_by(|a, b| a.id == b.id);
    if apps.is_empty() {
        warnings.push("No supported application inventory provider is available".into());
    }
    Catalog { apps, warnings }
}
fn resolve(id: &str) -> Result<Application, String> {
    list()
        .apps
        .into_iter()
        .find(|a| a.id == id)
        .ok_or("Application is no longer installed".into())
}
pub fn action(
    id: &str,
    action: &str,
    remove_data: bool,
    confirmed: bool,
) -> Result<serde_json::Value, String> {
    let a = resolve(id)?;
    if action == "preview_uninstall" {
        if !a.can_uninstall {
            return Err("No supported uninstaller is available".into());
        }
        let details = if a.manager == "deb" {
            run(
                "/usr/bin/apt-get",
                &["--simulate", "remove", "--", &a.package],
                30,
            )?
        } else {
            format!("{} · {}", a.manager, a.package)
        };
        return Ok(serde_json::json!({"details":details,"app":a}));
    }
    if action == "reveal" {
        let path = a
            .location
            .as_ref()
            .ok_or("Installation folder is unavailable")?;
        if !path.exists() {
            return Err("Installation folder no longer exists".into());
        }
        let program = if cfg!(target_os = "windows") {
            "explorer.exe"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        std::process::Command::new(program)
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    } else if action == "launch" {
        #[cfg(target_os = "linux")]
        {
            if a.manager == "flatpak" {
                std::process::Command::new("flatpak")
                    .args(["run", &a.package])
                    .spawn()
                    .map_err(|e| e.to_string())?;
            } else if a.manager == "snap" {
                std::process::Command::new("snap")
                    .args(["run", &a.package])
                    .spawn()
                    .map_err(|e| e.to_string())?;
            } else {
                let path = a
                    .launcher
                    .as_ref()
                    .ok_or("No application launcher is available")?;
                std::process::Command::new("gio")
                    .arg("launch")
                    .arg(path)
                    .spawn()
                    .map_err(|e| e.to_string())?;
            }
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(a.launcher.ok_or("No launcher")?)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        #[cfg(target_os = "windows")]
        {
            let id = a
                .launch_target
                .ok_or("No verified Start menu launcher is available")?;
            std::process::Command::new("explorer.exe")
                .arg(format!("shell:AppsFolder\\{id}"))
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    } else if action == "uninstall" {
        if !confirmed || !a.can_uninstall {
            return Err("Explicit confirmation is required; this application cannot be removed automatically".into());
        }
        if remove_data && !a.can_remove_data {
            return Err("This package manager cannot safely identify all application data".into());
        }
        if a.package.starts_with('-') || a.package.len() > 256 {
            return Err("Invalid package identity".into());
        }
        #[cfg(target_os = "linux")]
        {
            match a.manager.as_str() {
                "deb" => {
                    if !a
                        .package
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || ".+-:".contains(c))
                    {
                        return Err("Invalid package".into());
                    }
                    run(
                        "/usr/bin/pkexec",
                        &[
                            "/usr/bin/apt-get",
                            "remove",
                            "--assume-yes",
                            "--",
                            &a.package,
                        ],
                        600,
                    )?;
                }
                "flatpak" => {
                    let scope = if a.id.contains("flatpak:user:") {
                        "--user"
                    } else {
                        "--system"
                    };
                    let mut args = vec!["uninstall", scope, "--noninteractive"];
                    if remove_data {
                        args.push("--delete-data");
                    }
                    args.push(&a.package);
                    run("flatpak", &args, 600)?;
                }
                "snap" => {
                    let mut args = vec!["/usr/bin/snap", "remove"];
                    if remove_data {
                        args.push("--purge");
                    }
                    args.push(&a.package);
                    run("/usr/bin/pkexec", &args, 600)?;
                }
                _ => return Err("Use the application's original installer to remove it".into()),
            }
        }
        #[cfg(target_os = "windows")]
        {
            if a.manager == "winget" {
                run(
                    "winget",
                    &[
                        "uninstall",
                        "--id",
                        &a.package,
                        "--exact",
                        "--interactive",
                        "--source",
                        "winget",
                    ],
                    600,
                )?;
            } else {
                return Err("Use Windows Settings > Apps to remove this Store application".into());
            }
        }
        #[cfg(target_os = "macos")]
        {
            let path = a.launcher.ok_or("Application bundle is unavailable")?;
            let script = "on run argv\ntell application \"Finder\" to delete (POSIX file (item 1 of argv))\nend run";
            run("osascript", &["-e", script, &path.to_string_lossy()], 120)?;
        }
    } else {
        return Err("Unknown application action".into());
    }
    Ok(serde_json::json!({"completed":true}))
}
