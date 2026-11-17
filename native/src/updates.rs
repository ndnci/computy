//! Update checks against the public GitHub releases of Computy.
//!
//! This is Computy's only outgoing connection. It sends an anonymous request
//! (generic `Computy/<version>` user agent, no identifier, no hardware data)
//! to the GitHub API, at most once a day when automatic checks are enabled, or
//! when the user asks. Installers are verified against the published SHA-256
//! manifest before anything is installed.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

pub const REPOSITORY: &str = "ndnci/computy";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
/// Automatic checks are spaced by a day.
pub const INTERVAL_MS: u64 = 24 * 60 * 60 * 1000;
const MAX_INSTALLER: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Release {
    /// Version without the leading `v`.
    pub version: String,
    pub notes: String,
    pub page: String,
    pub published_at: String,
    /// Installer for this platform, when the release provides one.
    pub asset: Option<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    /// Lowercase hex digest from the release's `SHA256SUMS_<platform>.txt`.
    pub sha256: String,
}

/// Saved result of the last check (`updates.state`).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Status {
    pub current: String,
    pub checked_at: u64,
    pub latest: Option<Release>,
    pub available: bool,
    pub error: Option<String>,
}

/// Release platform name, as used by `scripts/collect-release.mjs`.
pub fn platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("linux-x86_64"),
        ("windows", "x86_64") => Some("windows-x86_64"),
        ("macos", "x86_64") => Some("macos-x86_64"),
        ("macos", "aarch64") => Some("macos-aarch64"),
        _ => None,
    }
}

/// Installer file name for a version and platform.
pub fn installer_name(version: &str, platform: &str) -> String {
    match platform {
        "linux-x86_64" => format!("Computy_{version}_amd64.deb"),
        "windows-x86_64" => format!("Computy_{version}_{platform}.zip"),
        _ => format!("Computy_{version}_{platform}.dmg"),
    }
}

/// `v1.2.3` / `1.2.3` → (1, 2, 3). Pre-release suffixes are not offered.
pub fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.trim().trim_start_matches('v').split('.');
    let version = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(version)
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    matches!(
        (parse_version(candidate), parse_version(current)),
        (Some(a), Some(b)) if a > b
    )
}

/// Digest for `name` in a `sha256sum`-style manifest.
pub fn checksum_for(manifest: &str, name: &str) -> Option<String> {
    manifest.lines().find_map(|line| {
        let (digest, file) = line.split_once(char::is_whitespace)?;
        let file = file.trim().trim_start_matches('*');
        (file == name && digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| digest.to_ascii_lowercase())
    })
}

/// Parse the GitHub "latest release" JSON. Returns the release and, when an
/// installer exists for `platform`, the checksum manifest URL to fetch.
pub fn parse_release(json: &str, platform: &str) -> Result<(Release, Option<String>), String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|_| "Données de version invalides.".to_owned())?;
    let tag = value["tag_name"].as_str().unwrap_or_default();
    let version = tag.trim_start_matches('v').to_owned();
    if parse_version(&version).is_none() || value["draft"] == true || value["prerelease"] == true {
        return Err("Aucune version publiée pour le moment.".into());
    }
    let assets = value["assets"].as_array().cloned().unwrap_or_default();
    let find = |name: &str| {
        assets
            .iter()
            .find(|a| a["name"].as_str() == Some(name))
            .cloned()
    };
    let installer = find(&installer_name(&version, platform));
    let manifest = find(&format!("SHA256SUMS_{platform}.txt"))
        .and_then(|a| a["browser_download_url"].as_str().map(str::to_owned));
    let release = Release {
        notes: value["body"]
            .as_str()
            .unwrap_or_default()
            .chars()
            .take(4000)
            .collect(),
        page: value["html_url"].as_str().unwrap_or_default().to_owned(),
        published_at: value["published_at"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        asset: installer.map(|a| Asset {
            name: a["name"].as_str().unwrap_or_default().to_owned(),
            url: a["browser_download_url"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            size: a["size"].as_u64().unwrap_or(0),
            sha256: String::new(),
        }),
        version,
    };
    Ok((release, manifest))
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(600))
        .user_agent(&format!("Computy/{CURRENT}"))
        .build()
}

fn get_text(agent: &ureq::Agent, url: &str, limit: u64) -> Result<String, String> {
    let response = agent
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(20))
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "Aucune version publiée pour le moment.".to_owned(),
            ureq::Error::Status(code, _) => {
                format!("Erreur du serveur de mises à jour (HTTP {code}).")
            }
            ureq::Error::Transport(_) => "Serveur de mises à jour injoignable.".to_owned(),
        })?;
    let mut text = String::new();
    response
        .into_reader()
        .take(limit)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    Ok(text)
}

/// Query the latest release. Network only; nothing is downloaded or saved.
pub fn latest() -> Result<Release, String> {
    let platform = platform().ok_or("Mises à jour indisponibles pour cette plateforme.")?;
    let agent = agent();
    let json = get_text(
        &agent,
        &format!("https://api.github.com/repos/{REPOSITORY}/releases/latest"),
        1 << 20,
    )?;
    let (mut release, manifest) = parse_release(&json, platform)?;
    if let (Some(asset), Some(manifest)) = (release.asset.as_mut(), manifest) {
        let text = get_text(&agent, &manifest, 64 * 1024)?;
        asset.sha256 = checksum_for(&text, &asset.name).unwrap_or_default();
    }
    // An installer without a published checksum is never offered.
    if release.asset.as_ref().is_some_and(|a| a.sha256.is_empty()) {
        release.asset = None;
    }
    Ok(release)
}

fn state_path(root: &Path) -> PathBuf {
    root.join("updates.state")
}

pub fn load(root: &Path) -> Status {
    let mut status: Status = std::fs::read(state_path(root))
        .ok()
        .filter(|b| b.len() < 64 * 1024)
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    status.current = CURRENT.into();
    status.available = status
        .latest
        .as_ref()
        .is_some_and(|r| is_newer(&r.version, CURRENT));
    status
}

/// Check now and save the result.
pub fn check(root: &Path) -> Status {
    let mut status = load(root);
    status.checked_at = crate::domain::timestamp_ms();
    match latest() {
        Ok(release) => {
            status.latest = Some(release);
            status.error = None;
        }
        Err(error) => status.error = Some(error),
    }
    status.available = status
        .latest
        .as_ref()
        .is_some_and(|r| is_newer(&r.version, CURRENT));
    let _ = std::fs::create_dir_all(root);
    if let Ok(bytes) = serde_json::to_vec(&status) {
        let _ = std::fs::write(state_path(root), bytes);
    }
    status
}

/// Whether an automatic check is due.
pub fn due(root: &Path, now_ms: u64) -> bool {
    now_ms.saturating_sub(load(root).checked_at) >= INTERVAL_MS
}

/// Download the installer into `directory` and verify its SHA-256.
pub fn download(asset: &Asset, directory: &Path) -> Result<PathBuf, String> {
    let name = Path::new(&asset.name)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| *n == asset.name && !n.starts_with('.'))
        .ok_or("Nom d’installateur invalide.")?;
    if !asset.url.starts_with(&format!(
        "https://github.com/{REPOSITORY}/releases/download/"
    )) {
        return Err("Emplacement de téléchargement inattendu.".into());
    }
    if asset.sha256.len() != 64 {
        return Err("Somme de contrôle de l’installateur manquante.".into());
    }
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let path = directory.join(name);
    let response = agent()
        .get(&asset.url)
        .call()
        .map_err(|_| "Téléchargement échoué.".to_owned())?;
    let mut reader = response.into_reader().take(MAX_INSTALLER + 1);
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut buffer = vec![0; 1 << 16];
    let mut total = 0u64;
    loop {
        let read = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > MAX_INSTALLER {
            let _ = std::fs::remove_file(&path);
            return Err("Installateur trop volumineux.".into());
        }
        hasher.update(&buffer[..read]);
        std::io::Write::write_all(&mut file, &buffer[..read]).map_err(|e| e.to_string())?;
    }
    drop(file);
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if digest != asset.sha256 {
        let _ = std::fs::remove_file(&path);
        return Err("Somme de contrôle incorrecte : le téléchargement a été supprimé.".into());
    }
    Ok(path)
}
