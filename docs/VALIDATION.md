# Local validation

Validated on Linux on October 29, 2026. No remote service or public tunnel was opened for testing.

| Check | Result |
|---|---|
| Rust engine | 63 passing tests: domain, firmware, actual RAM/disk/CPU loops, cancellation, history, preferences, MCP, Windows imports, real telemetry, process ranking and multi-GPU VRAM aggregation |
| Rust analysis | `cargo fmt --check` and Clippy, all targets, `-D warnings` |
| Node integration | 20 passing tests, including the real CLI, passive monitoring, tray settings, brand names across 25 languages, and official MCP SDK clients over stdio/HTTP |
| Playwright interface | Independent selections, progress, history/comparison, two real reruns with identical parameters and new identifiers, independent checkboxes, measurements/processes, USB spinner, notifications and Secure Boot guidance; light/dark screenshots |
| Internationalization | 25 languages visited, LTR/RTL layouts, system theme and explicit choices, three widths; additional screenshots in five languages and both themes |
| Native desktop | Tauri binary, real Rust IPC, three persistent indicators, three RAM toggle cycles and two global cycles, native window close preserving the process and indicators, reopening the same window, Active/Passive states read over D-Bus, real activation and menus, resolved application icon, D-Bus-only launcher without Exec, explicit dock/tray/in-app quit actions, confirmed erasure on disposable data, real measurements/processes, RAM/disk/CPU diagnostics, saved theme; twelve WebKitGTK screenshots, a three-style rendering test and an isolated D-Bus test |
| GRUB package 0.1.4 | Real Debian and Ubuntu libraries in a container, syntax validated by `grub-script-check`, separate/shared `/boot`, custom default preserved, failed probe producing no partial entry, existing configuration unchanged |
| Build | Production frontend, Linux `.deb`, native CLI and Tauri checks in Docker |
| JavaScript dependencies | `npm audit`: no reported vulnerability advisory at validation time |

Automated UI results are recorded in [`capture-results.json`](screenshots/capture-results.json), [`locales-results.json`](screenshots/locales-results.json), [`native-results.json`](screenshots/native-results.json) and [`history-monitor-results.json`](screenshots/history-monitor-results.json). Style scenarios are recorded in [`tray-style-results.json`](screenshots/tray-style-results.json). Validation scripts generate these records without simulated health measurements.

The 0.1.5 icon regression first crashed the native application with a valid D-Bus-only desktop fixture. After nullable executable handling was added, the same fixture passed and the application stayed open on the Ubuntu host. Separate private sessions verified dock, indicator-menu and in-app quit routes. No host data was erased or application uninstalled. `actionlint` validated both restored workflows; remote platform builds are reported by GitHub Actions.

On the Ubuntu host, restarting only Computy after checking the diagnostic lock restored three distinct Active entries (CPU, RAM and GPU) on the actual GNOME watcher. The existing login startup setting was preserved.

Version 0.1.6 adds native checks for GNOME rediscovery, same-owner watcher recovery, separate bus identities, hidden login startup, background duplicate launches, manual reopening, the generated login argument and maintenance spacing. Startup files are created only in the private test container's home directory. The host login setting is never enabled by automated tests.

## Reproduction

```bash
npm ci
npm run build:engine
npm run test:native
npm test
npm run build
npm run test:ui
```

`test:ui` starts its own server on an available local port, creates a temporary data directory, and removes it after taking screenshots. It uses Chrome at `/usr/bin/google-chrome`; `PLAYWRIGHT_CHROMIUM_EXECUTABLE` selects another test Chromium. This browser belongs to development tooling, not the Tauri distribution.

Native Linux validation uses [tauri-driver and WebKitWebDriver](https://v2.tauri.app/develop/tests/webdriver/manual-setup/) with an Xvfb display. A dedicated image is provided:

```bash
docker build --target desktop-qa -t computy-desktop-qa .
# Prepare release/computy with the compiled Linux binary.
docker run --rm --network none --cap-drop ALL \
  --security-opt no-new-privileges:true --shm-size=256m \
  -e LIBGL_ALWAYS_SOFTWARE=1 \
  -e WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 \
  -v "$PWD:/workspace:ro" \
  -v "$PWD/docs/screenshots:/workspace/docs/screenshots:rw" \
  computy-desktop-qa dbus-run-session -- \
  xvfb-run -a -s '-screen 0 1600x1200x24' node scripts/check-desktop.mjs
```

The WebKit sandbox exception applies only to this test container, which has no network, no Linux capabilities and read-only sources. It is not added to the application binary or launch commands. The image's `computy` user has UID 1000 and needs write access to the mounted screenshot directory. The test sends a real `WM_DELETE_WINDOW` event inside its private Xvfb display, verifies that monitoring remains active, and checks the displayed version against `package.json`.

The GRUB generator test uses another isolated image. It replaces probes only in the container's ephemeral filesystem and runs no program against real devices:

```bash
docker build --target boot-qa -t computy-boot-qa .
docker run --rm --user 0 --network none --cap-drop ALL \
  --security-opt no-new-privileges:true \
  -e COMPUTY_PACKAGING_TEST_SANDBOX=1 \
  -v "$PWD:/workspace:ro" -w /workspace \
  computy-boot-qa node scripts/check-grub-package.mjs
```

To also test the host GRUB library, mount that single file read-only at `/reference/grub-mkconfig_lib` and add `-e COMPUTY_HOST_GRUB_LIB=/reference/grub-mkconfig_lib`. `COMPUTY_TEST_GRUB_GENERATOR` selects a script extracted from a `.deb` inside the container for the same scenarios. The 0.1.4 package was extracted and its installed generator passed both libraries. An APT simulation confirmed an upgrade from 0.1.3, with no additional installation.

## Scope

Windows and macOS have adapters and a CI build matrix, but their packages, startup integration and the Windows diagnostic have not been physically executed in this Linux environment. SMBIOS and Windows parsers, including boot result events, are tested with fixtures. GitHub Actions are enabled; remote platform results must be checked on the actual workflow run.

Successful builds and tests do not certify every component's health. Screenshots retain unknown or inaccessible checks. Coverage, firmware, cache and telemetry limitations appear in reports and the [methodology](DIAGNOSTICS.md).

The guided workflow and preboot component are detailed in [BOOT-DIAGNOSTICS.md](BOOT-DIAGNOSTICS.md). Validation does not schedule real GRUB boots or prepare real USB drives on the host.


## 0.2 maintenance validation

The isolated Rust suite covers mandatory cleanup confirmation, one-use/expired
plans, metadata changes, symlink replacement, protected roots, operation locking,
Boolean rules and automation byte limits. CLI integration covers real temporary
file deletion, preserved exclusions, schedule creation/removal, and read-only
application/fan inventory. Real Docker objects, installed applications and fan
controls are never removed or modified by these tests.

The native WebKit suite captures cleanup configuration/preview, applications and
fan capabilities in both themes. It verifies the new default login entry,
independent ring popover and constant sidebar geometry across scrolling.
The screenshot server is an isolated Xvfb session; its lack of a compositor can
render transparent window corners black. GNOME uses its compositor for those
transparent pixels. The installed application uses a transparent native popover
surface instead of an opaque white background.
