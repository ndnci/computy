# Red → Green → Refactor journal

Tests were written and observed failing before the corresponding behavior was implemented. Artificial data is used for parser tests and injected corruption; application screenshots use real engine checks.

| Cycle | Observed red | Green / behavior | Refactor |
|---|---|---|---|
| Domain and inventory contracts | 12 tests failing on unimplemented APIs | Unknown data has no score; blocking minimum; injected corruption detected; SMART interpreted as a bitmask; no invented SMBIOS maximum rate; safe identifiers | Domain separated from probes and persistence |
| Real execution | Four new tests failing for disk and CPU | Synchronized file verified, cleanup preserving a sentinel, cancellation without success; RAM actually traversed; altered history rejected | Verification functions and RAII for files/memory |
| Boot results | Two failing Windows import tests | Provider and age checked; error and interruption events preserved | Import isolated from the in-session test engine |
| Presentation and comparison | Missing presentation module, failing tests | Missing information explicit, anomalies prioritized, protocol/machine warnings | Presentation functions independent of React |
| MCP | Two failing tests before protocol and authentication support | Minimal permissions, denied mutations, required token, expiry, Origin and Host checks | Transport separated from application service |
| System preferences | Failing test: default `light` theme, missing language | `system` theme, persisted language, validated values | Shared CLI/Tauri/UI preferences |
| Partial SMART evidence | Failing test: hardware alert hidden by a transport error | Explicit alerts remain failures despite partial telemetry | Hardware observation takes precedence over transport availability |
| UI workflow | Playwright could not click the native checkbox; previous result visible when starting a new test | Click target corrected, old state cleared at launch, two real diagnostics followed by comparison | Clearer application launch state |
| Settings parity and partial ECC | Two new failing tests: RAM setting ignored by transport; uncorrected ECC hidden by an absent counter | Shared preferences with explicit overrides; observed errors remain blocking | Options merged in the shared adapter |
| Invalid Origin header | Failing HTTP test: non-ASCII Origin reached content negotiation | Explicit `403`, as for forbidden origins | Transport validation before protocol processing |
| Temporary file ownership | Atomic creation contract missing | Filename collision refuses the test and preserves the existing file | RAII guard acquires the file only after successful creation |

## Checks

```bash
cargo test --manifest-path native/Cargo.toml
cargo clippy --manifest-path native/Cargo.toml --all-targets -- -D warnings
npm run build:engine
npm test
npm run build
npm run test:ui
cargo check --manifest-path src-tauri/Cargo.toml
```

Node integration tests launch the real CLI, check measurements, export and compare reports, and connect to MCP through the official SDK over stdio and authenticated HTTP. HTTP tests stay on localhost and open no tunnel.

`scripts/capture-ui.mjs` checks independent selection of three components, RAM-only / RAM + storage / complete modes, progress, persisted reports and comparisons. Every screen is captured in both themes. Validation data must be isolated with `COMPUTY_DATA_DIR`.

Language catalogs are checked for coverage, preserved placeholders, reading direction and result labels. Rendering checks include screenshots in multiple languages and window sizes.

## Update 0.1.1

A failing presentation test (missing functions) preceded separation of results from unavailable information. Linux contracts initially failed on a missing module; a later test failed on missing preparation consent. Validation then covered firmware/GRUB restrictions, reboot consent and USB writes without overwriting. GPL component algorithms are checked by hashes and actually booted in BIOS/UEFI VMs.

## Update 0.1.2

Monitor contracts failed on the missing module and preference fields before implementation. They then checked old preferences, unknown/invalid percentages, two real samples, and top-ten ranking without limiting the full snapshot. Native rendering distinguishes missing data from 0%.

The Playwright scenario performs two actual reruns, compares parameters and identifiers, then verifies preservation of the original, independent checkboxes and clickable rows. USB/Secure Boot fixtures concern only capabilities and transport errors; no USB writes or boot scheduling are performed. The existing 25-language suite detected a form replaced by an old result; state was corrected to distinguish a current launch from a previous report. Screenshots led to contrast fixes for new dark-mode cards.

Real-process checks then detected Linux TIDs exposed by sysinfo as distinct processes. A failing test compares PIDs with their actual `/proc` Tgid; collection now disables tasks to avoid repeating one process's memory and I/O. Another red/green cycle verifies VRAM aggregation for a PID present on multiple NVIDIA GPUs.

## Update 0.1.3

The style contract initially failed on the absent `tray_style` field; migration now selects `icon_text` and rejects unknown styles. Rendering tests cover three styles, 65/90% thresholds and the distinction between unknown values and zero. The Linux protocol is checked on a real private bus with a test watcher: repeated Active/Passive transitions preserve the object, properties can be read over D-Bus, and Activate opens the panel. Native validation adds three RAM cycles and two global cycles, DBusMenu activation, and PNG resolution of the real application's icon. The watcher and test desktop file supply no simulated hardware measurements.

Playwright screenshots verify the three persisted choices, RAM/CPU panels in both themes, links with icons, the shared Panel component for the privacy block, and remaining spacing between hovered sidebar items.

## Update 0.1.4

The menu-generation test reproduced `grub_probe: parameter not set` in red using the real GRUB library: shell `set -u` interrupted normal optional-variable initialization. The generator now retains `set -e` without enabling `nounset`. Device setup is computed before any output so a failed probe cannot leave a partial entry.

The check then passes with real Debian and Ubuntu libraries, the GRUB syntax validator, shared/separate `/boot` paths, and expected rejection cases. A failed probe produces no entry; custom defaults and the existing configuration are preserved. Tests run only in an isolated container without access to host disks. Native window closure is also checked with a real `WM_DELETE_WINDOW` event in Xvfb: the process and three indicators stay active, and the same window can be shown again. The final native test checks the visible application version against package metadata.

## Update 0.1.5

A valid `DBusActivatable=true` desktop fixture without `Exec` caused the existing native build to crash before its WebDriver session started. On the host, GDB stopped in `strlen`; the GIO application list contained an entry with a null executable. The non-optional gio 0.18 binding was replaced by an optional pointer conversion. The unchanged regression fixture then passed, and the corrected native application stayed open on the Ubuntu host.

GNOME falls back to closing application windows when no explicit `app.quit` action is exported. Computy now registers its GTK ID and a quit action, while retaining window-close-to-tray behavior. Separate private native sessions exercise the GTK action used by the dock, the real DBusMenu Quit event with indicators active, and the main application's Quit button.

Maintenance checks cover missing consent, active locks and symlink rejection, preserve unmanaged exports, and erase an actual disposable diagnostic history through CLI and native UI. Uninstall consent is tested without invoking the real uninstaller. Workflow syntax is checked with actionlint, release metadata is checked against the changelog, and the collector is run against actual Linux build artifacts.

## Update 0.1.6

The native regression first reproduced GNOME's service-based rediscovery reducing three exported indicators to one. Per-indicator persistent connections and names avoid that collision. Another scenario forgets registrations without changing the watcher's bus owner, verifying automatic recovery. Native startup checks cover a hidden `--background` launch, a background duplicate that must not raise the window, manual reopening of the same instance, and a real login entry containing `--background`, created only inside the disposable container. Pure Rust policy tests initially failed on the absent startup module and cover normal, legacy, disabled and disk-only configurations. A bounding-box assertion verifies the gap before the maintenance panel.
