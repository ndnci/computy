//! Battery of wireless peripherals (mice, keyboards, headsets…).
//!
//! Values come from caches the operating system already maintains (UPower and
//! BlueZ on Linux, Plug and Play properties on Windows, the Bluetooth system
//! profile on macOS). Reading them never wakes the device radio. Coarse levels
//! reported by some firmware stay levels: they are never turned into a precise
//! percentage. Hardware addresses never leave this module: devices are
//! identified by a truncated hash.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

/// Normalized device kinds, also the icons a user may choose.
pub const KINDS: &[&str] = &[
    "mouse",
    "keyboard",
    "headphones",
    "headset",
    "speaker",
    "gamepad",
    "phone",
    "tablet",
    "pen",
    "watch",
    "other",
];

/// Coarse levels, from the most to the least depleted.
pub const LEVELS: &[&str] = &["critical", "low", "normal", "high", "full"];

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct PeripheralSample {
    /// Stable opaque identifier (hash of the hardware address or serial).
    pub id: String,
    pub name: String,
    /// One of [`KINDS`].
    pub kind: String,
    /// Charge in percent when the device reports one.
    pub percent: Option<f64>,
    /// Coarse level ([`LEVELS`]) when the device only reports a level.
    pub level: String,
    /// The percentage is estimated (for example from the battery voltage).
    pub approximate: bool,
    pub charging: Option<bool>,
    pub connected: bool,
}

impl PeripheralSample {
    /// Value used for gauges and colors. A level maps to the middle of its
    /// range; it is never displayed as a percentage.
    pub fn gauge(&self) -> Option<f64> {
        self.percent.or_else(|| level_gauge(&self.level))
    }
}

pub fn level_gauge(level: &str) -> Option<f64> {
    Some(match level {
        "critical" => 5.0,
        "low" => 20.0,
        "normal" => 55.0,
        "high" => 80.0,
        "full" => 100.0,
        _ => return None,
    })
}

/// Opaque identifier; the raw address is not recoverable from it.
pub fn device_id(key: &str) -> String {
    let digest = Sha256::digest(format!("computy-peripheral|{}", key.trim()).as_bytes());
    digest[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// `aa:bb:cc:dd:ee:ff`, `AA_BB_…` or `aabbccddeeff` → `AA:BB:CC:DD:EE:FF`.
pub fn normalize_address(value: &str) -> Option<String> {
    let hex: String = value
        .chars()
        .filter(|c| !matches!(c, ':' | '_' | '-'))
        .collect();
    (hex.len() == 12 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| {
        hex.to_ascii_uppercase()
            .as_bytes()
            .chunks(2)
            .map(|pair| String::from_utf8_lossy(pair).into_owned())
            .collect::<Vec<_>>()
            .join(":")
    })
}

/// BlueZ `Device1.Icon` (freedesktop icon names).
pub fn kind_from_icon(icon: &str) -> Option<&'static str> {
    Some(match icon {
        "input-mouse" => "mouse",
        "input-keyboard" => "keyboard",
        "audio-headphones" => "headphones",
        "audio-headset" => "headset",
        "audio-card" | "audio-speakers" => "speaker",
        "input-gaming" => "gamepad",
        "phone" => "phone",
        "input-tablet" => "tablet",
        _ => return None,
    })
}

/// UPower `Device.Type`. `None` for power sources that are not peripherals.
pub fn kind_from_upower_type(kind: u32) -> Option<&'static str> {
    Some(match kind {
        5 | 14 => "mouse",
        6 => "keyboard",
        8 => "phone",
        10 => "tablet",
        12 => "gamepad",
        13 => "pen",
        17 => "headset",
        18 | 21 => "speaker",
        19 => "headphones",
        26 => "watch",
        7 | 9 | 20 | 22 | 25 | 27 | 28 => "other",
        _ => return None,
    })
}

/// Bluetooth Class of Device (major and minor classes).
pub fn kind_from_class(class: u32) -> Option<&'static str> {
    let minor = (class >> 2) & 0x3f;
    Some(match (class >> 8) & 0x1f {
        0x02 => "phone",
        0x04 => match minor {
            0x01 | 0x02 => "headset",
            0x06 => "headphones",
            0x05 | 0x07 | 0x0a => "speaker",
            _ => "other",
        },
        0x05 => match (minor >> 4, minor & 0x0f) {
            (_, 0x01 | 0x02) => "gamepad",
            (_, 0x05) => "tablet",
            (_, 0x06) => "other",
            (0x01, _) | (0x03, _) => "keyboard",
            (0x02, _) => "mouse",
            _ => "other",
        },
        0x07 if minor == 0x01 => "watch",
        0x07 | 0x08 => "other",
        _ => return None,
    })
}

/// macOS `device_minorType` and similar free-form names.
pub fn kind_from_name(name: &str) -> &'static str {
    let name = name.to_ascii_lowercase();
    [
        ("mouse", "mouse"),
        ("trackpad", "mouse"),
        ("keyboard", "keyboard"),
        ("headset", "headset"),
        ("headphone", "headphones"),
        ("earbud", "headphones"),
        ("airpods", "headphones"),
        ("speaker", "speaker"),
        ("gamepad", "gamepad"),
        ("controller", "gamepad"),
        ("joystick", "gamepad"),
        ("phone", "phone"),
        ("tablet", "tablet"),
        ("pen", "pen"),
        ("pencil", "pen"),
        ("watch", "watch"),
    ]
    .iter()
    .find(|(needle, _)| name.contains(needle))
    .map_or("other", |(_, kind)| kind)
}

/// UPower `Device.BatteryLevel`. `None` when a real percentage is reported.
pub fn upower_level(level: u32) -> Option<&'static str> {
    Some(match level {
        4 => "critical",
        3 => "low",
        6 => "normal",
        7 => "high",
        8 => "full",
        _ => return None,
    })
}

/// UPower `Device.State`.
pub fn upower_charging(state: u32) -> Option<bool> {
    match state {
        1 | 5 => Some(true),
        2..=4 | 6 => Some(false),
        _ => None,
    }
}

/// Whether a percentage reported by another source agrees with a coarse level.
/// Some firmware publishes a stale or rounded value through the Bluetooth
/// battery service; a level from the device itself wins when they disagree.
pub fn consistent_with_level(percent: f64, level: &str) -> bool {
    let (low, high) = match level {
        "critical" => (0.0, 10.0),
        "low" => (0.0, 30.0),
        "normal" => (20.0, 95.0),
        "high" => (60.0, 99.0),
        "full" => (85.0, 100.0),
        _ => (0.0, 100.0),
    };
    (low..=high).contains(&percent)
}

/// Single-cell lithium battery voltage (V) → estimated charge, as Solaar does.
pub fn percent_from_voltage(volts: f64) -> Option<f64> {
    const CURVE: [(f64, f64); 13] = [
        (4.186, 100.0),
        (4.067, 90.0),
        (3.989, 80.0),
        (3.922, 70.0),
        (3.859, 60.0),
        (3.811, 50.0),
        (3.778, 40.0),
        (3.751, 30.0),
        (3.717, 20.0),
        (3.671, 10.0),
        (3.646, 5.0),
        (3.579, 2.0),
        (3.500, 0.0),
    ];
    if !(2.5..=4.5).contains(&volts) {
        return None;
    }
    if volts >= CURVE[0].0 {
        return Some(100.0);
    }
    for pair in CURVE.windows(2) {
        let ((v1, p1), (v0, p0)) = (pair[0], pair[1]);
        if volts >= v0 {
            return Some((p0 + (volts - v0) / (v1 - v0) * (p1 - p0)).round());
        }
    }
    Some(0.0)
}

fn valid_percent(value: f64) -> Option<f64> {
    (value.is_finite() && (0.0..=100.0).contains(&value)).then_some(value)
}

fn clean_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_control())
        .take(64)
        .collect::<String>()
        .trim()
        .to_owned()
}

/// A device as reported by UPower.
#[derive(Debug, Clone, Default)]
pub struct UpowerDevice {
    pub native_path: String,
    pub serial: String,
    pub model: String,
    pub kind: u32,
    pub power_supply: bool,
    pub present: bool,
    pub percentage: f64,
    pub level: u32,
    pub state: u32,
    pub voltage: f64,
}

/// A device as reported by BlueZ.
#[derive(Debug, Clone, Default)]
pub struct BluezDevice {
    pub address: String,
    pub name: String,
    pub icon: String,
    pub class: u32,
    pub connected: bool,
    pub battery: Option<u8>,
}

/// Merge UPower and BlueZ views of the same devices (matched by address).
/// Only connected devices with battery information are returned.
pub fn merge(upower: &[UpowerDevice], bluez: &[BluezDevice]) -> Vec<PeripheralSample> {
    let mut samples: Vec<(String, PeripheralSample)> = vec![];
    for device in upower {
        if device.power_supply || !device.present {
            continue;
        }
        let Some(kind) = kind_from_upower_type(device.kind) else {
            continue;
        };
        let address = normalize_address(&device.serial).or_else(|| {
            device
                .native_path
                .rsplit_once("dev_")
                .and_then(|(_, a)| normalize_address(a))
        });
        let key = address.unwrap_or_else(|| format!("upower:{}", device.native_path));
        let level = upower_level(device.level).unwrap_or_default();
        let mut sample = PeripheralSample {
            id: device_id(&key),
            name: clean_name(&device.model),
            kind: kind.into(),
            charging: upower_charging(device.state),
            connected: true,
            ..Default::default()
        };
        if level.is_empty() {
            sample.percent = valid_percent(device.percentage);
        } else if let Some(percent) = percent_from_voltage(device.voltage) {
            sample.percent = Some(percent);
            sample.approximate = true;
        } else {
            sample.level = level.into();
        }
        if sample.percent.is_none() && sample.level.is_empty() {
            continue;
        }
        match samples.iter_mut().find(|(k, _)| *k == key) {
            // A device listed twice keeps the most precise value that agrees
            // with the level reported by the device itself.
            Some((_, existing)) => {
                let (precise, coarse) = if existing.percent.is_none() {
                    (sample, existing)
                } else {
                    (existing.clone(), existing)
                };
                if let Some(percent) = precise.percent
                    && consistent_with_level(percent, &coarse.level)
                {
                    let name = std::mem::take(&mut coarse.name);
                    *coarse = precise;
                    if coarse.name.is_empty() {
                        coarse.name = name;
                    }
                }
            }
            None => samples.push((key, sample)),
        }
    }
    for device in bluez {
        let Some(address) = normalize_address(&device.address) else {
            continue;
        };
        let kind = kind_from_icon(&device.icon)
            .or_else(|| kind_from_class(device.class))
            .unwrap_or("other");
        let name = clean_name(&device.name);
        match samples.iter_mut().find(|(k, _)| *k == address) {
            Some((_, sample)) => {
                if !name.is_empty() {
                    sample.name = name;
                }
                if sample.kind == "other" {
                    sample.kind = kind.into();
                }
                if sample.percent.is_none()
                    && let Some(battery) = device.battery.map(f64::from).and_then(valid_percent)
                    && consistent_with_level(battery, &sample.level)
                {
                    sample.percent = Some(battery);
                    sample.level.clear();
                    sample.approximate = false;
                }
                sample.connected = sample.connected || device.connected;
            }
            None if device.connected => {
                if let Some(battery) = device.battery.map(f64::from).and_then(valid_percent) {
                    samples.push((
                        address.clone(),
                        PeripheralSample {
                            id: device_id(&address),
                            name,
                            kind: kind.into(),
                            percent: Some(battery),
                            connected: true,
                            ..Default::default()
                        },
                    ));
                }
            }
            None => {}
        }
    }
    let mut samples: Vec<PeripheralSample> = samples.into_iter().map(|(_, s)| s).collect();
    for sample in &mut samples {
        if sample.name.is_empty() {
            sample.name = sample.kind.clone();
        }
    }
    samples.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    samples
}

/// Battery percentage string such as `"85%"` or `"85 %"`.
pub fn parse_percent_text(text: &str) -> Option<f64> {
    valid_percent(text.trim().trim_end_matches('%').trim().parse().ok()?)
}

/// `system_profiler SPBluetoothDataType -json` (macOS).
pub fn parse_macos_profile(json: &str) -> Vec<PeripheralSample> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return vec![];
    };
    let mut samples = vec![];
    for controller in value["SPBluetoothDataType"]
        .as_array()
        .into_iter()
        .flatten()
    {
        for (group, connected) in [("device_connected", true), ("device_not_connected", false)] {
            for entry in controller[group].as_array().into_iter().flatten() {
                for (name, device) in entry.as_object().into_iter().flatten() {
                    let battery = |key: &str| device[key].as_str().and_then(parse_percent_text);
                    let percent = battery("device_batteryLevelMain")
                        .or_else(|| battery("device_batteryLevel"))
                        .or_else(|| {
                            match (
                                battery("device_batteryLevelLeft"),
                                battery("device_batteryLevelRight"),
                            ) {
                                (Some(l), Some(r)) => Some(l.min(r)),
                                (l, r) => l.or(r),
                            }
                        });
                    let Some(percent) = percent else { continue };
                    let address = device["device_address"].as_str().unwrap_or(name);
                    let key = normalize_address(address).unwrap_or_else(|| name.clone());
                    samples.push(PeripheralSample {
                        id: device_id(&key),
                        name: clean_name(name),
                        kind: kind_from_name(
                            device["device_minorType"]
                                .as_str()
                                .filter(|t| !t.is_empty())
                                .unwrap_or(name),
                        )
                        .into(),
                        percent: Some(percent),
                        connected,
                        ..Default::default()
                    });
                }
            }
        }
    }
    samples.retain(|s| s.connected);
    samples
}

/// JSON produced by the Windows Plug and Play query (see [`read_windows`]).
pub fn parse_windows_devices(json: &str) -> Vec<PeripheralSample> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return vec![];
    };
    let items = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(_) => vec![value],
        _ => vec![],
    };
    let mut samples: Vec<PeripheralSample> = vec![];
    for item in items {
        let Some(percent) = item["battery"].as_f64().and_then(valid_percent) else {
            continue;
        };
        if item["connected"].as_bool() == Some(false) {
            continue;
        }
        let name = clean_name(item["name"].as_str().unwrap_or_default());
        let instance = item["id"].as_str().unwrap_or_default();
        // Instance identifiers end with the hardware address; hash, never expose.
        let key = instance
            .rsplit(['_', '&', '\\'])
            .find_map(normalize_address)
            .unwrap_or_else(|| instance.to_ascii_uppercase());
        let kind = item["cod"]
            .as_u64()
            .and_then(|c| kind_from_class(c as u32))
            .unwrap_or_else(|| kind_from_name(&name));
        let id = device_id(&key);
        if samples.iter().any(|s| s.id == id) {
            continue;
        }
        samples.push(PeripheralSample {
            id,
            name: if name.is_empty() { kind.into() } else { name },
            kind: kind.into(),
            percent: Some(percent),
            connected: true,
            ..Default::default()
        });
    }
    samples
}

/// Cached reader. Refreshes when the system signals a change (Linux) or at an
/// interval that grows on battery power where reading spawns a process.
#[derive(Default)]
pub struct Reader {
    samples: Vec<PeripheralSample>,
    read_at: Option<Instant>,
    #[cfg(target_os = "linux")]
    connection: Option<zbus::blocking::Connection>,
}

impl Reader {
    pub fn sample(&mut self, force: bool) -> Vec<PeripheralSample> {
        let now = Instant::now();
        #[cfg(target_os = "linux")]
        let changed = linux::watch();
        #[cfg(not(target_os = "linux"))]
        let changed = false;
        if force
            || changed
            || self
                .read_at
                .is_none_or(|t| now.duration_since(t) >= interval())
        {
            self.samples = self.read();
            self.read_at = Some(now);
        }
        self.samples.clone()
    }

    fn read(&mut self) -> Vec<PeripheralSample> {
        #[cfg(target_os = "linux")]
        {
            if self.connection.is_none() {
                self.connection = linux::connect();
            }
            match self.connection.as_ref().map(linux::read) {
                Some(Ok(samples)) => samples,
                _ => {
                    self.connection = None;
                    vec![]
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            read_windows()
        }
        #[cfg(target_os = "macos")]
        {
            crate::inventory::command(
                "/usr/sbin/system_profiler",
                &["SPBluetoothDataType", "-json"],
                Duration::from_secs(10),
            )
            .ok()
            .filter(|out| out.code == 0)
            .map(|out| parse_macos_profile(&out.stdout))
            .unwrap_or_default()
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            vec![]
        }
    }
}

/// Delay before re-reading without a change notification.
fn interval() -> Duration {
    #[cfg(target_os = "linux")]
    {
        // Signals drive updates; poll often only when they are unavailable.
        Duration::from_secs(if linux::watching() { 300 } else { 10 })
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Reading spawns a system tool: slow down further on battery power.
        Duration::from_secs(if on_battery() { 300 } else { 120 })
    }
}

#[cfg(target_os = "windows")]
fn on_battery() -> bool {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    // ACLineStatus: 0 offline, 1 online, 255 unknown.
    unsafe { GetSystemPowerStatus(&mut status) != 0 && status.ACLineStatus == 0 }
}

#[cfg(target_os = "macos")]
fn on_battery() -> bool {
    crate::inventory::command("/usr/bin/pmset", &["-g", "ps"], Duration::from_secs(2))
        .is_ok_and(|out| out.stdout.contains("'Battery Power'"))
}

#[cfg(target_os = "windows")]
fn read_windows() -> Vec<PeripheralSample> {
    let root = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:\\Windows".into());
    let program = root.join("System32/WindowsPowerShell/v1.0/powershell.exe");
    // Battery: DEVPKEY_Bluetooth_Battery. Class: DEVPKEY_Bluetooth_ClassOfDevice.
    // Connected: DEVPKEY_Device_IsConnected. Fixed script, no user input.
    let script = "$b='{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2';$c='{2BD67D8B-8BEB-48D5-87E0-6CDA3428040A} 10';$n='{83DA6326-97A6-4088-9453-A1923F573B29} 15';$r=@(Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue | Where-Object { $_.InstanceId -match '^(BTHENUM|BTHLE)\\\\' } | ForEach-Object { $v=(Get-PnpDeviceProperty -InstanceId $_.InstanceId -KeyName $b -ErrorAction SilentlyContinue).Data; if($null -ne $v){ [pscustomobject]@{name=$_.FriendlyName;id=$_.InstanceId;battery=[int]$v;cod=(Get-PnpDeviceProperty -InstanceId $_.InstanceId -KeyName $c -ErrorAction SilentlyContinue).Data;connected=(Get-PnpDeviceProperty -InstanceId $_.InstanceId -KeyName $n -ErrorAction SilentlyContinue).Data} } });ConvertTo-Json -InputObject $r -Compress";
    program
        .to_str()
        .and_then(|p| {
            crate::inventory::command(
                p,
                &["-NoProfile", "-NonInteractive", "-Command", script],
                Duration::from_secs(15),
            )
            .ok()
        })
        .filter(|out| out.code == 0)
        .map(|out| parse_windows_devices(&out.stdout))
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{BluezDevice, PeripheralSample, UpowerDevice};
    use std::{
        collections::HashMap,
        sync::{
            Once,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    use zbus::{
        blocking::Connection,
        zvariant::{OwnedObjectPath, OwnedValue},
    };

    type Properties = HashMap<String, OwnedValue>;

    static CHANGED: AtomicBool = AtomicBool::new(true);
    static WATCHING: AtomicBool = AtomicBool::new(false);
    static START: Once = Once::new();

    pub fn connect() -> Option<Connection> {
        zbus::blocking::connection::Builder::system()
            .ok()?
            .method_timeout(Duration::from_secs(2))
            .build()
            .ok()
    }

    /// Start the change listener once; report and clear pending changes.
    pub fn watch() -> bool {
        START.call_once(|| {
            let _ = std::thread::Builder::new()
                .name("computy-peripherals".into())
                .spawn(listen);
        });
        CHANGED.swap(false, Ordering::Relaxed)
    }

    pub fn watching() -> bool {
        WATCHING.load(Ordering::Relaxed)
    }

    /// Any UPower or BlueZ signal marks the cache stale. Reading happens later,
    /// on the sampling thread, so bursts of signals cost a single read.
    fn listen() {
        let Some(connection) = connect() else { return };
        for rule in [
            "type='signal',sender='org.freedesktop.UPower'",
            "type='signal',sender='org.bluez'",
        ] {
            if connection
                .call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    "AddMatch",
                    &(rule,),
                )
                .is_err()
            {
                return;
            }
        }
        WATCHING.store(true, Ordering::Relaxed);
        for message in zbus::blocking::MessageIterator::from(&connection) {
            let Ok(message) = message else { break };
            if message.message_type() == zbus::message::Type::Signal {
                CHANGED.store(true, Ordering::Relaxed);
            }
        }
        WATCHING.store(false, Ordering::Relaxed);
    }

    fn get<'a, T>(properties: &'a Properties, key: &str) -> Option<T>
    where
        T: TryFrom<&'a zbus::zvariant::Value<'a>>,
        <T as TryFrom<&'a zbus::zvariant::Value<'a>>>::Error: Into<zbus::zvariant::Error>,
    {
        properties.get(key)?.downcast_ref::<T>().ok()
    }

    fn text(properties: &Properties, key: &str) -> String {
        get::<&str>(properties, key).unwrap_or_default().to_owned()
    }

    fn upower(connection: &Connection) -> Vec<UpowerDevice> {
        let Ok(reply) = connection.call_method(
            Some("org.freedesktop.UPower"),
            "/org/freedesktop/UPower",
            Some("org.freedesktop.UPower"),
            "EnumerateDevices",
            &(),
        ) else {
            return vec![];
        };
        let Ok(paths) = reply.body().deserialize::<Vec<OwnedObjectPath>>() else {
            return vec![];
        };
        paths
            .iter()
            .take(64)
            .filter_map(|path| {
                let reply = connection
                    .call_method(
                        Some("org.freedesktop.UPower"),
                        path.as_str(),
                        Some("org.freedesktop.DBus.Properties"),
                        "GetAll",
                        &("org.freedesktop.UPower.Device",),
                    )
                    .ok()?;
                let p: Properties = reply.body().deserialize().ok()?;
                Some(UpowerDevice {
                    native_path: text(&p, "NativePath"),
                    serial: text(&p, "Serial"),
                    model: text(&p, "Model"),
                    kind: get(&p, "Type").unwrap_or(0),
                    power_supply: get(&p, "PowerSupply").unwrap_or(true),
                    present: get(&p, "IsPresent").unwrap_or(true),
                    percentage: get(&p, "Percentage").unwrap_or(f64::NAN),
                    level: get(&p, "BatteryLevel").unwrap_or(0),
                    state: get(&p, "State").unwrap_or(0),
                    voltage: get(&p, "Voltage").unwrap_or(0.0),
                })
            })
            .collect()
    }

    fn bluez(connection: &Connection) -> Vec<BluezDevice> {
        let Ok(reply) = connection.call_method(
            Some("org.bluez"),
            "/",
            Some("org.freedesktop.DBus.ObjectManager"),
            "GetManagedObjects",
            &(),
        ) else {
            return vec![];
        };
        let Ok(objects) = reply
            .body()
            .deserialize::<HashMap<OwnedObjectPath, HashMap<String, Properties>>>()
        else {
            return vec![];
        };
        objects
            .values()
            .filter_map(|interfaces| {
                let device = interfaces.get("org.bluez.Device1")?;
                Some(BluezDevice {
                    address: text(device, "Address"),
                    name: get::<&str>(device, "Alias")
                        .or_else(|| get::<&str>(device, "Name"))
                        .unwrap_or_default()
                        .to_owned(),
                    icon: text(device, "Icon"),
                    class: get(device, "Class").unwrap_or(0),
                    connected: get(device, "Connected").unwrap_or(false),
                    battery: interfaces
                        .get("org.bluez.Battery1")
                        .and_then(|b| get::<u8>(b, "Percentage")),
                })
            })
            .collect()
    }

    pub fn read(connection: &Connection) -> Result<Vec<PeripheralSample>, ()> {
        // Neither service is mandatory: a desktop without Bluetooth still has UPower.
        Ok(super::merge(&upower(connection), &bluez(connection)))
    }
}
