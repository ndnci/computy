use computy_engine::{
    peripherals::*,
    preferences::{BatteryDevice, Preferences, validate},
    startup::{LaunchMode, launch_mode},
};

fn mouse_upower(level: u32, percentage: f64) -> UpowerDevice {
    UpowerDevice {
        native_path: "hidpp_battery_0".into(),
        serial: "02:00:5e:10:00:01".into(),
        model: "MX Vertical Advanced Ergonomic Mouse".into(),
        kind: 5,
        power_supply: false,
        present: true,
        percentage,
        level,
        state: 2,
        voltage: 0.0,
    }
}

fn mouse_bluez(battery: Option<u8>) -> BluezDevice {
    BluezDevice {
        address: "02:00:5E:10:00:01".into(),
        name: "MX Vertical".into(),
        icon: "input-mouse".into(),
        class: 0,
        connected: true,
        battery,
    }
}

#[test]
fn kinds_are_mapped_from_every_source() {
    assert_eq!(kind_from_icon("input-mouse"), Some("mouse"));
    assert_eq!(kind_from_icon("audio-headphones"), Some("headphones"));
    assert_eq!(kind_from_icon("audio-card"), Some("speaker"));
    assert_eq!(kind_from_icon("computer"), None);
    assert_eq!(kind_from_upower_type(5), Some("mouse"));
    assert_eq!(kind_from_upower_type(17), Some("headset"));
    // Line power, laptop battery and UPS are not peripherals.
    for kind in [1, 2, 3] {
        assert_eq!(kind_from_upower_type(kind), None);
    }
    // Audio/Video headphones (0x240418) and a peripheral pointing device.
    assert_eq!(kind_from_class(0x240418), Some("headphones"));
    assert_eq!(kind_from_class(0x002580), Some("mouse"));
    assert_eq!(kind_from_class(0x002540), Some("keyboard"));
    assert_eq!(kind_from_class(0x000508), Some("gamepad"));
    assert_eq!(kind_from_name("Magic Keyboard"), "keyboard");
    assert_eq!(kind_from_name("AirPods Pro"), "headphones");
    assert_eq!(kind_from_name("Thing"), "other");
    for kind in KINDS {
        assert!(!kind.is_empty());
    }
}

#[test]
fn identifiers_are_stable_and_never_expose_the_address() {
    let id = device_id("02:00:5E:10:00:01");
    assert_eq!(id, device_id("02:00:5E:10:00:01"));
    assert_eq!(id.len(), 16);
    assert!(!id.to_ascii_uppercase().contains("005E"));
    assert_eq!(
        normalize_address("02_00_5e_10_00_01").as_deref(),
        Some("02:00:5E:10:00:01")
    );
    assert_eq!(normalize_address("not an address"), None);
    let samples = merge(&[mouse_upower(1, 64.0)], &[mouse_bluez(None)]);
    let json = serde_json::to_string(&samples).unwrap();
    assert!(!json.to_ascii_uppercase().contains("5E:10"));
    assert!(!json.to_ascii_lowercase().contains("hidpp"));
}

#[test]
fn a_coarse_level_is_never_turned_into_a_percentage() {
    // Firmware reports only "normal"; the Bluetooth battery service says 100 %.
    let samples = merge(&[mouse_upower(6, 55.0)], &[mouse_bluez(Some(100))]);
    assert_eq!(samples.len(), 1);
    let mouse = &samples[0];
    assert_eq!(mouse.percent, None);
    assert_eq!(mouse.level, "normal");
    assert_eq!(mouse.kind, "mouse");
    assert_eq!(mouse.name, "MX Vertical");
    assert_eq!(mouse.charging, Some(false));
    assert_eq!(mouse.gauge(), Some(55.0));
    // A consistent Bluetooth value is precise and replaces the level.
    let samples = merge(&[mouse_upower(6, 55.0)], &[mouse_bluez(Some(70))]);
    assert_eq!(samples[0].percent, Some(70.0));
    assert!(samples[0].level.is_empty());
}

#[test]
fn exact_and_voltage_based_values() {
    let samples = merge(&[mouse_upower(1, 64.0)], &[]);
    assert_eq!(samples[0].percent, Some(64.0));
    assert!(!samples[0].approximate);
    let mut device = mouse_upower(6, 55.0);
    device.voltage = 3.811;
    let samples = merge(&[device], &[]);
    assert_eq!(samples[0].percent, Some(50.0));
    assert!(samples[0].approximate);
    assert_eq!(percent_from_voltage(4.3), Some(100.0));
    assert_eq!(percent_from_voltage(3.4), Some(0.0));
    assert_eq!(percent_from_voltage(0.0), None);
}

#[test]
fn power_sources_and_disconnected_devices_are_excluded() {
    let mut laptop = mouse_upower(1, 80.0);
    laptop.kind = 2;
    laptop.power_supply = true;
    laptop.serial = "BAT0".into();
    let mut asleep = mouse_upower(1, 80.0);
    asleep.present = false;
    let mut headphones = mouse_bluez(Some(40));
    headphones.address = "02:00:5E:10:00:02".into();
    headphones.icon = "audio-headphones".into();
    headphones.connected = false;
    assert!(merge(&[laptop, asleep], &[headphones.clone()]).is_empty());
    headphones.connected = true;
    let samples = merge(&[], &[headphones]);
    assert_eq!(samples[0].kind, "headphones");
    assert_eq!(samples[0].percent, Some(40.0));
    // Unknown percentage without level: nothing to show.
    assert!(merge(&[mouse_upower(0, f64::NAN)], &[]).is_empty());
}

#[test]
fn macos_and_windows_payloads() {
    let profile = r#"{"SPBluetoothDataType":[{"device_connected":[
        {"MX Keys":{"device_address":"AA:BB:CC:DD:EE:01","device_batteryLevelMain":"85%","device_minorType":"Keyboard"}},
        {"AirPods":{"device_address":"AA:BB:CC:DD:EE:02","device_batteryLevelLeft":"70%","device_batteryLevelRight":"60%","device_minorType":"Headphones"}},
        {"Speaker":{"device_address":"AA:BB:CC:DD:EE:03","device_minorType":"Speaker"}}],
        "device_not_connected":[{"Old":{"device_address":"AA:BB:CC:DD:EE:04","device_batteryLevelMain":"10%"}}]}]}"#;
    let samples = parse_macos_profile(profile);
    assert_eq!(samples.len(), 2);
    assert_eq!(
        (samples[0].kind.as_str(), samples[0].percent),
        ("keyboard", Some(85.0))
    );
    assert_eq!(samples[1].percent, Some(60.0));
    let windows = r#"[{"name":"Logitech Mouse","id":"BTHLE\\DEV_02005E100001\\7&1","battery":42,"cod":null,"connected":true},
        {"name":"WH-1000XM4","id":"BTHENUM\\{0000111E}_LOCALMFG&0002\\7&2&0&02005E100002_C00000000","battery":90,"cod":2360344,"connected":null},
        {"name":"Off","id":"BTHENUM\\X\\1&AABBCCDDEEFF_C0","battery":10,"cod":0,"connected":false}]"#;
    let samples = parse_windows_devices(windows);
    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0].kind, "mouse");
    assert_eq!(samples[1].kind, "headphones");
    assert_eq!(samples[1].id, device_id("02:00:5E:10:00:02"));
    let single = r#"{"name":"Pad","id":"BTHENUM\\Y","battery":5,"cod":1288,"connected":true}"#;
    assert_eq!(parse_windows_devices(single)[0].kind, "gamepad");
    assert!(parse_windows_devices("").is_empty());
}

#[test]
fn battery_preferences_are_validated_and_count_as_indicators() {
    let device = |id: &str, icon: &str| BatteryDevice {
        id: id.into(),
        name: "MX Vertical".into(),
        shown: true,
        icon: icon.into(),
    };
    let mut prefs = Preferences {
        battery_enabled: true,
        battery_devices: vec![device("0123456789abcdef", "auto")],
        ..Preferences::default()
    };
    assert!(validate(&prefs).is_ok());
    for invalid in [
        vec![device("0123456789abcdef", "rocket")],
        vec![device("", "auto")],
        vec![device("dd:2c", "auto")],
        vec![device("a", "auto"), device("a", "mouse")],
        (0..17).map(|i| device(&format!("d{i}"), "auto")).collect(),
    ] {
        let mut copy = prefs.clone();
        copy.battery_devices = invalid;
        assert!(validate(&copy).is_err());
    }
    // A full set of devices fits in the preference file.
    let mut full = prefs.clone();
    full.battery_devices = (0..16)
        .map(|i| BatteryDevice {
            id: format!("{i:016x}"),
            name: "x".repeat(64),
            shown: true,
            icon: "headphones".into(),
        })
        .collect();
    assert!(validate(&full).is_ok());
    prefs.tray_enabled = false;
    let args = vec!["computy".to_owned(), "--background".to_owned()];
    assert_eq!(launch_mode(&args, &prefs), LaunchMode::Monitor);
    prefs.battery_devices[0].shown = false;
    assert_eq!(launch_mode(&args, &prefs), LaunchMode::Exit);
    prefs.battery_devices[0].shown = true;
    prefs.battery_enabled = false;
    assert_eq!(launch_mode(&args, &prefs), LaunchMode::Exit);
}
