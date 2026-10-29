<p align="center"><img src="public/computy-logo.png" width="120" alt="Computy logo" /></p>

# Computy

**Understand your hardware.** A **Rust + Tauri** desktop application, **native CLI** and **MCP server** for inspecting, testing and tracking your computer.

Created by **[NDNCI](https://www.ndnci.com)**. No account or usage tracking. Your history stays on your machine.

![Computy in light and dark mode](docs/screenshots/presentation-overview-with-history.png)

## Features

- **Choose checks with independent checkboxes**: RAM, storage, RAM + storage, CPU, or a complete assessment.
- Read real hardware information: visible memory, firmware-reported modules and maximum capacity, configured transfer rates, form factor and replacement hints, CPU, volumes and available sensors.
- Test RAM using multiple patterns and inversions, inspect available **SMART/NVMe** data, verify a synchronized temporary file, and check CPU calculations against known answers.
- Follow actual progress, cancel diagnostics, and preserve partial results.
- Keep a hardware snapshot for each report, rename it, export JSON, and compare **2–4 reports**.
- **Rerun a diagnostic** from its history row or report. The same settings create a new diagnostic while preserving the original. The whole row opens the report; checkboxes and actions work independently. Rerunning a Windows preboot report opens the wizard and requires fresh confirmation.
- Monitor RAM/CPU/GPU through configurable **system tray indicators**, enabled by default, with optional volume indicators. Open the **top 10 processes**, then **View all** for the complete searchable, sortable list. Missing measurements remain explicit. See [monitoring behavior and limits](docs/MONITORING.md).
- Follow the **system theme by default**, or choose **light/dark**. Select from **25 languages**, with localized formatting and appropriate text direction.
- Use the same checks, reports, preferences and comparisons from the terminal or an MCP agent.

**An error-free result means no error was observed under the test conditions. It does not certify fault-free hardware.** Computy shows missing information and the scope of each check. Its score out of 10 summarizes usable checks; it is not a probability of good health.

## Getting started

### Try the existing Linux development build

When local binaries have already been built in `release/`, run these commands from the repository root:

```bash
# Open the native application without compiling or running Docker
./release/computy

# Or run a quick RAM + storage diagnostic in the terminal
./release/computy-cli diagnose --profile quick --components memory,storage
```

In the application, open **Diagnostics**, select components, choose the quick profile, and start the diagnostic. The report is saved automatically in history.

On Ubuntu/Debian, installing the package also adds Computy to the applications menu:

```bash
sudo apt install ./release/Computy_0.1.4_amd64.deb
```

Runtime libraries are present on the development PC used for validation. Files in `release/` are local build artifacts excluded from Git; after cloning the repository, build them using the commands below.

### Desktop application

Requirements: Node.js 22+, stable Rust (1.95+), and the [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/). Linux uses WebKitGTK, macOS uses WKWebView, and Windows uses WebView2. **Computy does not bundle Chromium.**

```bash
npm ci
npm run desktop
```

Build a native package for the current platform:

```bash
npm run package
```

Packages are written to `src-tauri/target/release/bundle/`. Build Windows and macOS packages on their respective operating systems. The three-platform workflow is preserved in `.github/workflows/checks.yml.disabled`; automatic GitHub Actions are currently disabled. Releases are not signed or notarized yet.

The locally validated Linux package is [`release/Computy_0.1.4_amd64.deb`](release/Computy_0.1.4_amd64.deb), alongside the desktop and CLI binaries in `release/`. This build directory is excluded from Git. The package is approximately **7.6 MiB**.

### Develop and validate with Docker

Docker provides Rust and the Linux Tauri dependencies without installing a compiler on the host.

```bash
# Engine, frontend and Tauri integration checks
docker compose run --build --rm verify

# Development preview at http://127.0.0.1:1420
docker compose up --build dev

# Linux .deb package, copied into ./release
docker compose run --build --rm desktop-build
```

**A container reports its own runtime environment.** The Docker preview is a development tool and does not provide a complete hardware inventory of the host. Run the native binary on the computer you want to diagnose. No `--privileged` option is required.

### Headless CLI

```bash
cargo build --release --manifest-path native/Cargo.toml
./native/target/release/computy-cli --help

# Full inventory as JSON
./native/target/release/computy-cli --json inventory

# RAM only
./native/target/release/computy-cli diagnose --components memory --memory-mib 256 --passes 2

# RAM + storage
./native/target/release/computy-cli diagnose --components memory,storage --label "Before replacement"

# Complete assessment, quick profile
./native/target/release/computy-cli --json diagnose --profile quick --label "After replacement"

./native/target/release/computy-cli history
./native/target/release/computy-cli --json compare BEFORE_ID AFTER_ID
./native/target/release/computy-cli export ID --output report.json
./native/target/release/computy-cli rename ID "RAM after replacement"
./native/target/release/computy-cli settings set --theme system --language en

# Open the installed application at login (optional)
./native/target/release/computy-cli startup enable --desktop-path /absolute/path/to/Computy
```

`Ctrl+C` cancels a diagnostic cleanly. Progress goes to `stderr`; JSON results go to `stdout`. Exit codes: `0` completed without a blocking anomaly, `1` command error, `2` anomaly detected, `3` no usable checks, `130` interrupted. Code `0` may still accompany partial coverage: inspect `summary.status`, `checked/total`, and individual checks.

Use `--data-dir PATH` to isolate history. Installed CLI and Tauri builds use the current user's `com.ndnci.computy` application data directory. Development mode uses `.computy`; pass `--data-dir .computy` to the CLI to share that history. `COMPUTY_DATA_DIR` also configures the development engine's data directory.

## AI agents and MCP

Both transports use the same Rust engine. **Read-only by default**; agents receive no arbitrary shell, raw device writes, or reboot commands.

### Local stdio

```json
{
  "mcpServers": {
    "computy": {
      "command": "/absolute/path/to/computy-cli",
      "args": ["mcp"]
    }
  }
}
```

Add `--allow-diagnostics` to allow starting and cancelling bounded checks. Separately add `--allow-history-write` for renaming, deletion, preferences, and Windows result imports. Unauthorized tools are omitted from discovery and rejected on direct invocation.

### Temporary HTTP access and an optional tunnel

```bash
computy-cli mcp --http --port 8765 --ttl 900 --token-file /private/path/computy.token

# Explicitly publish a temporary endpoint if cloudflared is already installed
computy-cli mcp --http --ttl 900 --cloudflare-tunnel
```

The server listens exclusively on **127.0.0.1**, generates a 256-bit token, writes it to a private file (`0600` on Unix), validates each `Authorization: Bearer …` header, and expires after the selected duration (60–3600 seconds). Tokens never appear in URLs. Browser origins are denied by default; only exact HTTPS origins can be allowed with `--allow-origin`.

The tunnel forwards only the MCP port and sets the expected local Host header. It stops with the server on normal shutdown or expiration. **Opening Computy never opens a tunnel automatically.** The remote client uses the HTTPS URL printed by `cloudflared`, followed by `/mcp`, and sends the token in its headers.

This mode targets MCP clients that support an explicit Bearer token. It does not provide an OAuth server or universal compatibility with clients requiring OAuth. [Cloudflare Quick Tunnels](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/) are intended for development and temporary access; persistent deployments need a managed configuration with access controls.

See [MCP security](docs/SECURITY.md) for limits, permissions and revocation.

## Preboot memory testing

**Before startup** is separate from in-session profiles. The wizard offers paths **with and without a USB drive**, checks compatibility, and asks for consent before preparation, USB writes and rebooting. The integrated engine and its GPL sources ship with Computy.

On supported Linux x86-64 systems, Computy can select a GRUB entry for one boot, or copy the engine onto a FAT32 USB drive without formatting it or overwriting an existing bootloader. Enabled Secure Boot blocks the unapproved binary; Computy does not disable that protection. Standalone Linux results are not imported automatically and receive no assumed score.

Windows uses its system memory diagnostic and imports a recent result event after returning. Reopening Computy at login is enabled by default in that workflow. The Apple workflow remains model-dependent.

See [behavior, limits and confirmations](docs/BOOT-DIAGNOSTICS.md).

### Update an Ubuntu/Debian installation

Choose **Quit Computy** from an indicator's menu, then run these commands from the repository root:

```bash
sudo apt install ./release/Computy_0.1.4_amd64.deb
/usr/bin/computy
```

The package name remains `computy`: APT upgrades the existing installation to 0.1.4. History and preferences stay in the same user data directory. The header's **Refresh hardware info** action rereads hardware information and shows a notification; it does not update the application. A portable copy launched with `./release/computy` is separate from the APT installation: use the applications menu or `/usr/bin/computy` to test the installed package.

**Closing the window keeps Computy running in the background while its indicators are present.** APT replaces the installed file, but an already running process keeps executing the previous version. Single-instance handling also sends subsequent launches to that process. Choose **Quit Computy**, then reopen the application to load the updated version without restarting your PC. Use `dpkg-query -W computy` to check the installed package version separately.

## Screenshots

### Live monitoring and processes

Choose from three styles: **icon + percentage** by default, **ring**, or an **icon that fills up**. Blue below 65%, yellow from 65% to 89%, red from 90%. Indicators remain registered while disabled so they can be reactivated. A detailed panel shows the component, top processes with available application icons, and actions. On GNOME versions that force a native menu on the first click, its first entry or a double-click opens the panel.

| Light | Dark |
|---|---|
| ![Indicator styles in light mode](docs/screenshots/tray-styles-light.png) | ![Indicator styles in dark mode](docs/screenshots/tray-styles-dark.png) |
| ![RAM panel in light mode](docs/screenshots/tray-panel-memory-light.png) | ![RAM panel in dark mode](docs/screenshots/tray-panel-memory-dark.png) |

![Tauri panel with the real Computy application icon](docs/screenshots/native-linux-tray-panel.png)

| Light | Dark |
|---|---|
| ![Monitor preferences in light mode](docs/screenshots/monitor-settings-light.png) | ![Monitor preferences in dark mode](docs/screenshots/monitor-settings-dark.png) |
| ![Live processes in light mode](docs/screenshots/processes-live-light.png) | ![Live processes in dark mode](docs/screenshots/processes-live-dark.png) |
| ![History rerun actions in light mode](docs/screenshots/history-rerun-actions-light.png) | ![History rerun actions in dark mode](docs/screenshots/history-rerun-actions-dark.png) |

Native indicators stay in the system tray while the window is hidden. Their placement depends on the desktop; these screenshots show Computy's settings and measurements.

Screenshots show information exposed by the validation machine. Reports come from checks that actually ran, without simulating hardware replacements. The application language can be changed independently of this English documentation; existing screenshots include French and other supported languages.

### Overview

| Light | Dark |
|---|---|
| ![Overview in light mode](docs/screenshots/overview-with-history-light.png) | ![Overview in dark mode](docs/screenshots/overview-with-history-dark.png) |

### Choose a diagnostic

![Diagnostic overview split between light and dark mode](docs/screenshots/presentation-diagnostics.png)

| Light | Dark |
|---|---|
| ![Complete diagnostic in light mode](docs/screenshots/diagnostics-light.png) | ![Complete diagnostic in dark mode](docs/screenshots/diagnostics-dark.png) |
| ![RAM only in light mode](docs/screenshots/diagnostics-memory-only-light.png) | ![RAM only in dark mode](docs/screenshots/diagnostics-memory-only-dark.png) |
| ![RAM and storage in light mode](docs/screenshots/diagnostics-memory-storage-light.png) | ![RAM and storage in dark mode](docs/screenshots/diagnostics-memory-storage-dark.png) |

### Progress and results

| Light | Dark |
|---|---|
| ![Diagnostic progress in light mode](docs/screenshots/diagnostic-running-light.png) | ![Diagnostic progress in dark mode](docs/screenshots/diagnostic-running-dark.png) |
| ![Diagnostic results in light mode](docs/screenshots/diagnostic-result-light.png) | ![Diagnostic results in dark mode](docs/screenshots/diagnostic-result-dark.png) |

### History and comparisons

![Comparison split between light and dark mode](docs/screenshots/presentation-comparison.png)

| Light | Dark |
|---|---|
| ![History in light mode](docs/screenshots/history-light.png) | ![History in dark mode](docs/screenshots/history-dark.png) |
| ![Comparison in light mode](docs/screenshots/comparison-light.png) | ![Comparison in dark mode](docs/screenshots/comparison-dark.png) |

### Hardware and preferences

| Light | Dark |
|---|---|
| ![Memory in light mode](docs/screenshots/memory-light.png) | ![Memory in dark mode](docs/screenshots/memory-dark.png) |
| ![Storage in light mode](docs/screenshots/storage-light.png) | ![Storage in dark mode](docs/screenshots/storage-dark.png) |
| ![CPU in light mode](docs/screenshots/cpu-light.png) | ![CPU in dark mode](docs/screenshots/cpu-dark.png) |
| ![Preferences in light mode](docs/screenshots/settings-light.png) | ![Preferences in dark mode](docs/screenshots/settings-dark.png) |
| ![About in light mode](docs/screenshots/about-light.png) | ![About in dark mode](docs/screenshots/about-dark.png) |

<details>
<summary>Native Tauri Linux window: light and dark</summary>

These additional screenshots come from the final Tauri binary running WebKitGTK in a validation container, without a development web server. RAM, temporary file and CPU checks actually ran, and their measurements were read back from the reports.

| Light | Dark |
|---|---|
| ![Native Linux overview in light mode](docs/screenshots/native-linux-overview-light.png) | ![Native Linux overview in dark mode](docs/screenshots/native-linux-overview-dark.png) |
| ![Native Linux diagnostics in light mode](docs/screenshots/native-linux-diagnostics-light.png) | ![Native Linux diagnostics in dark mode](docs/screenshots/native-linux-diagnostics-dark.png) |
| ![Native Linux result in light mode](docs/screenshots/native-linux-result-light.png) | ![Native Linux result in dark mode](docs/screenshots/native-linux-result-dark.png) |

</details>

<details>
<summary>More languages: English, Arabic, Japanese, Chinese and Hindi</summary>

| Light | Dark |
|---|---|
| ![English in light mode](docs/screenshots/overview-en-light.png) | ![English in dark mode](docs/screenshots/overview-en-dark.png) |
| ![Arabic, right to left, in light mode](docs/screenshots/overview-ar-light.png) | ![Arabic, right to left, in dark mode](docs/screenshots/overview-ar-dark.png) |
| ![Japanese in light mode](docs/screenshots/overview-ja-light.png) | ![Japanese in dark mode](docs/screenshots/overview-ja-dark.png) |
| ![Chinese in light mode](docs/screenshots/overview-zh-CN-light.png) | ![Chinese in dark mode](docs/screenshots/overview-zh-CN-dark.png) |
| ![Hindi in light mode](docs/screenshots/overview-hi-light.png) | ![Hindi in dark mode](docs/screenshots/overview-hi-dark.png) |

</details>

### Understanding a RAM result

The same report may contain successful tests and unavailable counters. The interface distinguishes them and shows the memory actually tested and the number of passes.

| Light | Dark |
|---|---|
| ![Explained RAM result in light mode](docs/screenshots/ram-report-explained-light.png) | ![Explained RAM result in dark mode](docs/screenshots/ram-report-explained-dark.png) |

### Guided preboot workflow

| Light | Dark |
|---|---|
| ![Without a USB drive, light mode](docs/screenshots/boot-guide-internal-light.png) | ![Without a USB drive, dark mode](docs/screenshots/boot-guide-internal-dark.png) |
| ![With a USB drive, light mode](docs/screenshots/boot-guide-usb-light.png) | ![With a USB drive, dark mode](docs/screenshots/boot-guide-usb-dark.png) |

The integrated preboot component in a UEFI VM (its display environment depends on firmware):

![Integrated memory test running in QEMU UEFI](docs/screenshots/boot-engine-uefi.png)

## Architecture and verification

```text
native/             Domain, diagnostics, OS inventory, history, CLI, MCP
src-tauri/          Rust desktop host: validated commands, windows and OS integration
src/                React/TypeScript interface and bundled i18n catalogs
scripts/            Native preview, offline localization, screenshots and presentations
tests/              Presentation contracts and CLI/MCP integration with the official SDK
docs/               Methodology, security, RGR journal and screenshots
Dockerfile          Build, validation and development preview
```

The domain and diagnostics are independent of the interface. Inventory and history adapters use dedicated interfaces. CLI, Tauri and MCP share the same application service. Reports are written atomically with a SHA-256 digest to detect accidental changes; this digest does not authenticate files against an attacker with the user's permissions.

```bash
npm run build:engine
npm test
npm run test:native
npm run build
npm run check:desktop

# Complete workflows, isolated data and automatic screenshots
npm run test:ui
```

**Red → Green → Refactor** cycles are recorded in the [RGR journal](docs/RGR.md). The [methodology](docs/DIAGNOSTICS.md) explains algorithms, scoring and primary sources. The [localization documentation](docs/I18N.md) describes offline generation and editorial overrides.

The [validation report](docs/VALIDATION.md) lists observed results, native window checks and platforms that still need testing on physical machines.
