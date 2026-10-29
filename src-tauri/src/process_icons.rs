//! Optional application icons, resolved locally from PID. No paths supplied by the frontend.
use base64::Engine;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
fn cache() -> &'static Mutex<BTreeMap<PathBuf, Option<String>>> {
    static CACHE: OnceLock<Mutex<BTreeMap<PathBuf, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}
#[cfg(target_os = "linux")]
pub fn cached_png(pid: u32) -> Vec<u8> {
    computy_engine::monitor::process_executable(pid)
        .and_then(|path| {
            cache()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&path)
                .cloned()
                .flatten()
        })
        .and_then(|value| {
            base64::engine::general_purpose::STANDARD
                .decode(value.trim_start_matches("data:image/png;base64,"))
                .ok()
        })
        .unwrap_or_default()
}
pub fn encode_rgba(pixels: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(pixels).ok()?;
    }
    Some(bytes)
}
#[tauri::command]
pub fn process_icons(pids: Vec<u32>) -> Result<BTreeMap<u32, Option<String>>, String> {
    if pids.len() > 32 {
        return Err("Requête trop volumineuse.".into());
    }
    let mut values = BTreeMap::new();
    for pid in pids {
        let icon = computy_engine::monitor::process_executable(pid).and_then(|path| {
            let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
            if let Some(value) = cache.get(&path) {
                return value.clone();
            }
            let value = load_icon(&path)
                .filter(|bytes| bytes.len() <= 128 * 1024)
                .map(|bytes| {
                    format!(
                        "data:image/png;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    )
                });
            if cache.len() >= 256 {
                cache.clear();
            }
            cache.insert(path, value.clone());
            value
        });
        values.insert(pid, icon);
    }
    Ok(values)
}
#[cfg(target_os = "linux")]
fn load_icon(path: &Path) -> Option<Vec<u8>> {
    use gio::prelude::*;
    use gtk::prelude::*;
    if !gtk::is_initialized_main_thread() {
        return None;
    }
    fn normalized(value: &str) -> String {
        value
            .to_lowercase()
            .trim_end_matches(".desktop")
            .trim_end_matches(".exe")
            .trim_start_matches("google-")
            .trim_start_matches("microsoft-")
            .trim_end_matches("-stable")
            .trim_end_matches("-beta")
            .replace([' ', '_'], "-")
    }
    let stem = normalized(path.file_name()?.to_str()?);
    let app = gio::AppInfo::all().into_iter().find(|app| {
        // Some valid DBusActivatable desktop entries have no Exec key. gio 0.18's
        // non-optional executable() binding passes that null pointer to strlen.
        use gtk::glib::translate::{ToGlibPtr, from_glib_none};
        let executable: Option<PathBuf> =
            unsafe { from_glib_none(gio::ffi::g_app_info_get_executable(app.to_glib_none().0)) };
        let name = executable
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|s| s.to_str())
            .unwrap_or("");
        normalized(name) == stem
            || executable.as_deref() == Some(path)
            || app.id().is_some_and(|id| {
                normalized(
                    id.trim_end_matches(".desktop")
                        .rsplit('.')
                        .next()
                        .unwrap_or(""),
                ) == stem
            })
    })?;
    let icon = app.icon()?;
    let theme = gtk::IconTheme::default()?;
    let info = theme.lookup_by_gicon(&icon, 32, gtk::IconLookupFlags::FORCE_SIZE)?;
    info.load_icon().ok()?.save_to_bufferv("png", &[]).ok()
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn load_icon(path: &Path) -> Option<Vec<u8>> {
    #[cfg(target_os = "macos")]
    let path = path
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .unwrap_or(path);
    let icon = file_icon_provider::get_file_icon(path, 32).ok()?;
    encode_rgba(&icon.pixels, icon.width, icon.height)
}
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn load_icon(_: &Path) -> Option<Vec<u8>> {
    None
}
