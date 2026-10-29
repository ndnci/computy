use computy_engine::{monitor::Snapshot, preferences::Preferences, service::AppService};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri::{
    Manager,
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
};

struct Indicator {
    tray: TrayIcon,
    last: String,
    language: String,
    detail: MenuItem<tauri::Wry>,
    process_items: Vec<MenuItem<tauri::Wry>>,
}
type NativeMenu = (
    Menu<tauri::Wry>,
    MenuItem<tauri::Wry>,
    Vec<MenuItem<tauri::Wry>>,
);
#[derive(Default)]
pub struct TrayState {
    indicators: Mutex<BTreeMap<String, Indicator>>,
    pub stopped: AtomicBool,
}
impl TrayState {
    pub fn has_icons(&self) -> bool {
        !self
            .indicators
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
    }
}
fn show(app: &tauri::AppHandle, destination: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let script = match destination {
            "settings" => "window.location.hash='settings'",
            "memory" => "window.location.hash='processes?metric=memory'",
            "cpu" => "window.location.hash='processes?metric=cpu'",
            "gpu" => "window.location.hash='processes?metric=gpu'",
            "storage" => "window.location.hash='processes?metric=storage'",
            _ => "window.location.hash='overview'",
        };
        let _ = window.eval(script);
    }
}
fn process_lines(preferences: &Preferences, snapshot: &Snapshot, metric: &str) -> Vec<String> {
    let percent = |value: Option<f64>| {
        value
            .map(|v| format!("{v:.1}%"))
            .unwrap_or_else(|| "--".into())
    };
    let memory = |value: Option<u64>| {
        value
            .map(|v| {
                format!(
                    "{:.1} {}",
                    v as f64 / 1048576.0,
                    message(preferences, "Mio")
                )
            })
            .unwrap_or_else(|| "--".into())
    };
    computy_engine::monitor::top_processes(snapshot, metric)
        .iter()
        .map(|process| {
            let name: String = process
                .name
                .chars()
                .filter(|c| !c.is_control())
                .take(45)
                .collect::<String>()
                .replace('&', "&&");
            let amount = match metric {
                "memory" => format!(
                    "{} · {}",
                    memory(process.memory_bytes),
                    percent(process.memory_percent)
                ),
                "cpu" => format!(
                    "{} · RAM {}",
                    percent(process.cpu_percent),
                    memory(process.memory_bytes)
                ),
                "gpu" => format!("VRAM {}", memory(process.gpu_memory_bytes)),
                _ => format!(
                    "{:.2} MiB/s · {}",
                    (process.read_bytes_per_second.unwrap_or(0.0)
                        + process.written_bytes_per_second.unwrap_or(0.0))
                        / 1048576.0,
                    percent(process.io_percent)
                ),
            };
            format!("{name} [{}] · {amount}", process.pid)
        })
        .collect()
}
fn menu(
    app: &tauri::AppHandle,
    preferences: &Preferences,
    detail: &str,
    metric: &str,
    lines: &[String],
) -> tauri::Result<NativeMenu> {
    let menu = Menu::new(app)?;
    let detail_item = MenuItem::new(app, detail, false, None::<&str>)?;
    menu.append(&detail_item)?;
    let mut process_items = vec![];
    let note = match metric {
        "gpu" => "Mémoire GPU des processus de calcul NVIDIA uniquement",
        "storage" => "Activité par processus : tous les volumes",
        _ => "Processus les plus consommateurs",
    };
    menu.append(&MenuItem::new(
        app,
        message(preferences, note),
        false,
        None::<&str>,
    )?)?;
    if lines.is_empty() {
        menu.append(&MenuItem::new(
            app,
            message(preferences, "Aucune mesure par processus disponible."),
            false,
            None::<&str>,
        )?)?;
    }
    for line in lines {
        let item = MenuItem::new(app, line, false, None::<&str>)?;
        menu.append(&item)?;
        process_items.push(item);
    }
    for (id, text) in [
        (format!("monitor-all-{metric}"), "Voir tout"),
        ("monitor-open".into(), "Ouvrir Computy"),
        ("monitor-settings".into(), "Préférences"),
        ("monitor-quit".into(), "Quitter Computy"),
    ] {
        menu.append(&MenuItem::with_id(
            app,
            id,
            message(preferences, text),
            true,
            None::<&str>,
        )?)?;
    }
    Ok((menu, detail_item, process_items))
}
fn message(preferences: &Preferences, key: &str) -> String {
    // Native menus use the same bundled translations as the desktop UI.
    let language = preferences.language.clone();
    if language == "fr" {
        return key.into();
    }
    static CATALOGS: OnceLock<serde_json::Value> = OnceLock::new();
    let catalogs = CATALOGS.get_or_init(|| {
        serde_json::from_str(include_str!("../../src/locales/catalogs.json")).unwrap_or_default()
    });
    catalogs
        .get(&language)
        .or_else(|| catalogs.get("en"))
        .and_then(|c| c.get(key))
        .and_then(|s| s.as_str())
        .unwrap_or(key)
        .into()
}
pub fn start(app: tauri::AppHandle, service: Arc<AppService>) {
    std::thread::spawn(move || {
        while !app.state::<TrayState>().stopped.load(Ordering::Relaxed) {
            if let Ok(mut preferences) = computy_engine::preferences::load(&service.data_root()) {
                if preferences.language == "system" {
                    preferences.language = sys_locale::get_locales()
                        .find_map(|locale| {
                            let lower = locale.to_lowercase();
                            if lower.starts_with("zh") {
                                return Some(
                                    if lower.contains("tw")
                                        || lower.contains("hk")
                                        || lower.contains("hant")
                                    {
                                        "zh-TW"
                                    } else {
                                        "zh-CN"
                                    }
                                    .into(),
                                );
                            }
                            if lower.starts_with("pt") {
                                return Some("pt-BR".into());
                            }
                            let base = lower.split(['-', '_']).next().unwrap_or("en");
                            [
                                "en", "fr", "es", "de", "it", "nl", "pl", "ru", "uk", "tr", "ar",
                                "hi", "bn", "ur", "id", "vi", "th", "ko", "ja", "fa", "he", "sw",
                            ]
                            .contains(&base)
                            .then(|| base.to_owned())
                        })
                        .unwrap_or_else(|| "en".into());
                }
                let snapshot = if preferences.tray_enabled
                    && (!preferences.tray_components.is_empty()
                        || !preferences.tray_disks.is_empty())
                {
                    service.monitor(preferences.tray_components.iter().any(|c| c == "gpu"))
                } else {
                    Snapshot::default()
                };
                let handle = app.clone();
                if app
                    .run_on_main_thread(move || {
                        if let Err(error) = apply(&handle, &preferences, &snapshot) {
                            eprintln!("Computy tray: {error}");
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}
fn apply(
    app: &tauri::AppHandle,
    preferences: &Preferences,
    snapshot: &Snapshot,
) -> tauri::Result<()> {
    let state = app.state::<TrayState>();
    if state.stopped.load(Ordering::Relaxed) {
        return Ok(());
    }
    let mut items = state.indicators.lock().unwrap_or_else(|e| e.into_inner());
    let had_icons = !items.is_empty();
    let mut values = vec![];
    if preferences.tray_enabled {
        for component in &preferences.tray_components {
            let (label, value) = match component.as_str() {
                "memory" => ("RAM", snapshot.memory_percent),
                "cpu" => ("CPU", snapshot.cpu_percent),
                "gpu" => ("GPU", snapshot.gpu_percent),
                _ => continue,
            };
            values.push((component.clone(), label, value, String::new()));
        }
        for id in &preferences.tray_disks {
            if let Some(disk) = snapshot.disks.iter().find(|disk| &disk.id == id) {
                let rate = |value: Option<f64>| {
                    value
                        .map(|v| format!("{:.1} MiB/s", v / 1048576.0))
                        .unwrap_or_else(|| "--".into())
                };
                values.push((
                    format!("disk:{id}"),
                    "DSK",
                    disk.used_percent,
                    format!(
                        " · {id} · {} · ↓ {} · ↑ {}",
                        message(preferences, "Espace occupé"),
                        rate(disk.read_bytes_per_second),
                        rate(disk.written_bytes_per_second)
                    ),
                ));
            }
        }
    }
    let obsolete: Vec<_> = items
        .iter()
        .filter(|(id, item)| {
            item.language != preferences.language || !values.iter().any(|(key, ..)| key == *id)
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in obsolete {
        if let Some(item) = items.remove(&id) {
            app.remove_tray_by_id(item.tray.id());
        }
    }
    for (id, label, value, extra) in values {
        let text = value
            .map(|v| format!("{v:.0}%"))
            .unwrap_or_else(|| message(preferences, "Indisponible"));
        let detail = format!("Computy · {label} {text}{extra}");
        let metric = if id.starts_with("disk:") {
            "storage"
        } else {
            id.as_str()
        };
        let lines = process_lines(preferences, snapshot, metric);
        let signature = format!("{detail}{lines:?}");
        if !items.contains_key(&id) {
            let (menu, detail_item, process_items) =
                menu(app, preferences, &detail, metric, &lines)?;
            let tray = TrayIconBuilder::with_id(format!("computy-{id}"))
                .icon(Image::new_owned(
                    crate::tray_icon::render(label, value),
                    64,
                    64,
                ))
                .tooltip(&detail)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "monitor-open" => show(app, "overview"),
                    "monitor-settings" => show(app, "settings"),
                    "monitor-quit" => app.exit(0),
                    "monitor-all-memory" => show(app, "memory"),
                    "monitor-all-cpu" => show(app, "cpu"),
                    "monitor-all-gpu" => show(app, "gpu"),
                    "monitor-all-storage" => show(app, "storage"),
                    _ => {}
                })
                .build(app)?;
            items.insert(
                id.clone(),
                Indicator {
                    tray,
                    last: signature.clone(),
                    language: preferences.language.clone(),
                    detail: detail_item,
                    process_items,
                },
            );
        }
        if let Some(item) = items.get_mut(&id)
            && item.last != signature
        {
            item.tray.set_icon(Some(Image::new_owned(
                crate::tray_icon::render(label, value),
                64,
                64,
            )))?;
            item.tray.set_tooltip(Some(&detail))?;
            if item.process_items.len() != lines.len() {
                let (menu, detail_item, process_items) =
                    menu(app, preferences, &detail, metric, &lines)?;
                item.tray.set_menu(Some(menu))?;
                item.detail = detail_item;
                item.process_items = process_items;
            } else {
                item.detail.set_text(&detail)?;
                for (entry, text) in item.process_items.iter().zip(&lines) {
                    entry.set_text(text)?;
                }
            }
            item.last = signature;
        }
    }
    if had_icons
        && items.is_empty()
        && let Some(window) = app.get_webview_window("main")
        && !window.is_visible().unwrap_or(true)
    {
        let _ = window.show();
    }
    Ok(())
}
