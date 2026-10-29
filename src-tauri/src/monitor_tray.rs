use computy_engine::{monitor::Snapshot, preferences::Preferences, service::AppService};
use serde::Serialize;
#[cfg(not(target_os = "linux"))]
use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
#[derive(Clone, Serialize)]
pub struct PanelTarget {
    pub metric: String,
    pub volume: String,
    pub visible: bool,
}
impl Default for PanelTarget {
    fn default() -> Self {
        Self {
            metric: "memory".into(),
            volume: String::new(),
            visible: false,
        }
    }
}
#[derive(Default)]
pub struct TrayState {
    pub stopped: AtomicBool,
    active: AtomicUsize,
    target: Mutex<PanelTarget>,
    #[cfg(target_os = "linux")]
    linux: Mutex<Option<crate::linux_indicator::LinuxIndicators>>,
    #[cfg(not(target_os = "linux"))]
    native: Mutex<BTreeMap<String, NativeIndicator>>,
}
impl TrayState {
    pub fn has_icons(&self) -> bool {
        self.active.load(Ordering::Relaxed) > 0
    }
    pub fn status(&self) -> serde_json::Value {
        #[cfg(target_os = "linux")]
        let items = self
            .linux
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|s| s.entries())
            .unwrap_or_default();
        #[cfg(not(target_os = "linux"))]
        let items: Vec<_> = self
            .native
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(id, i)| serde_json::json!({"id":id,"active":i.active}))
            .collect();
        serde_json::json!({"active":self.active.load(Ordering::Relaxed),"items":items})
    }
}
pub fn setup_panel(app: &tauri::AppHandle) -> tauri::Result<()> {
    WebviewWindowBuilder::new(
        app,
        "monitor-popover",
        WebviewUrl::App("index.html?popover=1".into()),
    )
    .title("Computy")
    .inner_size(420.0, 720.0)
    .decorations(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .visible(false)
    .shadow(false)
    .transparent(true)
    .background_color(tauri::window::Color(0, 0, 0, 0))
    .build()?;
    Ok(())
}
pub fn panel_target(app: &tauri::AppHandle) -> PanelTarget {
    let mut target = app
        .state::<TrayState>()
        .target
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    target.visible = app
        .get_webview_window("monitor-popover")
        .is_some_and(|w| w.is_visible().unwrap_or(false));
    target
}
pub fn open_panel(app: &tauri::AppHandle, metric: &str, volume: &str, x: i32, y: i32) {
    let Some(window) = app.get_webview_window("monitor-popover") else {
        return;
    };
    let target = PanelTarget {
        metric: metric.into(),
        volume: volume.into(),
        visible: true,
    };
    *app.state::<TrayState>()
        .target
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = target;
    let point = if x >= 0 && y >= 0 {
        (x, y)
    } else {
        app.cursor_position()
            .map(|p| (p.x as i32, p.y as i32))
            .unwrap_or((500, 30))
    };
    if let Ok(monitors) = app.available_monitors() {
        let monitor = monitors
            .iter()
            .find(|m| {
                let p = m.position();
                let s = m.size();
                point.0 >= p.x
                    && point.1 >= p.y
                    && point.0 < p.x + s.width as i32
                    && point.1 < p.y + s.height as i32
            })
            .or_else(|| monitors.first());
        if let Some(monitor) = monitor {
            let origin = monitor.position();
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let width = (420.0 * scale) as i32;
            let height = (720.0 * scale)
                .min(f64::from(size.height) - 48.0)
                .max(300.0) as i32;
            let _ = window.set_size(tauri::PhysicalSize::new(width as u32, height as u32));
            let px = (point.0 - width / 2).clamp(
                origin.x + 8,
                (origin.x + size.width as i32 - width - 8).max(origin.x + 8),
            );
            let py = if point.1 < origin.y + size.height as i32 / 2 {
                point.1 + 12
            } else {
                point.1 - height - 12
            };
            let py = py.clamp(
                origin.y + 8,
                (origin.y + size.height as i32 - height - 8).max(origin.y + 8),
            );
            let _ = window.set_position(tauri::PhysicalPosition::new(px, py));
        }
    }
    let _ = window.show();
    let _ = window.set_focus();
}
pub fn panel_action(app: &tauri::AppHandle, destination: &str) -> Result<(), String> {
    let target = panel_target(app);
    let route = match destination {
        "hide" => {
            if let Some(w) = app.get_webview_window("monitor-popover") {
                let _ = w.hide();
            }
            return Ok(());
        }
        "quit" => {
            app.exit(0);
            return Ok(());
        }
        "overview" => "window.location.hash='overview'",
        "settings" => "window.location.hash='settings'",
        "processes" => match target.metric.as_str() {
            "cpu" => "window.location.hash='processes?metric=cpu'",
            "gpu" => "window.location.hash='processes?metric=gpu'",
            "storage" => "window.location.hash='processes?metric=storage'",
            _ => "window.location.hash='processes?metric=memory'",
        },
        _ => return Err("Commande non autorisée.".into()),
    };
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(target_os = "macos")]
        app.set_activation_policy(tauri::ActivationPolicy::Regular)
            .map_err(|e| e.to_string())?;
        window.eval(route).map_err(|e| e.to_string())?;
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    if let Some(window) = app.get_webview_window("monitor-popover") {
        let _ = window.hide();
    }
    Ok(())
}
fn resolved_preferences(mut preferences: Preferences) -> Preferences {
    if preferences.language == "system" {
        preferences.language = sys_locale::get_locales()
            .find_map(|locale| {
                let lower = locale.to_lowercase();
                if lower.starts_with("zh") {
                    return Some(
                        if lower.contains("tw") || lower.contains("hk") || lower.contains("hant") {
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
                    "en", "fr", "es", "de", "it", "nl", "pl", "ru", "uk", "tr", "ar", "hi", "bn",
                    "ur", "id", "vi", "th", "ko", "ja", "fa", "he", "sw",
                ]
                .contains(&base)
                .then(|| base.to_owned())
            })
            .unwrap_or_else(|| "en".into());
    }
    preferences
}
fn message(preferences: &Preferences, key: &str) -> String {
    if preferences.language == "fr" {
        return key.into();
    }
    static CATALOGS: OnceLock<serde_json::Value> = OnceLock::new();
    CATALOGS
        .get_or_init(|| {
            serde_json::from_str(include_str!("../../src/locales/catalogs.json"))
                .unwrap_or_default()
        })
        .get(&preferences.language)
        .and_then(|c| c.get(key))
        .or_else(|| {
            CATALOGS
                .get()
                .and_then(|c| c.get("en"))
                .and_then(|c| c.get(key))
        })
        .and_then(|s| s.as_str())
        .unwrap_or(key)
        .into()
}
struct Value {
    id: String,
    metric: String,
    volume: String,
    label: &'static str,
    value: Option<f64>,
}
#[cfg(target_os = "linux")]
fn linux_menu_rows(
    preferences: &Preferences,
    snapshot: &Snapshot,
    value: &Value,
) -> Vec<crate::linux_menu::Row> {
    use crate::linux_menu::Row;
    let percent = |n: Option<f64>| n.map(|n| format!("{n:.1}%")).unwrap_or_else(|| "—".into());
    let bytes = |n: Option<u64>| {
        n.map(|n| {
            format!(
                "{:.1} {}",
                n as f64 / 1048576.0,
                message(preferences, "Mio")
            )
        })
        .unwrap_or_else(|| "—".into())
    };
    let metric_name = match value.metric.as_str() {
        "memory" => "Mémoire vive",
        "cpu" => "Processeur",
        "gpu" => "Processeur graphique",
        _ => "Stockage",
    };
    let png = crate::process_icons::encode_rgba(
        &crate::tray_icon::render_style(value.label, value.value, &preferences.tray_style, false),
        64,
        64,
    )
    .unwrap_or_default();
    let mut rows = vec![
        Row {
            id: 1,
            label: format!(
                "{} · {} — {}",
                value.label,
                percent(value.value),
                message(preferences, metric_name)
            ),
            png,
            action: "panel".into(),
            ..Default::default()
        },
        Row {
            id: 2,
            separator: true,
            ..Default::default()
        },
        Row {
            id: 3,
            label: message(preferences, "Processus les plus consommateurs"),
            ..Default::default()
        },
    ];
    for (i, p) in computy_engine::monitor::top_processes(snapshot, &value.metric)
        .iter()
        .enumerate()
    {
        let name: String = p
            .name
            .chars()
            .filter(|c| !c.is_control())
            .take(45)
            .collect();
        let amount = match value.metric.as_str() {
            "memory" => format!("{} · {}", bytes(p.memory_bytes), percent(p.memory_percent)),
            "cpu" => format!("{} · RAM {}", percent(p.cpu_percent), bytes(p.memory_bytes)),
            "gpu" => format!("VRAM {}", bytes(p.gpu_memory_bytes)),
            _ => format!(
                "{:.2} MiB/s · {}",
                (p.read_bytes_per_second.unwrap_or(0.0)
                    + p.written_bytes_per_second.unwrap_or(0.0))
                    / 1048576.0,
                percent(p.io_percent)
            ),
        };
        rows.push(Row {
            id: 10 + i as i32,
            label: format!("{name} · {amount}"),
            png: crate::process_icons::cached_png(p.pid),
            ..Default::default()
        });
    }
    if rows.len() == 3 {
        rows.push(Row {
            id: 10,
            label: message(preferences, "Aucune mesure par processus disponible."),
            ..Default::default()
        });
    }
    rows.push(Row {
        id: 29,
        separator: true,
        ..Default::default()
    });
    for (id, key, action, icon) in [
        (30, "Voir tout", "processes", "view-list-details"),
        (31, "Préférences", "settings", "preferences-system"),
        (32, "Ouvrir Computy", "overview", "window-new"),
        (33, "Quitter Computy", "quit", "application-exit"),
    ] {
        rows.push(Row {
            id,
            label: message(preferences, key),
            action: action.into(),
            icon: icon.into(),
            ..Default::default()
        });
    }
    rows
}
fn values(preferences: &Preferences, snapshot: &Snapshot) -> Vec<Value> {
    if !preferences.tray_enabled {
        return vec![];
    }
    let mut values = vec![];
    for component in &preferences.tray_components {
        let (label, value) = match component.as_str() {
            "memory" => ("RAM", snapshot.memory_percent),
            "cpu" => ("CPU", snapshot.cpu_percent),
            "gpu" => ("GPU", snapshot.gpu_percent),
            _ => continue,
        };
        values.push(Value {
            id: format!("computy-{component}"),
            metric: component.clone(),
            volume: String::new(),
            label,
            value,
        });
    }
    for id in &preferences.tray_disks {
        values.push(Value {
            id: format!("computy-disk-{id}"),
            metric: "storage".into(),
            volume: id.clone(),
            label: "DSK",
            value: snapshot
                .disks
                .iter()
                .find(|d| &d.id == id)
                .and_then(|d| d.used_percent),
        });
    }
    values
}
pub fn start(app: tauri::AppHandle, service: Arc<AppService>) {
    std::thread::spawn(move || {
        while !app.state::<TrayState>().stopped.load(Ordering::Relaxed) {
            if let Ok(preferences) = computy_engine::preferences::load(&service.data_root()) {
                let preferences = resolved_preferences(preferences);
                let snapshot = if preferences.tray_enabled
                    && (!preferences.tray_components.is_empty()
                        || !preferences.tray_disks.is_empty())
                {
                    service.monitor(preferences.tray_components.iter().any(|c| c == "gpu"))
                } else {
                    Snapshot::default()
                };
                let values = values(&preferences, &snapshot);
                #[cfg(target_os = "linux")]
                let monitoring_disabled = values.is_empty();
                #[cfg(target_os = "linux")]
                {
                    let state = app.state::<TrayState>();
                    let was_active = state.has_icons();
                    let mut client = state.linux.lock().unwrap_or_else(|e| e.into_inner());
                    if client.is_none() {
                        let handle = app.clone();
                        let activate = Arc::new(
                            move |destination: String,
                                  metric: String,
                                  volume: String,
                                  x: i32,
                                  y: i32| {
                                let app = handle.clone();
                                let task = app.clone();
                                let _ = app.run_on_main_thread(move || {
                                    if destination == "panel" {
                                        open_panel(&task, &metric, &volume, x, y);
                                    } else {
                                        *task
                                            .state::<TrayState>()
                                            .target
                                            .lock()
                                            .unwrap_or_else(|e| e.into_inner()) = PanelTarget {
                                            metric,
                                            volume,
                                            visible: false,
                                        };
                                        let _ = panel_action(&task, &destination);
                                    }
                                });
                            },
                        );
                        match crate::linux_indicator::LinuxIndicators::new(activate) {
                            Ok(value) => *client = Some(value),
                            Err(error) => eprintln!("Computy tray: {error}"),
                        }
                    }
                    if let Some(client) = client.as_mut() {
                        let pids: Vec<_> = values
                            .iter()
                            .flat_map(|v| {
                                computy_engine::monitor::top_processes(&snapshot, &v.metric)
                                    .into_iter()
                                    .map(|p| p.pid)
                            })
                            .collect::<std::collections::BTreeSet<_>>()
                            .into_iter()
                            .take(32)
                            .collect();
                        let _ = app.run_on_main_thread(move || {
                            let _ = crate::process_icons::process_icons(pids);
                        });
                        let data = values
                            .into_iter()
                            .map(|v| {
                                let text = v
                                    .value
                                    .map(|n| format!("{n:.0}%"))
                                    .unwrap_or_else(|| "—".into());
                                let menu = linux_menu_rows(&preferences, &snapshot, &v);
                                crate::linux_indicator::ItemData {
                                    id: v.id,
                                    metric: v.metric,
                                    volume: v.volume.clone(),
                                    title: format!("Computy · {} · {text} {}", v.label, v.volume),
                                    label: if preferences.tray_style == "icon_text" {
                                        text
                                    } else {
                                        String::new()
                                    },
                                    active: true,
                                    pixels: crate::tray_icon::render_style(
                                        v.label,
                                        v.value,
                                        &preferences.tray_style,
                                        false,
                                    ),
                                    menu,
                                    revision: 0,
                                }
                            })
                            .collect();
                        match client.sync(data) {
                            Ok(count) => state.active.store(count, Ordering::Relaxed),
                            Err(error) => eprintln!("Computy tray: {error}"),
                        }
                    }
                    if was_active && !state.has_icons() && monitoring_disabled {
                        let handle = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            if let Some(w) = handle.get_webview_window("main")
                                && !w.is_visible().unwrap_or(true)
                            {
                                let _ = w.show();
                            }
                        });
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        if let Err(error) = apply_native(&handle, &preferences, values) {
                            eprintln!("Computy tray: {error}");
                        }
                    });
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}
#[cfg(not(target_os = "linux"))]
struct NativeIndicator {
    tray: tauri::tray::TrayIcon,
    active: bool,
}
#[cfg(not(target_os = "linux"))]
fn apply_native(
    app: &tauri::AppHandle,
    preferences: &Preferences,
    values: Vec<Value>,
) -> tauri::Result<()> {
    use tauri::{
        image::Image,
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };
    let state = app.state::<TrayState>();
    let mut items = state.native.lock().unwrap_or_else(|e| e.into_inner());
    for (id, item) in items.iter_mut() {
        if !values.iter().any(|v| &v.id == id) && item.active {
            item.tray.set_visible(false)?;
            item.active = false;
        }
    }
    for v in values {
        let text = v
            .value
            .map(|n| format!("{n:.0}%"))
            .unwrap_or_else(|| "—".into());
        if !items.contains_key(&v.id) {
            let menu = Menu::new(app)?;
            for (id, key) in [
                ("panel", "Processus les plus consommateurs"),
                ("settings", "Préférences"),
                ("overview", "Ouvrir Computy"),
                ("quit", "Quitter Computy"),
            ] {
                menu.append(&MenuItem::with_id(
                    app,
                    format!("{}:{id}", v.id),
                    message(preferences, key),
                    true,
                    None::<&str>,
                )?)?;
            }
            let metric = v.metric.clone();
            let volume = v.volume.clone();
            let cm = metric.clone();
            let cv = volume.clone();
            let event_prefix = format!("{}:", v.id);
            let tray = TrayIconBuilder::with_id(&v.id)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        open_panel(
                            tray.app_handle(),
                            &metric,
                            &volume,
                            position.x as i32,
                            position.y as i32,
                        );
                    }
                })
                .on_menu_event(move |app, event| {
                    let Some(action) = event.id.as_ref().strip_prefix(&event_prefix) else {
                        return;
                    };
                    if action == "panel" {
                        open_panel(app, &cm, &cv, -1, -1);
                    } else {
                        let _ = panel_action(app, action);
                    }
                })
                .build(app)?;
            items.insert(v.id.clone(), NativeIndicator { tray, active: true });
        }
        let item = items.get_mut(&v.id).unwrap();
        item.tray.set_icon(Some(Image::new_owned(
            crate::tray_icon::render_style(
                v.label,
                v.value,
                &preferences.tray_style,
                cfg!(target_os = "windows"),
            ),
            64,
            64,
        )))?;
        item.tray
            .set_tooltip(Some(format!("Computy · {} · {text} {}", v.label, v.volume)))?;
        item.tray
            .set_title(if preferences.tray_style == "icon_text" {
                Some(text)
            } else {
                None
            })?;
        if !item.active {
            item.tray.set_visible(true)?;
            item.active = true;
        }
    }
    state.active.store(
        items.values().filter(|i| i.active).count(),
        Ordering::Relaxed,
    );
    Ok(())
}
