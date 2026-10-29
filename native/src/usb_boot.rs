//! Prepare an existing mounted FAT USB without formatting or overwriting files.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(target_os = "linux")]
use std::time::Duration;
use std::{
    io::{Read, Write},
    path::Path,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub id: String,
    pub label: String,
    pub mount: String,
    pub size_bytes: u64,
}
fn walk<'a>(node: &'a Value, found: &mut Vec<&'a Value>) {
    found.push(node);
    if let Some(children) = node["children"].as_array() {
        for child in children {
            walk(child, found);
        }
    }
}
pub fn parse_targets(value: &Value) -> Vec<Target> {
    let mut targets = Vec::new();
    for disk in value["blockdevices"].as_array().into_iter().flatten() {
        if disk["type"] != "disk" || disk["tran"] != "usb" {
            continue;
        }
        let mut nodes = Vec::new();
        walk(disk, &mut nodes);
        if nodes.iter().any(|node| {
            node["mountpoints"].as_array().is_some_and(|mounts| {
                mounts.iter().filter_map(Value::as_str).any(|mount| {
                    ["/", "/boot", "/boot/efi", "/home", "/usr", "/var"].contains(&mount)
                })
            })
        }) {
            continue;
        }
        for node in nodes {
            if node["fstype"] != "vfat" || node["fsver"] != "FAT32" {
                continue;
            }
            let Some(name) = node["name"]
                .as_str()
                .filter(|name| name.starts_with("/dev/"))
            else {
                continue;
            };
            let Some(uuid) = node["uuid"].as_str().filter(|uuid| !uuid.is_empty()) else {
                continue;
            };
            for mount in node["mountpoints"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if !mount.starts_with('/') {
                    continue;
                }
                targets.push(Target {
                    id: format!("{name}:{uuid}"),
                    label: node["label"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .unwrap_or(name)
                        .into(),
                    mount: mount.into(),
                    size_bytes: node["size"].as_u64().unwrap_or(0),
                });
            }
        }
    }
    targets
}
#[cfg(target_os = "linux")]
pub fn targets() -> Result<Vec<Target>, String> {
    let output = crate::inventory::command(
        "/usr/bin/lsblk",
        &[
            "--json",
            "--bytes",
            "--paths",
            "--output",
            "NAME,TRAN,FSTYPE,FSVER,LABEL,SIZE,MOUNTPOINTS,TYPE,UUID",
        ],
        Duration::from_secs(5),
    )?;
    if output.code != 0 {
        return Err("boot:usb_scan_failed".into());
    }
    let value = serde_json::from_str(&output.stdout).map_err(|_| "boot:usb_scan_failed")?;
    Ok(parse_targets(&value))
}
pub fn copy_payload(payload: &Path, destination: &Path) -> Result<(), String> {
    if destination.canonicalize().map_err(|e| e.to_string())? != destination {
        return Err("boot:usb_invalid_mount".into());
    }
    let directory = destination.join("EFI");
    let boot = directory.join("BOOT");
    for path in [&directory, &boot] {
        if !path.try_exists().map_err(|e| e.to_string())? {
            std::fs::create_dir(path).map_err(|e| e.to_string())?;
        }
        let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("boot:usb_invalid_directory".into());
        }
    }
    let target = boot.join("BOOTX64.EFI");
    let mut input = std::fs::File::open(payload).map_err(|_| "boot:payload_missing")?;
    let size = input.metadata().map_err(|e| e.to_string())?.len();
    if size == 0 || size > 4 * 1024 * 1024 {
        return Err("boot:invalid_payload".into());
    }
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&target)
        .map_err(|_| "boot:usb_existing_loader")?;
    let result = (|| {
        let mut bytes = Vec::new();
        input.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        output
            .write_all(&bytes)
            .and_then(|_| output.sync_all())
            .map_err(|e| e.to_string())
    })();
    drop(output);
    if result.is_err() {
        let _ = std::fs::remove_file(target);
    }
    result
}
#[cfg(target_os = "linux")]
pub fn prepare(id: &str) -> Result<Target, String> {
    let target = targets()?
        .into_iter()
        .find(|target| target.id == id)
        .ok_or("boot:usb_not_found")?;
    copy_payload(
        Path::new("/boot/computy-memory.efi"),
        Path::new(&target.mount),
    )?;
    Ok(target)
}
