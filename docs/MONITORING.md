# System tray monitoring

Computy provides RAM, CPU and GPU indicators enabled by default. In **Preferences → System tray monitoring**, select individual indicators and volumes, or disable monitoring entirely. Saved changes take effect within two seconds.

Three styles are available: **icon + percentage** by default, the previous **ring** design, and an **icon filled** according to usage. Colors are blue below 65%, yellow from 65% to 89%, and red from 90%. Unknown values remain gray and never become 0%.

The detailed panel reuses application components: a large ring, percentage, component icon and name, ten processes with measurements, application icons when accessible, and actions with icons. **View all** opens the complete process list in the main window. Escape, the close button or loss of focus dismisses the panel. A hidden panel does not sample telemetry.

On Linux, the StatusNotifier backend retains the same object and switches between Active and Passive; it registers again when the tray host returns. This avoids recreating AppIndicators with the same identifier. The DBusMenu also provides measurements, icons and actions to hosts that request them. Some GNOME versions force this menu on the first click: its first entry, a double-click or a middle-click opens the panel. Hosts forwarding Activate directly, along with Windows/macOS, open the panel on click.

Process icons are resolved locally from PIDs: desktop applications and GTK themes on Linux, native executable and application icons on Windows/macOS. Images are converted to PNG and cached; a neutral symbol replaces missing icons. Executable paths are not included in CLI/MCP snapshots.

Closing the window keeps monitoring active if at least one icon exists. **Quit Computy** in the menu terminates the application. Launching it again restores the existing window without creating another monitor. Disabling the last indicators while the window is hidden brings it back. Measurements stop when monitoring is disabled, except when a measurement view is open or a CLI/MCP client explicitly requests them. Monitoring never starts diagnostics or adds reports to history.

Logging out or restarting the PC also stops Computy. To restore indicators at the next login without manually opening the application, enable **Open Computy at login** in preferences. After a package upgrade, choose **Quit Computy**, then reopen the application: closing only the window keeps the previous process and its version in memory.

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
```

`monitor --watch` emits JSON snapshots until Ctrl+C. The read-only MCP tool `computy_monitor` exposes the same snapshot, with unknown values set to `null`. The first call initializes the CPU counter; wait at least one second before the next. Preferences remain shared with the application.

## References and validation

The native implementation uses [Tauri System Tray](https://v2.tauri.app/learn/system-tray/), [sysinfo](https://docs.rs/sysinfo/0.37.2/sysinfo/), [kernel-documented AMD counters](https://kernel.org/doc/html/latest/gpu/amdgpu/thermal.html), and optional driver counters. Linux is validated locally; Windows/macOS integration and GPU drivers absent from this machine still require physical testing.
