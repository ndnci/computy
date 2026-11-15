# Local validation

Validated on Linux on October 29, 2026 (0.2.x, WebKit interface). The 0.3.0 native interface is covered by the visual regression and benchmark sections below; the table describes the earlier validation. No remote service or public tunnel was opened for testing.

| Check | Result |
|---|---|
| Rust engine | 63 passing tests: domain, firmware, actual RAM/disk/CPU loops, cancellation, history, preferences, MCP, Windows imports, real telemetry, process ranking and multi-GPU VRAM aggregation |
| Rust analysis | `cargo fmt --check` and Clippy, all targets, `-D warnings` |
| Node integration | 20 passing tests, including the real CLI, passive monitoring, tray settings, brand names across 25 languages, and official MCP SDK clients over stdio/HTTP |
| Playwright interface | Independent selections, progress, history/comparison, two real reruns with identical parameters and new identifiers, independent checkboxes, measurements/processes, USB spinner, notifications and Secure Boot guidance; light/dark screenshots |
| Internationalization | 25 languages visited, LTR/RTL layouts, system theme and explicit choices, three widths; additional screenshots in five languages and both themes |
| Native desktop | Tauri binary, real Rust IPC, three persistent indicators, three RAM toggle cycles and two global cycles, native window close preserving the process and indicators, reopening the same window, Active/Passive states read over D-Bus, real activation and menus, resolved application icon, D-Bus-only launcher without Exec, explicit dock/tray/in-app quit actions, confirmed erasure on disposable data, real measurements/processes, RAM/disk/CPU diagnostics, saved theme; twelve WebKitGTK screenshots, a three-style rendering test and an isolated D-Bus test |
| GRUB package 0.1.4 | Real Debian and Ubuntu libraries in a container, syntax validated by `grub-script-check`, separate/shared `/boot`, custom default preserved, failed probe producing no partial entry, existing configuration unchanged |
| Build | Linux `.deb`, native CLI and desktop checks in Docker |
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
npm run test:desktop
npm run package        # release/Computy_<version>_amd64.deb
```

The desktop crate is built in the `gpui` Docker stage (`scripts/gpui-docker.sh`), which provides Rust, the GPUI system libraries, Mesa's software Vulkan driver, a headless sway compositor and `grim`. No compiler is needed on the host.

## Visual regression (0.3.0)

The move from the WebKit interface to the native GPUI interface was checked pixel by pixel against screenshots of the previous interface:

1. `scripts/visual/record-fixture.mjs` records every engine response once into a frozen fixture (`tests/visual/fixture.json`), so live measurements, processes and clocks cannot change between captures.
2. Before the migration, `scripts/visual/capture-react.mjs` captured the React interface with Playwright from that fixture: every page in light and dark, diagnostic states, report and comparison dialogs, cleanup preview, popovers, five extra locales (en, ar, ja, zh-CN, hi) and four window widths (1000, 1200, 1440, 1920). Each long page is captured in scroll steps. This script only runs on commits that still contain the React sources; its output is the reference set in `tests/visual/baseline/`.
3. `npm run visual:capture` replays the same scenarios (`scripts/visual/scenarios.mjs`) with the native binary's `--capture` mode inside a headless sway session and captures them with `grim`.
4. `npm run visual:compare` writes per-pixel heatmaps to `tests/visual/diff/` and a side-by-side report to `tests/visual/report.html`, sorted by the share of pixels differing by more than 32 levels on a channel.

On November 16, 2026, 176 captures were compared: the median share of differing pixels is 4.1 %, the largest 8.6 % (boot-mode diagnostics, Arabic overview and the 1000 px layout), and 121 captures differ by less than 5 %. Text rasterization differs between WebKit and GPUI (antialiasing, hinting), so identical pixels are not expected; heatmaps were reviewed for layout, size, spacing and color differences. Known accepted differences: dialog backdrops are not blurred, right-to-left languages keep a left-to-right layout, and narrow layouts differ slightly in line wrapping.

## Resource benchmark

`npm run benchmark -- --binary <path> --label <name>` starts the binary in a private D-Bus session with a temporary data directory, then measures the whole process tree (PSS, RSS, USS, CPU, threads). Method and results are in [BENCHMARK.md](BENCHMARK.md).

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
