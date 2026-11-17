# System tray monitoring

Computy provides RAM, CPU and GPU indicators enabled by default. In **Preferences → System tray monitoring**, select individual indicators and volumes, or disable monitoring entirely. Saved changes take effect within two seconds.

**Keep running in the background when closed** is enabled by default: closing the window leaves Computy and its indicators running, and the indicator menu or Ctrl+Q quits. When it is disabled, or when no indicator is shown, closing the window quits Computy.

Three styles are available: **icon + percentage** by default, the previous **ring** design, and an **icon filled** according to usage. Colors are blue below 65%, yellow from 65% to 89%, and red from 90%. Unknown values remain gray and never become 0%.

The detailed panel reuses application components: a large ring, percentage, component icon and name, ten processes with measurements, application icons when accessible, and actions with icons. **View all** opens the complete process list in the main window. Escape, the close button or loss of focus dismisses the panel. A hidden panel does not sample telemetry.

On Linux, each indicator has its own persistent D-Bus service and switches between Active and Passive. Separate identities prevent GNOME rediscovery from coalescing CPU/RAM/GPU objects. The backend also checks the watcher’s registrations and recovers missing entries, including resets that retain the same watcher owner. This avoids recreating AppIndicators with the same identifier. The DBusMenu also provides measurements, icons and actions to hosts that request them. Some GNOME versions force this menu on the first click: its first entry, a double-click or a middle-click opens the panel. Hosts forwarding Activate directly, along with Windows/macOS, open the panel on click.

Process icons are resolved locally from PIDs: desktop applications and GTK themes on Linux, native executable and application icons on Windows/macOS. Images are converted to PNG and cached; a neutral symbol replaces missing icons. Executable paths are not included in CLI/MCP snapshots.

Closing the window keeps monitoring active if at least one icon exists. **Quit Computy** in the menu terminates the application. Launching it again restores the existing window without creating another monitor. Disabling the last indicators while the window is hidden brings it back. Measurements stop when monitoring is disabled, except when a measurement view is open or a CLI/MCP client explicitly requests them. Monitoring never starts diagnostics or adds reports to history.

Logging out or restarting the PC also stops Computy. To restore indicators at the next login without manually opening the application, **Show indicators at login** is enabled by default in installed builds; you can disable it in preferences. The OS startup entry uses `--background`: only enabled indicators appear, and the main window stays hidden. A normal launch or an indicator action opens the existing window. Background launches with all indicators disabled exit without opening a window. Startup preserves an explicit opt-out and does not run diagnostics. Legacy `--return-from-diagnostic` entries follow the same background policy. On macOS, background startup uses accessory mode to keep the Dock unobtrusive. After a package upgrade, choose **Quit Computy**, then reopen the application: closing only the window keeps the previous process and its version in memory.

## Wireless device batteries

**Preferences → Wireless device batteries** is disabled by default. Once enabled, it lists connected wireless peripherals that report a battery, along with remembered devices that are currently disconnected. Each device has its own checkbox to show an indicator, and an icon chooser: **Automatic** uses the detected type (mouse, keyboard, headphones, headset, speaker, game controller, phone, tablet, stylus, watch or other device). These indicators use the same **system tray style** as the metrics. When the metric monitor is disabled, the style and **Keep running in the background when closed** are shown in this section instead.

Colors encode remaining charge, not usage: blue above 30%, yellow from 16% to 30%, red at 15% and below. A disconnected device's indicator is hidden and comes back on reconnection; its settings remain saved (16 devices at most). Clicking an indicator shows the device, its charging state and every connected wireless device.

| Platform | Source |
|---|---|
| Linux | UPower devices (Bluetooth, Logitech Unifying/Bolt receivers and other HID batteries) merged with BlueZ `Device1`/`Battery1` by address. |
| Windows | Plug and Play Bluetooth devices exposing the Bluetooth battery property, through PowerShell. Classic and LE devices are supported when the driver publishes this property. |
| macOS | `system_profiler SPBluetoothDataType` battery levels (main, or the lower of left/right earbuds). |

Values are read from what the operating system already holds: Computy never contacts the device, so reading does not affect its battery. On Linux, UPower and BlueZ change signals trigger a new read, with a five-minute fallback (ten seconds if signals are unavailable). Windows and macOS spawn a system tool, so they re-read every two minutes on mains power and every five minutes on battery.

Some firmware only reports a coarse level (Critical, Low, Normal, High, Full). Computy displays that level rather than a percentage, and the tray draws four blocks instead of digits. A percentage from the Bluetooth battery service is used only when it agrees with the level the device reports; some devices publish a stale or rounded value there. When UPower exposes a battery voltage, Computy estimates the charge from a lithium discharge curve and marks it `≈`. Headset batteries over hands-free profiles depend on BlueZ and the audio server exposing them. Hardware addresses are never shown or stored: devices are identified by a truncated hash. Windows and macOS support has not been physically validated yet.

## Meaning of measurements

| Measurement | Source and scope |
|---|---|
| RAM | Used memory / physical memory visible to the operating system, through sysinfo. |
| CPU | Overall load between two samples; the first sample remains unknown. Per-process values are normalized by logical CPU count to represent a share of total capacity. |
| Linux GPU | Highest available utilization among AMD cards exposing `gpu_busy_percent` and NVIDIA GPUs exposed by `nvidia-smi`. Intel and drivers without these counters remain unavailable. |
| Windows GPU | WDDM counters through CIM: process values summed per engine, then the busiest engine. Availability depends on drivers and permissions. |
| macOS GPU | IOAccelerator utilization counters when available. Availability is not guaranteed and depends on model and driver version; frequency and VRAM usage are never converted into GPU load. |
| Disk indicator | Occupied space on the selected volume. The menu also shows read/write rates exposed by sysinfo. This percentage is neither an activity rate nor a health score. |
| Per-process RAM | Resident memory and its share of physical RAM; shared pages may be counted more than once. Protected processes may hide counters. |
| Per-process disk | I/O across all volumes. The percentage is the process's share of observable I/O among readable processes; attribution to a particular volume is unavailable. |
| Per-process GPU | VRAM of **NVIDIA compute processes** when reported. This does not enumerate every graphics process or provide per-process GPU utilization. Other cases remain unknown. |

CPU/RAM/processes/volumes are sampled every two seconds. GPU queries are spaced by at least six seconds and have timeouts; they run outside the UI thread. Unavailable data is shown as `—` or “Unavailable,” never as an invented success or zero. These measurements are not health diagnostics.

On Linux, collection excludes individual threads to avoid repeating their parent process's counters.

Names and PIDs remain data: they are never executed, control characters are removed from menus, and menu names are shortened. The full view retains original names. Neither command-line arguments nor process environments are collected. Monitoring requires no privilege escalation, driver installation, disk writes or network connection.

## Desktop integration

macOS displays indicators in the menu bar; Windows uses the notification area, possibly its hidden-icons menu. Linux requires an AppIndicator/StatusNotifier host; GNOME may need the corresponding extension. Placement and final icon size depend on the desktop. On Linux/macOS, icon + percentage uses a native label when available: the desktop controls its text color, while the icon remains colored by threshold. Windows uses a compact rendering inside its icon slot because it provides no adjacent label. Detailed-panel percentages, rings and fills retain their colors. Menus provide complete text when the notification area's small size limits legibility.

## CLI and MCP

```bash
computy-cli --json monitor
computy-cli --json monitor --watch
computy-cli settings set --tray-style icon_text
computy-cli settings set --tray-style ring
computy-cli settings set --tray-style fill
computy-cli settings set --tray-enabled false
computy-cli settings set --tray-enabled true --tray-components memory,cpu,gpu
computy-cli settings set --tray-disks '/,/home'
computy-cli settings set --tray-disks ''
computy-cli --json peripherals
computy-cli settings set --battery-enabled true --battery-devices '0123456789abcdef,fedcba9876543210:headset'
```

`monitor --watch` emits JSON snapshots until Ctrl+C. `peripherals` lists connected wireless devices with an opaque `id`, `kind`, `percent` or coarse `level`; `--battery-devices` takes `id[:icon]` values from that list and hides the other remembered devices. The read-only MCP tools `computy_monitor` and `computy_peripherals` expose the same data; `computy_monitor` returns the snapshot, with unknown values set to `null`. The first call initializes the CPU counter; wait at least one second before the next. Preferences remain shared with the application.

## References and validation

The native implementation uses a StatusNotifierItem over D-Bus on Linux and the [tray-icon](https://docs.rs/tray-icon/) crate on Windows/macOS, [sysinfo](https://docs.rs/sysinfo/0.37.2/sysinfo/), [kernel-documented AMD counters](https://kernel.org/doc/html/latest/gpu/amdgpu/thermal.html), and optional driver counters. Linux is validated locally; Windows/macOS integration and GPU drivers absent from this machine still require physical testing.
