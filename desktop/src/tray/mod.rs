//! Tray indicators. A monitoring thread samples the engine every two seconds
//! and updates one indicator per selected metric or volume. Clicks are
//! forwarded to the UI thread, which opens the popover or the main window.
mod icon;
#[cfg(target_os = "linux")]
mod linux_indicator;
#[cfg(target_os = "linux")]
mod linux_menu;

use computy_engine::{
    monitor::Snapshot, peripherals::PeripheralSample, preferences::Preferences, service::AppService,
};
use futures::{StreamExt, channel::mpsc};
use gpui::App;
use std::{
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

/// Message for the UI thread.
enum Message {
    Activate(Activation),
    /// Native indicators must be created and updated on the UI thread.
    #[cfg(not(target_os = "linux"))]
    Sync(Preferences, Vec<Value>),
}

/// Request from an indicator: destination, metric, volume and click position.
struct Activation {
    destination: String,
    metric: String,
    volume: String,
    position: Option<(i32, i32)>,
}

static STOPPED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static WAKE: OnceLock<Mutex<Option<std::sync::mpsc::Sender<()>>>> = OnceLock::new();

pub fn has_icons() -> bool {
    ACTIVE.load(Ordering::Relaxed) > 0
}

pub fn shutdown() {
    STOPPED.store(true, Ordering::Relaxed);
    preferences_wake();
}

fn preferences_wake() {
    if let Some(sender) = WAKE
        .get()
        .and_then(|w| w.lock().ok())
        .and_then(|s| s.clone())
    {
        let _ = sender.send(());
    }
}

/// Apply saved preferences immediately instead of at the next tick.
pub fn preferences_changed(_cx: &mut App) {
    preferences_wake();
}

fn message(preferences: &Preferences, key: &str) -> String {
    let language = crate::i18n::resolve(&preferences.language, &crate::i18n::system_languages());
    crate::i18n::translate_in(language, key)
}

struct Value {
    id: String,
    metric: String,
    volume: String,
    label: &'static str,
    value: Option<f64>,
    battery: Option<Battery>,
}

/// Wireless peripheral shown instead of a metric.
struct Battery {
    icon: &'static str,
    /// False for a coarse level, which is never drawn as a number.
    exact: bool,
    name: String,
    /// Percentage, `≈` estimate or translated level.
    text: String,
    charging: Option<bool>,
}

impl Value {
    fn text(&self) -> String {
        match &self.battery {
            Some(battery) => battery.text.clone(),
            None => self
                .value
                .map(|n| format!("{n:.0}%"))
                .unwrap_or_else(|| "—".into()),
        }
    }

    fn title(&self) -> String {
        match &self.battery {
            Some(battery) => format!(
                "Computy · {} · {}{}",
                battery.name,
                battery.text,
                if battery.charging == Some(true) {
                    " ⚡"
                } else {
                    ""
                }
            ),
            None => format!("Computy · {} · {} {}", self.label, self.text(), self.volume),
        }
    }

    fn pixels(&self, style: &str, compact_text: bool) -> Vec<u8> {
        match &self.battery {
            Some(battery) => {
                icon::render_battery(battery.icon, self.value, battery.exact, style, compact_text)
            }
            None => icon::render_style(self.label, self.value, style, compact_text),
        }
    }
}

fn values(
    preferences: &Preferences,
    snapshot: &Snapshot,
    peripherals: &[PeripheralSample],
) -> Vec<Value> {
    let mut values = vec![];
    if preferences.battery_enabled {
        for device in preferences.battery_devices.iter().filter(|d| d.shown) {
            // A disconnected device has no indicator; it returns on reconnection.
            let Some(sample) = peripherals
                .iter()
                .find(|p| p.id == device.id && p.connected)
            else {
                continue;
            };
            let kind = if device.icon == "auto" {
                sample.kind.as_str()
            } else {
                device.icon.as_str()
            };
            let text = match sample.percent {
                Some(v) if sample.approximate => format!("≈{v:.0}%"),
                Some(v) => format!("{v:.0}%"),
                None => message(preferences, &format!("battery.level_{}", sample.level)),
            };
            values.push(Value {
                id: format!("computy-battery-{}", device.id),
                metric: "battery".into(),
                volume: device.id.clone(),
                label: "BAT",
                value: sample.gauge(),
                battery: Some(Battery {
                    icon: crate::monitor_ui::device_icon(kind),
                    exact: sample.percent.is_some(),
                    name: sample
                        .name
                        .chars()
                        .filter(|c| !c.is_control())
                        .take(45)
                        .collect(),
                    text,
                    charging: sample.charging,
                }),
            });
        }
    }
    if !preferences.tray_enabled {
        return values;
    }
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
            battery: None,
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
            battery: None,
        });
    }
    values
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

#[cfg(target_os = "linux")]
fn process_png(pid: u32) -> Vec<u8> {
    crate::platform::process_icon(pid)
        .filter(|path| path.extension().is_some_and(|e| e == "png"))
        .and_then(|path| std::fs::read(path).ok())
        .filter(|bytes| bytes.len() <= 128 * 1024)
        .unwrap_or_default()
}

/// Actions shared by every indicator menu: (id, key, action, freedesktop icon).
#[cfg(target_os = "linux")]
const ACTIONS: [(i32, &str, &str, &str); 3] = [
    (31, "common.preferences", "settings", "preferences-system"),
    (32, "common.open_computy", "overview", "window-new"),
    (33, "common.quit_computy", "quit", "application-exit"),
];

#[cfg(target_os = "linux")]
fn linux_battery_rows(
    preferences: &Preferences,
    value: &Value,
    battery: &Battery,
) -> Vec<linux_menu::Row> {
    use linux_menu::Row;
    let png =
        encode_rgba(&value.pixels(&preferences.tray_style, false), 64, 64).unwrap_or_default();
    let mut rows = vec![Row {
        id: 1,
        label: format!("{} · {}", battery.name, battery.text),
        png,
        action: "panel".into(),
        ..Default::default()
    }];
    if let Some(charging) = battery.charging {
        rows.push(Row {
            id: 2,
            label: message(
                preferences,
                if charging {
                    "battery.charging"
                } else {
                    "battery.on_battery"
                },
            ),
            ..Default::default()
        });
    }
    rows.push(Row {
        id: 29,
        separator: true,
        ..Default::default()
    });
    for (id, key, action, icon) in ACTIONS {
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

#[cfg(target_os = "linux")]
fn linux_menu_rows(
    preferences: &Preferences,
    snapshot: &Snapshot,
    value: &Value,
) -> Vec<linux_menu::Row> {
    use linux_menu::Row;
    if let Some(battery) = &value.battery {
        return linux_battery_rows(preferences, value, battery);
    }
    let percent = |n: Option<f64>| n.map(|n| format!("{n:.1}%")).unwrap_or_else(|| "—".into());
    let bytes = |n: Option<u64>| {
        n.map(|n| {
            format!(
                "{:.1} {}",
                n as f64 / 1048576.0,
                message(preferences, "common.unit_mio")
            )
        })
        .unwrap_or_else(|| "—".into())
    };
    let metric_name = match value.metric.as_str() {
        "memory" => "common.memory",
        "cpu" => "common.processor",
        "gpu" => "common.graphics_processor",
        _ => "common.storage",
    };
    let png =
        encode_rgba(&value.pixels(&preferences.tray_style, false), 64, 64).unwrap_or_default();
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
            label: message(preferences, "common.top_resource_consumers"),
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
            png: process_png(p.pid),
            ..Default::default()
        });
    }
    if rows.len() == 3 {
        rows.push(Row {
            id: 10,
            label: message(preferences, "common.no_per_process_measurement_available"),
            ..Default::default()
        });
    }
    rows.push(Row {
        id: 29,
        separator: true,
        ..Default::default()
    });
    for (id, key, action, icon) in
        std::iter::once((30, "common.view_all", "processes", "view-list-details")).chain(ACTIONS)
    {
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

/// Start the monitoring thread and the UI-side activation handler.
pub fn start(service: Arc<AppService>, cx: &mut App) {
    let (sender, mut receiver) = mpsc::unbounded::<Message>();
    let (wake_sender, wake) = std::sync::mpsc::channel::<()>();
    let _ = WAKE.set(Mutex::new(Some(wake_sender)));
    std::thread::Builder::new()
        .name("computy-tray".into())
        .spawn(move || run(service, sender, wake))
        .ok();
    #[cfg(not(target_os = "linux"))]
    let forward = sender_for_native(&cx);
    cx.spawn(async move |cx| {
        #[cfg(not(target_os = "linux"))]
        let mut native = native::Indicators::new(forward);
        while let Some(message) = receiver.next().await {
            match message {
                Message::Activate(activation) => {
                    let _ = cx.update(|cx| handle(activation, cx));
                }
                #[cfg(not(target_os = "linux"))]
                Message::Sync(preferences, values) => {
                    ACTIVE.store(native.sync(&preferences, values), Ordering::Relaxed);
                }
            }
        }
    })
    .detach();
}

#[cfg(not(target_os = "linux"))]
fn sender_for_native(_cx: &&mut App) -> mpsc::UnboundedSender<Activation> {
    let (sender, mut receiver) = mpsc::unbounded::<Activation>();
    NATIVE_ACTIVATIONS.with(|_| ());
    std::thread::spawn(move || {
        while let Some(activation) = futures::executor::block_on(receiver.next()) {
            if let Some(forward) = UI_SENDER.get() {
                let _ = forward.unbounded_send(Message::Activate(activation));
            }
        }
    });
    sender
}

#[cfg(not(target_os = "linux"))]
static UI_SENDER: OnceLock<mpsc::UnboundedSender<Message>> = OnceLock::new();

#[cfg(not(target_os = "linux"))]
thread_local!(static NATIVE_ACTIVATIONS: () = ());

fn handle(activation: Activation, cx: &mut App) {
    match activation.destination.as_str() {
        "panel" => crate::popover::open(
            activation.metric,
            activation.volume,
            activation.position,
            cx,
        ),
        "quit" => crate::quit(cx),
        "settings" => crate::open_main_window(Some(crate::app::Page::Settings), cx),
        "processes" => {
            crate::set_requested_metric(activation.metric, cx);
            crate::open_main_window(Some(crate::app::Page::Processes), cx);
        }
        "overview" => crate::open_main_window(Some(crate::app::Page::Overview), cx),
        "ensure-window" if cx.global::<crate::AppWindows>().main.is_none() => {
            crate::open_main_window(None, cx);
        }
        _ => {}
    }
}

fn run(
    service: Arc<AppService>,
    sender: mpsc::UnboundedSender<Message>,
    wake: std::sync::mpsc::Receiver<()>,
) {
    #[cfg(not(target_os = "linux"))]
    let _ = UI_SENDER.set(sender.clone());
    #[cfg(target_os = "linux")]
    let mut indicators: Option<linux_indicator::LinuxIndicators> = None;
    let mut was_active = false;
    while !STOPPED.load(Ordering::Relaxed) {
        if let Ok(preferences) = computy_engine::preferences::load(&service.data_root()) {
            let monitoring = preferences.has_indicators();
            let metrics = preferences.tray_enabled
                && (!preferences.tray_components.is_empty() || !preferences.tray_disks.is_empty());
            let snapshot = if metrics {
                service.monitor(preferences.tray_components.iter().any(|c| c == "gpu"))
            } else {
                Snapshot::default()
            };
            // Cached: re-read only on a system change notification or interval.
            let peripherals = if preferences.battery_enabled
                && preferences.battery_devices.iter().any(|d| d.shown)
            {
                service.peripherals(false)
            } else {
                vec![]
            };
            let values = values(&preferences, &snapshot, &peripherals);
            #[cfg(target_os = "linux")]
            {
                if indicators.is_none() && !values.is_empty() {
                    let forward = sender.clone();
                    let activate: linux_indicator::Activation =
                        Arc::new(move |destination, metric, volume, x, y| {
                            let _ = forward.unbounded_send(Message::Activate(Activation {
                                destination,
                                metric,
                                volume,
                                position: (x >= 0 && y >= 0).then_some((x, y)),
                            }));
                        });
                    match linux_indicator::LinuxIndicators::new(activate) {
                        Ok(value) => indicators = Some(value),
                        Err(error) => eprintln!("Computy tray: {error}"),
                    }
                }
                if let Some(client) = indicators.as_mut() {
                    let data = values
                        .into_iter()
                        .map(|v| {
                            let menu = linux_menu_rows(&preferences, &snapshot, &v);
                            linux_indicator::ItemData {
                                title: v.title(),
                                label: if preferences.tray_style == "icon_text" {
                                    v.text()
                                } else {
                                    String::new()
                                },
                                active: true,
                                pixels: v.pixels(&preferences.tray_style, false),
                                menu,
                                revision: 0,
                                id: v.id,
                                metric: v.metric,
                                volume: v.volume,
                            }
                        })
                        .collect();
                    match client.sync(data) {
                        Ok(count) => ACTIVE.store(count, Ordering::Relaxed),
                        Err(error) => eprintln!("Computy tray: {error}"),
                    }
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = sender.unbounded_send(Message::Sync(preferences.clone(), values));
            }
            let active = ACTIVE.load(Ordering::Relaxed) > 0;
            if was_active && !active && !monitoring {
                // Monitoring was turned off while the window is closed: show it
                // so the application never keeps running invisibly.
                let _ = sender.unbounded_send(Message::Activate(Activation {
                    destination: "ensure-window".into(),
                    metric: String::new(),
                    volume: String::new(),
                    position: None,
                }));
            }
            was_active = active;
        }
        let _ = wake.recv_timeout(Duration::from_secs(2));
    }
}

#[cfg(not(target_os = "linux"))]
mod native {
    //! Windows and macOS indicators through the `tray-icon` crate.
    use super::{Activation, Value, message};
    use computy_engine::preferences::Preferences;
    use futures::channel::mpsc::UnboundedSender;
    use std::collections::BTreeMap;
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
        menu::{Menu, MenuEvent, MenuItem},
    };

    pub struct Indicators {
        items: BTreeMap<String, (TrayIcon, String, String, bool)>,
        sender: UnboundedSender<Activation>,
    }

    impl Indicators {
        pub fn new(sender: UnboundedSender<Activation>) -> Self {
            let forward = sender.clone();
            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                if let TrayIconEvent::Click {
                    id,
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    position,
                    ..
                } = event
                {
                    let id = id.0;
                    let (metric, volume) = parse_id(&id);
                    let _ = forward.unbounded_send(Activation {
                        destination: "panel".into(),
                        metric,
                        volume,
                        position: Some((position.x as i32, position.y as i32)),
                    });
                }
            }));
            let forward = sender.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                let id = event.id.0;
                if let Some((indicator, action)) = id.rsplit_once(':') {
                    let (metric, volume) = parse_id(indicator);
                    let _ = forward.unbounded_send(Activation {
                        destination: action.into(),
                        metric,
                        volume,
                        position: None,
                    });
                }
            }));
            Self {
                items: BTreeMap::new(),
                sender,
            }
        }

        pub fn sync(&mut self, preferences: &Preferences, values: Vec<Value>) -> usize {
            let _ = &self.sender;
            for (id, item) in self.items.iter_mut() {
                if !values.iter().any(|v| &v.id == id) && item.3 {
                    let _ = item.0.set_visible(false);
                    item.3 = false;
                }
            }
            for v in values {
                let pixels = v.pixels(&preferences.tray_style, cfg!(target_os = "windows"));
                let image = Icon::from_rgba(pixels, 64, 64).ok();
                if !self.items.contains_key(&v.id) {
                    let menu = Menu::new();
                    let details = if v.battery.is_some() {
                        "battery.details"
                    } else {
                        "common.top_resource_consumers"
                    };
                    for (action, key) in [
                        ("panel", details),
                        ("settings", "common.preferences"),
                        ("overview", "common.open_computy"),
                        ("quit", "common.quit_computy"),
                    ] {
                        let _ = menu.append(&MenuItem::with_id(
                            format!("{}:{action}", v.id),
                            message(preferences, key),
                            true,
                            None,
                        ));
                    }
                    let mut builder = TrayIconBuilder::new()
                        .with_id(&v.id)
                        .with_menu(Box::new(menu))
                        .with_menu_on_left_click(false);
                    if let Some(image) = image.clone() {
                        builder = builder.with_icon(image);
                    }
                    match builder.build() {
                        Ok(tray) => {
                            self.items.insert(
                                v.id.clone(),
                                (tray, v.metric.clone(), v.volume.clone(), true),
                            );
                        }
                        Err(error) => {
                            eprintln!("Computy tray: {error}");
                            continue;
                        }
                    }
                }
                let item = self.items.get_mut(&v.id).unwrap();
                let _ = item.0.set_icon(image);
                let _ = item.0.set_tooltip(Some(v.title()));
                #[cfg(target_os = "macos")]
                item.0.set_title(if preferences.tray_style == "icon_text" {
                    Some(v.text())
                } else {
                    None
                });
                if !item.3 {
                    let _ = item.0.set_visible(true);
                    item.3 = true;
                }
            }
            self.items.values().filter(|i| i.3).count()
        }
    }

    fn parse_id(id: &str) -> (String, String) {
        if let Some(volume) = id.strip_prefix("computy-disk-") {
            ("storage".into(), volume.into())
        } else if let Some(device) = id.strip_prefix("computy-battery-") {
            ("battery".into(), device.into())
        } else {
            (id.trim_start_matches("computy-").into(), String::new())
        }
    }
}
