# Changelog

Notable user-facing changes are recorded here in English, newest first, following
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions use
[Semantic Versioning](https://semver.org/spec/v2.0.0.html); `0.x` releases remain
under active development. Earlier entries were reconstructed from repository
commits and locally built packages, not from previously published store releases.

## [Unreleased]

## [0.2.0] - 2026-10-29

### Added

- Cleanup workspace and native CLI with a mandatory persisted dry run, reviewed
  selection, logical-byte totals, and explicit confirmation before deletion.
- Nested AND/OR/NOT conditions for paths/regex, names, extensions, content MIME,
  size, age and creation/modification dates; included/excluded directories and
  a project `node_modules` mode.
- Opt-in local Docker image/stopped-container/unused-volume inspection and exact
  object removal, without blanket prune commands or automated volume deletion.
- File cleanup automations created from reviewed previews, with fresh scans,
  per-run byte limits, missed-run handling and retained last results.
- Searchable installed-application inventory, launch/reveal actions, manager
  removal previews and confirmed uninstall. Flatpak/Snap can optionally remove
  package-managed data; unsupported data ownership remains explicit.
- Read-only fan RPM/PWM capabilities where the OS exposes them, plus application
  and fan inventory tools for MCP. Unvalidated manual fan controls stay disabled.

### Changed

- Indicators at login are enabled by default for installed builds. Explicit
  opt-outs persist; background launches keep the main window hidden.
- Removed repeated privacy slogans and the normal Rust-engine connection badge.
  Connection errors and loading feedback remain actionable.
- New maintenance pages use the existing themes and 25-language interface.

### Fixed

- White corners around the tray popover by making its native surface transparent.
- Linux menu summary follows the selected system-bar style; the detailed
  popover keeps its percentage ring independently.
- Sidebar scrolling uses a separate compositing layer and a stable scrollbar gutter.


## [0.1.6] - 2026-10-29

### Fixed

- CPU/RAM indicators disappearing after GNOME rediscovers a shared D-Bus service.
  Every indicator now has a persistent independent service identity, and missing
  registrations are recovered even when the watcher keeps the same bus owner.
- Missing spacing between Save preferences and Installation and data.

### Changed

- Login startup now uses `--background`: enabled indicators appear without the
  main window. Manual launches reopen the existing window; duplicate background
  launches do not raise it. A background launch with no enabled indicators exits.
- Startup settings explicitly describe monitoring at login in all 25 languages.
  Startup remains opt-in, and existing legacy startup arguments stay supported.

## [0.1.5] - 2026-10-29

### Added

- An explicit Quit Computy action in the main application and a GTK `app.quit`
  action for GNOME's dock, separate from hiding the window.
- Confirmed uninstall and data-erasure controls in Preferences, including portable
  installation guidance. History, preferences and pending imports can be erased
  separately; user-exported reports are preserved.
- Headless `computy-cli erase-data --yes`, with diagnostic locking and symlink
  checks. Erasure is not exposed through MCP.
- GitHub release builds for Linux x86-64, Windows x86-64, Intel Mac and Apple
  Silicon, including native CLI archives with license notices and per-platform
  SHA-256 manifests.
- Store distribution guidance and this changelog, covering the project from its
  first local release.

### Changed

- Original Computy code is licensed under GPL-3.0-only; the independent preboot
  component retains its GPLv2 license and corresponding source distribution.
- GitHub Actions are enabled again for pushes, pull requests and manual builds.
  Publishing a version-matching GitHub release builds and attaches downloads.

### Fixed

- A startup crash while resolving application icons on systems with valid
  D-Bus-activated desktop entries containing no executable, reproduced on Ubuntu.
- GNOME dock Quit falling back to window-close behavior and leaving monitoring
  running. Exported application actions now provide an explicit quit request.
- Strict Windows/macOS Clippy checks failing on a Linux-only mutable cache-advice
  variable. Obsolete branch builds are cancelled when a newer push arrives.
- Windows preboot-import compilation and portable USB-test fixture paths, while
  preserving the production rejection of noncanonical USB destinations.
- Consistent LF source checkouts on Windows so formatting checks agree across OSes.

## [0.1.4] - 2026-10-29

### Fixed

- GRUB menu generation failing with `grub_probe: parameter not set` on
  Debian/Ubuntu. Device-probe failure no longer emits a partial menu entry.

### Changed

- Project documentation is maintained in English, including README and agent
  instructions. UI localization remains available in 25 languages.
- Upgrade instructions explain why a running process keeps the previous version
  until it is fully quit and restarted.

### Added

- Isolated GRUB packaging checks with real Debian/Ubuntu libraries and syntax
  validation, plus native window-close/background-monitoring regression checks.

## [0.1.3] - 2026-10-29

### Added

- Three indicator styles: icon + percentage by default, ring, and a filling icon,
  with usage-based colors and explicit unknown values.
- A detailed monitoring popover with process icons and navigation actions.

### Fixed

- Indicators disappearing permanently after toggling RAM or global monitoring
  off and back on. Linux retains its StatusNotifier objects between states.
- Privacy-panel styling and sidebar hover spacing.

## [0.1.2] - 2026-10-29

### Added

- Configurable real-time RAM, CPU, GPU and volume indicators, process ranking,
  and a searchable complete process view.
- Diagnostic reruns from history rows and report dialogs, preserving the original
  and reusing its parameters for a new report.
- USB-search progress and result notifications, hardware-refresh notifications,
  and clearer Secure Boot guidance.

### Fixed

- Linux threads being counted as separate processes, and VRAM aggregation for
  processes present on multiple NVIDIA GPUs.

## [0.1.1] - 2026-10-28

### Added

- Integrated GPLv2 preboot memory component derived from Memtest86+ 9.00, with
  corresponding sources, notices and BIOS/UEFI VM validation.
- Guided USB/no-USB workflows, compatibility checks, separate preparation and
  reboot consent, and a constrained Linux PolicyKit boot helper.

### Fixed

- Successful usable RAM checks showing a neutral result face.
- Ambiguous partial-result labels, explanatory layouts, and preboot instructions.

### Changed

- Automatic GitHub Actions were temporarily disabled during local development.

## [0.1.0] - 2026-10-28

### Added

- Rust diagnostic engine, lightweight Tauri desktop application and native CLI.
- Bounded RAM pattern/inversion checks, SMART/NVMe interpretation, temporary-file
  readback verification, CPU known-answer checks and hardware inventory.
- Component selection, progress, cancellation, stored hardware snapshots,
  report export, history and comparisons.
- Read-only-by-default MCP over stdio and authenticated temporary localhost HTTP,
  with optional explicitly requested Cloudflare tunneling.
- 25 interface languages, right-to-left layouts, system/light/dark themes,
  Computy branding and NDNCI attribution.
- Docker builds, Red → Green → Refactor checks, and real UI screenshots in both
  themes, including diagonal presentation images.
