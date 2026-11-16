//! Operating-system integration: window options, external links, file dialogs,
//! login startup, maintenance, boot/USB helpers and process icons. These were
//! Tauri host commands; their validation rules are unchanged.
use crate::{app::Computy, i18n::t, model::Report};
use computy_engine::service::AppService;
use gpui::*;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};

pub const APP_ID: &str = "computy";

/// Height of the title bar drawn by the application on Linux. GNOME's Wayland
/// compositor provides no server-side decorations, so the window draws its own
/// (as the former GTK header bar did). The requested size grows by this height
/// so the page area keeps the previous 1440×960 viewport.
pub const TITLE_BAR_HEIGHT: f32 = 34.;

pub fn client_decorated() -> bool {
    cfg!(target_os = "linux") && !crate::capture_mode()
}

pub fn main_window_options(size: Size<Pixels>, cx: &App) -> WindowOptions {
    let capture = crate::capture_mode();
    let client = client_decorated();
    let size = if client {
        gpui::size(size.width, size.height + px(TITLE_BAR_HEIGHT))
    } else {
        size
    };
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size, cx))),
        window_min_size: Some(gpui::size(
            px(1000.),
            px(680. + if client { TITLE_BAR_HEIGHT } else { 0. }),
        )),
        titlebar: Some(TitlebarOptions {
            title: Some(format!("Computy — {}", t("common.hardware_clearly")).into()),
            ..Default::default()
        }),
        window_decorations: Some(if client {
            WindowDecorations::Client
        } else {
            WindowDecorations::Server
        }),
        app_id: Some(APP_ID.into()),
        focus: !capture,
        ..Default::default()
    }
}

pub fn initialize_window(window: &mut Window) {
    window.set_app_id(APP_ID);
}

const EXTERNAL_HOSTS: &[&str] = &[
    "www.ndnci.com",
    "ndnci.com",
    "www.memtest.org",
    "memtest.org",
    "www.memtest86.com",
    "memtest86.com",
    "www.smartmontools.org",
    "smartmontools.org",
    "learn.microsoft.com",
    "support.apple.com",
    "kernel.org",
    "www.kernel.org",
    "github.com",
];

pub fn open_external(link: &str, cx: &mut App) {
    let Ok(parsed) = url::Url::parse(link) else {
        return;
    };
    if parsed.scheme() == "https"
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed
            .host_str()
            .is_some_and(|h| EXTERNAL_HOSTS.contains(&h))
    {
        cx.open_url(parsed.as_str());
    }
}

pub fn export_report(
    app: &mut Computy,
    report: Report,
    _window: &mut Window,
    cx: &mut Context<Computy>,
) {
    let Some(service) = app.backend.service().cloned() else {
        return;
    };
    let directory = dirs_documents();
    let receiver = cx.prompt_for_new_path(&directory, Some(&format!("computy-{}.json", report.id)));
    cx.spawn(async move |this, cx| {
        let Ok(Ok(Some(path))) = receiver.await else {
            return;
        };
        let result = service
            .repository
            .get(&report.id)
            .and_then(|value| serde_json::to_vec_pretty(&value).map_err(|e| e.to_string()))
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        let _ = this.update(cx, |this, cx| match result {
            Ok(()) => this.notify(t("platform.report_exported"), false, cx),
            Err(error) => this.notify(error, true, cx),
        });
    })
    .detach();
}

fn dirs_documents() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// Due cleanup schedules run every minute unless a diagnostic is active.
pub fn start_housekeeping(service: Arc<AppService>) {
    std::thread::Builder::new()
        .name("computy-housekeeping".into())
        .spawn(move || {
            loop {
                if !service.active_job().is_some_and(|j| j.running)
                    && let Err(error) = computy_engine::clean::run_due(&service.data_root())
                {
                    eprintln!("Computy automation: {error}");
                }
                std::thread::sleep(Duration::from_secs(60));
            }
        })
        .ok();
}

fn autolaunch() -> Result<auto_launch::AutoLaunch, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Computy")
        .set_app_path(&exe.to_string_lossy())
        .set_args(&[computy_engine::startup::BACKGROUND_ARGUMENT])
        .build()
        .map_err(|e| e.to_string())
}

/// Login monitoring is enabled on first installed launch, never in development.
pub fn sync_login_startup(service: &AppService) {
    if cfg!(debug_assertions) {
        return;
    }
    if computy_engine::startup::desired(&service.data_root()).unwrap_or(false)
        && let Err(error) = autolaunch().and_then(|a| a.enable().map_err(|e| e.to_string()))
    {
        eprintln!("Computy login startup: {error}");
    }
}

pub fn startup_status() -> bool {
    autolaunch()
        .and_then(|a| a.is_enabled().map_err(|e| e.to_string()))
        .unwrap_or(false)
}

pub fn set_startup(service: &AppService, enabled: bool) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("Activez le démarrage automatique depuis une version installée de Computy, pas depuis le mode développement.".into());
    }
    let launcher = autolaunch()?;
    if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    }
    .map_err(|e| e.to_string())?;
    computy_engine::startup::save_desired(&service.data_root(), enabled)
}

pub fn open_offline_tool(service: &Arc<AppService>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        computy_engine::offline::prepare(service)?;
        Command::new("mdsched.exe")
            .spawn()
            .map_err(|e| format!("Impossible d’ouvrir le diagnostic mémoire Windows : {e}"))?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        computy_engine::offline::prepare(service)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = service;
        Err("Utilisez le parcours de test autonome indiqué pour votre système.".into())
    }
}

pub fn boot_action(action: &str) -> Result<serde_json::Value, String> {
    #[cfg(target_os = "linux")]
    {
        match action {
            "verify" | "cancel" => computy_engine::linux_boot::request(action),
            "reboot" => computy_engine::linux_boot::reboot().map(|_| serde_json::json!(true)),
            _ => Err("platform.command_not_allowed".into()),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = action;
        Err("Ce parcours automatique est disponible sous Linux.".into())
    }
}

pub fn usb_targets() -> Result<serde_json::Value, String> {
    #[cfg(target_os = "linux")]
    {
        computy_engine::usb_boot::targets().map(|targets| serde_json::json!(targets))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(serde_json::json!([]))
    }
}

pub fn prepare_usb(id: &str) -> Result<serde_json::Value, String> {
    #[cfg(target_os = "linux")]
    {
        computy_engine::usb_boot::prepare(id).map(|target| serde_json::json!(target))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = id;
        Err("Préparation USB automatique disponible sous Linux.".into())
    }
}

pub fn installation() -> Result<(&'static str, PathBuf), String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    if executable == Path::new("/usr/bin/computy") {
        let status = Command::new("/usr/bin/dpkg-query")
            .args(["-W", "-f=${db:Status-Status}", "computy"])
            .output()
            .map_err(|e| e.to_string())?;
        if status.status.success() && status.stdout == b"installed" {
            return Ok(("deb", executable));
        }
    }
    #[cfg(target_os = "windows")]
    if let Some(parent) = executable.parent() {
        let uninstaller = parent.join("uninstall.exe");
        if uninstaller.is_file() {
            return Ok(("nsis", uninstaller));
        }
    }
    #[cfg(target_os = "macos")]
    if let Some(bundle) = executable
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
    {
        return Ok(("app", bundle.to_owned()));
    }
    Ok(("portable", executable))
}

/// Uninstall through the platform's package owner, after explicit confirmation.
pub fn uninstall(service: &AppService) -> Result<(), String> {
    if service.active_job().is_some_and(|j| j.running) {
        return Err("Arrêtez le diagnostic avant de continuer.".into());
    }
    let (kind, path) = installation()?;
    if kind == "portable" {
        return Err("Supprimez le fichier portable depuis votre gestionnaire de fichiers.".into());
    }
    #[cfg(target_os = "linux")]
    {
        let _ = path;
        let status = Command::new("/usr/bin/pkexec")
            .args(["/usr/bin/apt-get", "remove", "--yes", "computy"])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Désinstallation annulée ou refusée.".into());
        }
    }
    #[cfg(target_os = "windows")]
    {
        Command::new(path).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("/usr/bin/osascript")
            .args(["-e", "on run argv\ntell application \"Finder\" to delete POSIX file (item 1 of argv)\nend run", "--"])
            .arg(path)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Désinstallation annulée ou refusée.".into());
        }
    }
    if let Ok(launcher) = autolaunch()
        && launcher.is_enabled().unwrap_or(false)
    {
        let _ = launcher.disable();
    }
    Ok(())
}

/// Application icon for a process, resolved locally from its executable.
pub fn process_icon(pid: u32) -> Option<PathBuf> {
    use std::{collections::HashMap, sync::Mutex, sync::OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<PathBuf>>>> = OnceLock::new();
    let executable = computy_engine::monitor::process_executable(pid)?;
    let cache = CACHE.get_or_init(Default::default);
    if let Some(value) = cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&executable)
    {
        return value.clone();
    }
    let value = icons::lookup(&executable);
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() >= 256 {
        cache.clear();
    }
    cache.insert(executable, value.clone());
    value
}

/// Edge of the decoded process icon: the avatar draws it at 20 px, so 40 px
/// stays sharp at 2× scaling.
const ICON_PIXELS: u32 = 40;

/// Small decoded image for a process icon. Icon themes often ship 256 px or
/// larger PNGs and SVGs with large canvases; decoding those at native size
/// costs megabytes per icon in RAM and in the GPU atlas for a 20 px avatar.
pub fn process_image(pid: u32) -> Option<Arc<RenderImage>> {
    let path = process_icon(pid)?;
    icon_image(&path)
}

pub fn icon_image(path: &Path) -> Option<Arc<RenderImage>> {
    use std::{collections::HashMap, sync::Mutex, sync::OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<Arc<RenderImage>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(value) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(path) {
        return value.clone();
    }
    let value = decode_icon(path, ICON_PIXELS).map(|frame| {
        Arc::new(RenderImage::new(smallvec::SmallVec::from_elem(
            image::Frame::new(frame),
            1,
        )))
    });
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() >= 256 {
        cache.clear();
    }
    cache.insert(path.to_owned(), value.clone());
    value
}

/// Decode and scale an icon file to a BGRA square of `edge` pixels.
fn decode_icon(path: &Path, edge: u32) -> Option<image::RgbaImage> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > 8 * 1024 * 1024 {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let is_svg = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("svg") || e.eq_ignore_ascii_case("svgz"));
    let mut rgba = if is_svg {
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).ok()?;
        let size = tree.size();
        let scale = edge as f32 / size.width().max(size.height()).max(1.);
        let mut pixmap = resvg::tiny_skia::Pixmap::new(edge, edge)?;
        let dx = (edge as f32 - size.width() * scale) / 2.;
        let dy = (edge as f32 - size.height() * scale) / 2.;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_row(scale, 0., 0., scale, dx, dy),
            &mut pixmap.as_mut(),
        );
        // tiny-skia stores premultiplied RGBA; GPUI expects straight BGRA.
        let mut data = pixmap.take();
        for px in data.as_chunks_mut::<4>().0 {
            let a = px[3] as u32;
            if a > 0 && a < 255 {
                for channel in &mut px[..3] {
                    *channel = ((*channel as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        image::RgbaImage::from_raw(edge, edge, data)?
    } else {
        let decoded = image::load_from_memory(&bytes).ok()?;
        decoded
            .resize(edge, edge, image::imageops::FilterType::Triangle)
            .into_rgba8()
    };
    for px in rgba.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    Some(rgba)
}

#[cfg(target_os = "linux")]
mod icons {
    //! Desktop-entry and icon-theme lookup without GTK.
    use std::{
        path::{Path, PathBuf},
        sync::OnceLock,
    };

    fn normalized(value: &str) -> String {
        value
            .to_lowercase()
            .trim_end_matches(".desktop")
            .trim_start_matches("google-")
            .trim_start_matches("microsoft-")
            .trim_end_matches("-stable")
            .trim_end_matches("-beta")
            .replace([' ', '_'], "-")
    }

    fn data_dirs() -> Vec<PathBuf> {
        let mut dirs = vec![];
        if let Some(home) = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        {
            dirs.push(home);
        }
        let system =
            std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
        dirs.extend(
            system
                .split(':')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from),
        );
        dirs.push(PathBuf::from("/var/lib/flatpak/exports/share"));
        dirs
    }

    struct Entry {
        id: String,
        exec: Option<String>,
        icon: String,
    }

    fn entries() -> &'static [Entry] {
        static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
        ENTRIES.get_or_init(|| {
            let mut result = vec![];
            for dir in data_dirs() {
                let Ok(files) = std::fs::read_dir(dir.join("applications")) else {
                    continue;
                };
                for file in files.flatten() {
                    let path = file.path();
                    if path.extension().is_none_or(|e| e != "desktop") {
                        continue;
                    }
                    let Ok(text) = std::fs::read_to_string(&path) else {
                        continue;
                    };
                    let mut exec = None;
                    let mut icon = None;
                    let mut in_entry = false;
                    for line in text.lines() {
                        if line.starts_with('[') {
                            in_entry = line == "[Desktop Entry]";
                            continue;
                        }
                        if !in_entry {
                            continue;
                        }
                        if let Some(value) = line.strip_prefix("Exec=") {
                            exec = value
                                .split_whitespace()
                                .find(|part| !part.contains('='))
                                .map(|p| p.trim_matches('"').to_owned());
                        } else if let Some(value) = line.strip_prefix("Icon=") {
                            icon = Some(value.trim().to_owned());
                        }
                    }
                    if let Some(icon) = icon {
                        result.push(Entry {
                            id: path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or_default()
                                .to_owned(),
                            exec,
                            icon,
                        });
                    }
                }
            }
            result
        })
    }

    fn resolve(icon: &str) -> Option<PathBuf> {
        let path = Path::new(icon);
        if path.is_absolute() {
            return path.is_file().then(|| path.to_owned());
        }
        for dir in data_dirs() {
            for theme in ["hicolor", "Adwaita", "Yaru"] {
                for size in ["48x48", "64x64", "32x32", "128x128", "256x256", "scalable"] {
                    for ext in ["png", "svg"] {
                        let candidate = dir.join(format!("icons/{theme}/{size}/apps/{icon}.{ext}"));
                        if candidate.is_file() {
                            return Some(candidate);
                        }
                    }
                }
            }
            for ext in ["png", "svg"] {
                let candidate = dir.join(format!("pixmaps/{icon}.{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    pub fn lookup(executable: &Path) -> Option<PathBuf> {
        let stem = normalized(executable.file_name()?.to_str()?);
        let entry = entries().iter().find(|entry| {
            let exec = entry.exec.as_deref().unwrap_or("");
            let name = Path::new(exec)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            normalized(name) == stem
                || Path::new(exec) == executable
                || normalized(entry.id.rsplit('.').next().unwrap_or("")) == stem
        })?;
        resolve(&entry.icon)
    }
}

#[cfg(not(target_os = "linux"))]
mod icons {
    use std::path::{Path, PathBuf};
    pub fn lookup(executable: &Path) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        let executable = executable
            .ancestors()
            .find(|p| p.extension().is_some_and(|e| e == "app"))
            .unwrap_or(executable);
        let icon = file_icon_provider::get_file_icon(executable, 32).ok()?;
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, icon.width, icon.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().ok()?;
            writer.write_image_data(&icon.pixels).ok()?;
        }
        let path = std::env::temp_dir().join(format!(
            "computy-icon-{:x}.png",
            executable
                .to_string_lossy()
                .bytes()
                .fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64))
        ));
        std::fs::write(&path, bytes).ok()?;
        Some(path)
    }
}
