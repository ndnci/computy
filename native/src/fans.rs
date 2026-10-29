//! Read-only fan capabilities. Firmware protocols are not interchangeable.
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct Fan {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub rpm: Option<u64>,
    pub pwm_percent: Option<f64>,
    pub mode: Option<u64>,
    pub manual_supported: bool,
}
#[derive(Debug, Serialize)]
pub struct FanInventory {
    pub fans: Vec<Fan>,
    pub control_available: bool,
    pub reason: &'static str,
}
pub fn inventory() -> FanInventory {
    #[allow(unused_mut)]
    let mut fans = vec![];
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(devices) = fs::read_dir("/sys/class/hwmon") {
            for device in devices.flatten() {
                let base = device.path();
                let driver = fs::read_to_string(base.join("name"))
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                for i in 1..=16 {
                    let read = |name: String| {
                        fs::read_to_string(base.join(name))
                            .ok()
                            .and_then(|s| s.trim().parse::<u64>().ok())
                    };
                    let rpm = read(format!("fan{i}_input"));
                    if rpm.is_none() {
                        continue;
                    }
                    let name = fs::read_to_string(base.join(format!("fan{i}_label")))
                        .unwrap_or_else(|_| format!("{driver} · {i}"));
                    fans.push(Fan {
                        id: format!("{}:{i}", base.display()),
                        name: name.trim().into(),
                        driver: driver.clone(),
                        rpm,
                        pwm_percent: read(format!("pwm{i}")).map(|n| n as f64 / 255.0 * 100.0),
                        mode: read(format!("pwm{i}_enable")),
                        manual_supported: false,
                    });
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = crate::commands::run(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "@(Get-CimInstance Win32_Fan | Select-Object DeviceID,Name,DesiredSpeed) | ConvertTo-Json -Compress",
            ],
            10,
        ) && let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&output)
        {
            for item in items {
                fans.push(Fan {
                    id: item["DeviceID"].as_str().unwrap_or("").into(),
                    name: item["Name"].as_str().unwrap_or("Fan").into(),
                    driver: "WMI".into(),
                    rpm: None,
                    pwm_percent: None,
                    mode: None,
                    manual_supported: false,
                });
            }
        }
    }
    FanInventory {
        fans,
        control_available: false,
        reason: "vendor_adapter_required",
    }
}
