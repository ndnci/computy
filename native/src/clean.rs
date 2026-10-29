//! A persisted preview is the only authority for cleanup. No recursive deletion.
use crate::domain::{new_id, timestamp_ms};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
const MAX_FILES: usize = 30_000;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Rule {
    All { rules: Vec<Rule> },
    Any { rules: Vec<Rule> },
    Not { rule: Box<Rule> },
    PathRegex { value: String },
    Name { value: String },
    Extension { value: String },
    Mime { value: String },
    MinSize { value: u64 },
    MaxSize { value: u64 },
    OlderDays { value: u64 },
    ModifiedBefore { value: u64 },
    ModifiedAfter { value: u64 },
    CreatedBefore { value: u64 },
    CreatedAfter { value: u64 },
}
impl Default for Rule {
    fn default() -> Self {
        Self::OlderDays { value: 7 }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub roots: Vec<PathBuf>,
    pub exclude: Vec<PathBuf>,
    pub rule: Rule,
    pub node_modules: bool,
    pub docker: bool,
    pub docker_volumes: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    pub path: PathBuf,
    pub bytes: u64,
    pub modified_ms: u64,
    pub created_ms: Option<u64>,
    pub mime: String,
    pub identity: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerItem {
    pub kind: String,
    pub id: String,
    pub bytes: u64,
    pub label: String,
    pub identity: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub created_at: u64,
    pub policy: Policy,
    pub files: Vec<FileItem>,
    pub docker: Vec<DockerItem>,
    pub total_bytes: u64,
    pub warnings: Vec<String>,
    pub truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Outcome {
    pub deleted: usize,
    pub skipped: usize,
    pub deleted_bytes: u64,
    pub errors: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub policy: Policy,
    pub interval_hours: u64,
    pub max_bytes: u64,
    pub next_run: u64,
    pub last_result: Option<Outcome>,
    pub enabled: bool,
}
fn fail(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn safe_path(path: &Path) -> bool {
    if !path.is_absolute()
        || path
            .components()
            .any(|p| matches!(p, std::path::Component::ParentDir))
    {
        return false;
    }
    for part in path.ancestors() {
        if let Ok(m) = fs::symlink_metadata(part) {
            if m.file_type().is_symlink() {
                return false;
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if m.file_attributes() & 0x400 != 0 {
                    return false;
                }
            }
        } else {
            return false;
        }
    }
    true
}
fn validate_root(path: &Path) -> Result<(), String> {
    if !safe_path(path) || !path.is_dir() || path.parent().is_none() {
        return Err(format!("Unsafe cleanup root: {}", path.display()));
    }
    let text = path.to_string_lossy();
    let lower = text.to_lowercase();
    if [
        "/var/lib",
        "/var/log",
        "/var/cache",
        "/opt",
        "/System",
        "/Library",
        "/Applications",
    ]
    .iter()
    .any(|p| path.starts_with(p))
        || ["c:\\windows", "c:\\program files", "c:\\programdata"]
            .iter()
            .any(|p| lower == *p || lower.starts_with(&format!("{p}\\")))
    {
        return Err("System and application directories cannot be cleaned".into());
    }
    if [
        "/",
        "/home",
        "/Users",
        "/tmp",
        "/var",
        "/usr",
        "/etc",
        "/boot",
        "/sys",
        "/proc",
        "/dev",
        "/opt",
        "/bin",
        "/lib",
        "/root",
        "C:\\",
        "C:\\Windows",
        "C:\\Program Files",
    ]
    .iter()
    .any(|p| text.eq_ignore_ascii_case(p))
        || home().as_ref() == Some(&path.to_path_buf())
    {
        return Err(
            "Select a specific cache or project folder, not a system or home directory".into(),
        );
    }
    if [
        "/usr/", "/etc/", "/boot/", "/sys/", "/proc/", "/dev/", "/bin/", "/lib/",
    ]
    .iter()
    .any(|p| text.starts_with(p))
    {
        return Err("System directories cannot be cleaned".into());
    }
    Ok(())
}
fn check_rule(
    rule: &Rule,
    depth: usize,
    count: &mut usize,
    regexes: &mut HashMap<String, regex::Regex>,
) -> Result<(), String> {
    *count += 1;
    if depth > 8 || *count > 64 {
        return Err("Limit: 64 conditions, 8 nested groups".into());
    }
    match rule {
        Rule::All { rules } | Rule::Any { rules } => {
            for r in rules {
                check_rule(r, depth + 1, count, regexes)?;
            }
        }
        Rule::Not { rule } => check_rule(rule, depth + 1, count, regexes)?,
        Rule::PathRegex { value } => {
            if value.len() > 512 {
                return Err("Regex too long".into());
            }
            regexes.insert(
                value.clone(),
                regex::RegexBuilder::new(value)
                    .size_limit(256 * 1024)
                    .build()
                    .map_err(fail)?,
            );
        }
        Rule::Name { value } | Rule::Extension { value } | Rule::Mime { value }
            if (value.len() > 256 || value.is_empty()) =>
        {
            return Err("Invalid filter value".into());
        }
        _ => {}
    }
    Ok(())
}
fn matches(
    rule: &Rule,
    item: &FileItem,
    patterns: &HashMap<String, regex::Regex>,
    now: u64,
) -> bool {
    evaluate(rule, item, patterns, now) == Some(true)
}
fn evaluate(
    rule: &Rule,
    item: &FileItem,
    patterns: &HashMap<String, regex::Regex>,
    now: u64,
) -> Option<bool> {
    match rule {
        Rule::All { rules } => {
            let values = rules
                .iter()
                .map(|r| evaluate(r, item, patterns, now))
                .collect::<Vec<_>>();
            if values.contains(&Some(false)) {
                Some(false)
            } else if values.contains(&None) {
                None
            } else {
                Some(true)
            }
        }
        Rule::Any { rules } => {
            let values = rules
                .iter()
                .map(|r| evaluate(r, item, patterns, now))
                .collect::<Vec<_>>();
            if values.contains(&Some(true)) {
                Some(true)
            } else if values.contains(&None) {
                None
            } else {
                Some(false)
            }
        }
        Rule::Not { rule } => evaluate(rule, item, patterns, now).map(|v| !v),
        Rule::CreatedBefore { value } => item.created_ms.map(|t| t < *value),
        Rule::CreatedAfter { value } => item.created_ms.map(|t| t > *value),
        _ => Some(match rule {
            Rule::PathRegex { value } => patterns[value].is_match(&item.path.to_string_lossy()),
            Rule::Name { value } => item.path.file_name().is_some_and(|s| s == value.as_str()),
            Rule::Extension { value } => item.path.extension().is_some_and(|s| {
                s.to_string_lossy()
                    .eq_ignore_ascii_case(value.trim_start_matches('.'))
            }),
            Rule::Mime { value } => &item.mime == value,
            Rule::MinSize { value } => item.bytes >= *value,
            Rule::MaxSize { value } => item.bytes <= *value,
            Rule::OlderDays { value } => {
                item.modified_ms < now.saturating_sub(value.saturating_mul(86_400_000))
            }
            Rule::ModifiedBefore { value } => item.modified_ms < *value,
            Rule::ModifiedAfter { value } => item.modified_ms > *value,
            _ => unreachable!(),
        }),
    }
}

fn millis(t: std::time::SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn identity(m: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        format!(
            "{}:{}:{}:{}:{}",
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime(),
            m.mtime_nsec()
        )
    }
    #[cfg(not(unix))]
    {
        format!("{}:{:?}:{:?}", m.len(), m.modified(), m.created())
    }
}
fn file_item(path: &Path) -> Result<FileItem, String> {
    let m = fs::symlink_metadata(path).map_err(fail)?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err("Not a regular file".into());
    }
    let mut buffer = [0; 8192];
    use std::io::Read;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options.open(path).map_err(fail)?;
    let opened = file.metadata().map_err(fail)?;
    if !opened.is_file() || identity(&opened) != identity(&m) {
        return Err("File changed during scan".into());
    }
    let size = file.read(&mut buffer).map_err(fail)?;
    let mime = infer::get(&buffer[..size])
        .map(|t| t.mime_type())
        .unwrap_or_else(|| {
            if std::str::from_utf8(&buffer[..size]).is_ok() {
                "text/plain"
            } else {
                "application/octet-stream"
            }
        })
        .to_owned();
    Ok(FileItem {
        path: path.to_owned(),
        bytes: m.len(),
        modified_ms: millis(m.modified().map_err(fail)?),
        created_ms: m.created().ok().map(millis),
        mime,
        identity: identity(&m),
    })
}
pub fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}
pub fn sources() -> serde_json::Value {
    let mut roots = vec![];
    if let Some(home) = home() {
        #[cfg(target_os = "linux")]
        roots.push(
            std::env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".cache")),
        );
        #[cfg(target_os = "macos")]
        roots.push(home.join("Library/Caches"));
        #[cfg(target_os = "windows")]
        {
            let _ = home;
            if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                roots.push(PathBuf::from(local).join("Temp"));
            }
        }
    }
    roots.retain(|r| validate_root(r).is_ok());
    serde_json::json!({"roots":roots,"docker_available":crate::commands::run("docker",&["version","--format","{{.Server.Version}}"],4).is_ok()})
}
fn folder(root: &Path) -> PathBuf {
    root.join("clean")
}
fn store<T: Serialize>(root: &Path, name: &str, value: &T) -> Result<(), String> {
    let dir = folder(root);
    fs::create_dir_all(&dir).map_err(fail)?;
    if !safe_path(&dir) {
        return Err("Unsafe cleanup state directory".into());
    }
    let temporary = dir.join(format!("{}.tmp", new_id()));
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    let mut f = options.open(&temporary).map_err(fail)?;
    f.write_all(&serde_json::to_vec(value).map_err(fail)?)
        .and_then(|_| f.sync_all())
        .map_err(fail)?;
    fs::rename(temporary, dir.join(name)).map_err(fail)
}
fn load<T: serde::de::DeserializeOwned>(root: &Path, name: &str) -> Result<T, String> {
    let path = folder(root).join(name);
    if !safe_path(&path) || fs::metadata(&path).map_err(fail)?.len() > 24 * 1024 * 1024 {
        return Err("Invalid cleanup state".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(fail)?).map_err(fail)
}
fn plan_name(id: &str) -> Result<String, String> {
    if id.is_empty() || id.len() > 100 || !id.chars().all(|c| c.is_ascii_digit() || c == '-') {
        return Err("Invalid preview identifier".into());
    }
    Ok(format!("plan-{id}.json"))
}
fn ensure_state(root: &Path) -> Result<(), String> {
    let dir = folder(root);
    for part in dir.ancestors() {
        if let Ok(m) = fs::symlink_metadata(part)
            && m.file_type().is_symlink()
        {
            return Err("Unsafe cleanup state directory".into());
        }
    }
    fs::create_dir_all(&dir).map_err(fail)?;
    if !safe_path(&dir) {
        return Err("Unsafe cleanup state directory".into());
    }
    Ok(())
}
struct Lease {
    _cleanup: fs::File,
    _diagnostic: fs::File,
}
fn lock(root: &Path) -> Result<Lease, String> {
    ensure_state(root)?;
    let p = folder(root).join("lock");
    if p.exists() && !safe_path(&p) {
        return Err("Unsafe cleanup lock".into());
    }
    let f = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(p)
        .map_err(fail)?;
    fs2::FileExt::try_lock_exclusive(&f)
        .map_err(|_| "Another cleanup operation is active".to_owned())?;
    let path = root.join("diagnostic.lock");
    if path.exists() && !safe_path(&path) {
        return Err("Unsafe diagnostic lock".into());
    }
    let diagnostic = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(fail)?;
    fs2::FileExt::try_lock_exclusive(&diagnostic)
        .map_err(|_| "A diagnostic or data maintenance operation is active".to_owned())?;
    Ok(Lease {
        _cleanup: f,
        _diagnostic: diagnostic,
    })
}
pub fn preview(root: &Path, mut policy: Policy) -> Result<Plan, String> {
    let _lock = lock(root)?;
    if policy.roots.len() > 16 || policy.exclude.len() > 64 {
        return Err("Too many roots or exclusions".into());
    }
    let mut patterns = HashMap::new();
    check_rule(&policy.rule, 0, &mut 0, &mut patterns)?;
    for r in &policy.roots {
        validate_root(r)?;
    }
    for e in &policy.exclude {
        if !e.is_absolute()
            || e.components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("Exclusions must be absolute".into());
        }
    }
    for path in &mut policy.roots {
        *path = path.canonicalize().map_err(fail)?;
    }
    policy.roots.sort();
    policy.roots.dedup();
    let now = timestamp_ms();
    let mut plan = Plan {
        id: new_id(),
        created_at: now,
        policy: policy.clone(),
        files: vec![],
        docker: vec![],
        total_bytes: 0,
        warnings: vec![],
        truncated: false,
    };
    let started = std::time::Instant::now();
    let mut seen = HashSet::new();
    let mut visited = 0;
    let mut stack = policy
        .roots
        .iter()
        .map(|p| (p.clone(), 0, false))
        .collect::<Vec<_>>();
    while let Some((path, depth, in_modules)) = stack.pop() {
        if started.elapsed() > std::time::Duration::from_secs(30)
            || visited >= 150_000
            || plan.files.len() >= MAX_FILES
        {
            plan.truncated = true;
            break;
        }
        visited += 1;
        if depth > 48
            || policy.exclude.iter().any(|e| path.starts_with(e))
            || path.starts_with(root)
            || !seen.insert(path.clone())
        {
            continue;
        }
        let Ok(m) = fs::symlink_metadata(&path) else {
            plan.warnings
                .push(format!("Cannot inspect {}", path.display()));
            continue;
        };
        if m.file_type().is_symlink() {
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if m.uid() != unsafe { libc::geteuid() } {
                continue;
            }
        }
        if m.is_dir() {
            let modules = in_modules || path.file_name().is_some_and(|n| n == "node_modules");
            if path
                .file_name()
                .is_some_and(|n| n == ".git" || n == ".ssh" || n == ".gnupg")
            {
                continue;
            }
            match fs::read_dir(&path) {
                Ok(entries) => {
                    let mut entries = entries
                        .filter_map(Result::ok)
                        .map(|e| e.path())
                        .collect::<Vec<_>>();
                    entries.sort();
                    for e in entries.into_iter().rev() {
                        stack.push((e, depth + 1, modules));
                    }
                }
                Err(e) => plan.warnings.push(format!("{}: {e}", path.display())),
            };
        } else if m.is_file()
            && (!policy.node_modules || in_modules)
            && let Ok(item) = file_item(&path)
            && matches(&policy.rule, &item, &patterns, now)
        {
            plan.total_bytes = plan.total_bytes.saturating_add(item.bytes);
            plan.files.push(item);
        }
    }
    if policy.docker {
        match docker_preview(policy.docker_volumes) {
            Ok(items) => plan.docker = items,
            Err(e) => plan.warnings.push(e),
        }
    }
    // Docker shared layers/volume sizes are estimates, never add them to exact file totals.
    plan.warnings.truncate(100);
    store(root, &plan_name(&plan.id)?, &plan)?;
    Ok(plan)
}
/// Open every Unix parent with O_NOFOLLOW. A renamed parent cannot redirect deletion.
fn delete_file(item: &FileItem) -> Result<(), String> {
    if !safe_path(&item.path) {
        return Err("Path contains a symbolic link or no longer exists".into());
    }
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::{
            fd::{AsRawFd, FromRawFd, OwnedFd},
            unix::ffi::OsStrExt,
        };
        let mut fd = unsafe {
            let raw = libc::open(
                c"/".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            );
            if raw < 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            OwnedFd::from_raw_fd(raw)
        };
        let parent = item.path.parent().ok_or("Missing parent")?;
        for c in parent.components() {
            if let std::path::Component::Normal(name) = c {
                let name = CString::new(name.as_bytes()).map_err(fail)?;
                let raw = unsafe {
                    libc::openat(
                        fd.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    )
                };
                if raw < 0 {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                fd = unsafe { OwnedFd::from_raw_fd(raw) };
            }
        }
        let name =
            CString::new(item.path.file_name().ok_or("Missing name")?.as_bytes()).map_err(fail)?;
        let raw = unsafe {
            libc::openat(
                fd.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if raw < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let file = unsafe { fs::File::from_raw_fd(raw) };
        let m = file.metadata().map_err(fail)?;
        if !m.is_file() || identity(&m) != item.identity {
            return Err("File changed since dry run".into());
        }
        if unsafe { libc::unlinkat(fd.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        if identity(&fs::symlink_metadata(&item.path).map_err(fail)?) != item.identity {
            return Err("File changed since dry run".into());
        }
        fs::remove_file(&item.path).map_err(fail)
    }
}
pub fn execute(
    root: &Path,
    id: &str,
    excluded: &[usize],
    confirmed: bool,
) -> Result<Outcome, String> {
    if !confirmed {
        return Err("Explicit confirmation of the dry run is required".into());
    }
    let _lock = lock(root)?;
    let name = plan_name(id)?;
    let plan: Plan = load(root, &name)?;
    let excluded = excluded.iter().copied().collect::<HashSet<_>>();
    if plan.truncated {
        return Err("Scan incomplete; narrow the scope and preview again".into());
    }
    if timestamp_ms().saturating_sub(plan.created_at) > 86_400_000 {
        return Err("Preview expired; run a new dry run".into());
    }
    if excluded
        .iter()
        .any(|&i| i >= plan.files.len() + plan.docker.len())
    {
        return Err("Invalid selection".into());
    }
    // Consume before any destructive operation: a crash cannot replay this authority.
    fs::rename(
        folder(root).join(&name),
        folder(root).join(format!("consumed-{id}.json")),
    )
    .map_err(fail)?;
    let mut result = Outcome::default();
    for (i, item) in plan.files.iter().enumerate() {
        if excluded.contains(&i) {
            continue;
        }
        if !plan.policy.roots.iter().any(|r| item.path.starts_with(r))
            || plan.policy.exclude.iter().any(|e| item.path.starts_with(e))
        {
            result.skipped += 1;
            continue;
        }
        match delete_file(item) {
            Ok(()) => {
                result.deleted += 1;
                result.deleted_bytes = result.deleted_bytes.saturating_add(item.bytes);
            }
            Err(e) => {
                result.skipped += 1;
                if result.errors.len() < 100 {
                    result.errors.push(format!("{}: {e}", item.path.display()));
                }
            }
        }
    }
    for (n, item) in plan.docker.iter().enumerate() {
        if excluded.contains(&(plan.files.len() + n)) {
            continue;
        }
        match docker_remove(item) {
            Ok(()) => result.deleted += 1,
            Err(e) => {
                result.skipped += 1;
                result.errors.push(e);
            }
        }
    }
    store(root, &format!("result-{id}.json"), &result)?;
    Ok(result)
}
fn docker_fingerprint(value: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).unwrap_or_default())
    )
}
fn docker_preview(volumes: bool) -> Result<Vec<DockerItem>, String> {
    let started = std::time::Instant::now();
    if let Ok(host) = std::env::var("DOCKER_HOST")
        && !host.starts_with("unix://")
        && !host.starts_with("npipe://")
    {
        return Err("Remote Docker endpoints cannot be cleaned".into());
    }
    let endpoint = crate::commands::run(
        "docker",
        &[
            "context",
            "inspect",
            "--format",
            "{{.Endpoints.docker.Host}}",
        ],
        5,
    )?;
    if !endpoint.trim().starts_with("unix://") && !endpoint.trim().starts_with("npipe://") {
        return Err("Only a local Docker context can be cleaned".into());
    }
    let mut result = vec![];
    let mut used = HashSet::new();
    let ids = crate::commands::run(
        "docker",
        &["container", "ls", "--all", "--quiet", "--no-trunc"],
        10,
    )?;
    if ids.lines().count() > 512 {
        return Err("Too many Docker containers; narrow the local inventory first".into());
    }
    for id in ids.lines() {
        if started.elapsed() > std::time::Duration::from_secs(20) {
            return Err("Docker scan time limit reached; no Docker resources selected".into());
        }
        let output = crate::commands::run("docker", &["container", "inspect", "--size", id], 10)?;
        let data: Vec<serde_json::Value> = serde_json::from_str(&output).map_err(fail)?;
        for container in data {
            if let Some(image) = container["Image"].as_str() {
                used.insert(image.to_owned());
            }
            if container["State"]["Running"] == false && container["State"]["Paused"] == false {
                result.push(DockerItem {
                    kind: "container".into(),
                    id: id.into(),
                    bytes: container["SizeRw"].as_u64().unwrap_or(0),
                    label: container["Name"].as_str().unwrap_or(id).into(),
                    identity: docker_fingerprint(&container),
                });
            }
        }
    }
    let ids = crate::commands::run("docker", &["image", "ls", "--quiet", "--no-trunc"], 10)?;
    let mut seen = HashSet::new();
    if ids.lines().count() > 512 {
        return Err("Docker image scan limit reached".into());
    }
    for id in ids.lines() {
        if started.elapsed() > std::time::Duration::from_secs(20) {
            return Err("Docker scan time limit reached; no Docker resources selected".into());
        }
        if used.contains(id) || !seen.insert(id) {
            continue;
        }
        let output = crate::commands::run("docker", &["image", "inspect", id], 10)?;
        let data: Vec<serde_json::Value> = serde_json::from_str(&output).map_err(fail)?;
        if let Some(image) = data.first() {
            result.push(DockerItem {
                kind: "image".into(),
                id: id.into(),
                bytes: image["Size"].as_u64().unwrap_or(0),
                label: image["RepoTags"][0].as_str().unwrap_or(id).into(),
                identity: id.into(),
            });
        }
    }
    if volumes {
        let ids = crate::commands::run(
            "docker",
            &["volume", "ls", "--quiet", "--filter", "dangling=true"],
            10,
        )?;
        if ids.lines().count() > 512 {
            return Err("Docker volume scan limit reached".into());
        }
        for id in ids.lines() {
            if started.elapsed() > std::time::Duration::from_secs(20) {
                return Err("Docker scan time limit reached; no Docker resources selected".into());
            }
            let inspect = crate::commands::run("docker", &["volume", "inspect", id], 10)?;
            let data: Vec<serde_json::Value> = serde_json::from_str(&inspect).map_err(fail)?;
            let fingerprint = docker_fingerprint(data.first().ok_or("Volume disappeared")?);
            result.push(DockerItem {
                kind: "volume".into(),
                id: id.into(),
                bytes: 0,
                label: id.into(),
                identity: fingerprint,
            });
        }
    }
    Ok(result)
}
fn docker_remove(item: &DockerItem) -> Result<(), String> {
    if item.id.is_empty()
        || item.id.starts_with('-')
        || !item
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_:-.".contains(c))
    {
        return Err("Invalid Docker resource".into());
    }
    let current = docker_preview(item.kind == "volume")?;
    if !current
        .iter()
        .any(|i| i.id == item.id && i.kind == item.kind && i.identity == item.identity)
    {
        return Err("Docker resource is now in use or unavailable".into());
    }
    let kind = match item.kind.as_str() {
        "image" => "image",
        "volume" => "volume",
        "container" => "container",
        _ => return Err("Unsupported Docker operation".into()),
    };
    crate::commands::run("docker", &[kind, "rm", &item.id], 120).map(|_| ())
}
pub fn schedules(root: &Path) -> Result<Vec<Schedule>, String> {
    if !folder(root).join("schedules.json").exists() {
        return Ok(vec![]);
    }
    load(root, "schedules.json")
}
pub fn schedule(
    root: &Path,
    plan_id: &str,
    interval_hours: u64,
    max_bytes: u64,
    confirmed: bool,
) -> Result<Schedule, String> {
    if !confirmed || !(1..=8760).contains(&interval_hours) || max_bytes == 0 {
        return Err("Confirm a bounded schedule (1–8760 hours)".into());
    }
    let _lock = lock(root)?;
    let plan: Plan = load(root, &plan_name(plan_id)?)?;
    if plan.truncated
        || plan.policy.docker
        || timestamp_ms().saturating_sub(plan.created_at) > 86_400_000
    {
        return Err("Run a complete dry run; Docker resources cannot be automated".into());
    }
    let mut list = schedules(root)?;
    if list.len() >= 16 {
        return Err("Maximum 16 schedules".into());
    }
    let task = Schedule {
        id: new_id(),
        policy: plan.policy,
        interval_hours,
        max_bytes,
        next_run: timestamp_ms() + interval_hours * 3_600_000,
        last_result: None,
        enabled: true,
    };
    list.push(task.clone());
    store(root, "schedules.json", &list)?;
    Ok(task)
}
pub fn remove_schedule(root: &Path, id: &str) -> Result<(), String> {
    let _lock = lock(root)?;
    let mut list = schedules(root)?;
    list.retain(|s| s.id != id);
    store(root, "schedules.json", &list)
}
pub fn run_due(root: &Path) -> Result<Vec<Outcome>, String> {
    // Hold a separate process-wide lease for the full refresh/execute/update cycle.
    ensure_state(root)?;
    let lease = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(folder(root).join("scheduler.lock"))
        .map_err(fail)?;
    fs2::FileExt::try_lock_exclusive(&lease).map_err(|_| "Scheduler already active".to_owned())?;
    let list = schedules(root)?;
    let mut outcomes = vec![];
    for task in list
        .into_iter()
        .filter(|s| s.enabled && s.next_run <= timestamp_ms())
    {
        let result = match preview(root, task.policy.clone()) {
            Ok(plan)
                if !plan.truncated
                    && plan.total_bytes <= task.max_bytes
                    && !plan.policy.docker_volumes =>
            {
                execute(root, &plan.id, &[], true)
            }
            Ok(_) => {
                Err("Automation byte limit exceeded or scan incomplete; nothing removed".into())
            }
            Err(e) => Err(e),
        };
        let result = result.unwrap_or_else(|e| Outcome {
            errors: vec![e],
            ..Outcome::default()
        });
        let _lock = lock(root)?;
        let mut latest = schedules(root)?;
        if let Some(entry) = latest.iter_mut().find(|s| s.id == task.id) {
            entry.next_run = timestamp_ms() + entry.interval_hours * 3_600_000;
            entry.last_result = Some(result.clone());
        }
        store(root, "schedules.json", &latest)?;
        outcomes.push(result);
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_creation_times_stay_unknown_through_negation_and_boolean_groups() {
        let item = FileItem {
            path: "/example/file.log".into(),
            bytes: 1,
            modified_ms: 0,
            created_ms: None,
            mime: "text/plain".into(),
            identity: String::new(),
        };
        let rule = Rule::Not {
            rule: Box::new(Rule::CreatedBefore { value: 100 }),
        };
        assert_eq!(evaluate(&rule, &item, &HashMap::new(), 0), None);
        assert!(!matches(
            &Rule::All { rules: vec![rule] },
            &item,
            &HashMap::new(),
            0
        ));
    }
}
