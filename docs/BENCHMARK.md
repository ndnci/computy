# Resource benchmark: 0.2.0 (Tauri/WebKit) vs 0.3.0 (GPUI)

Computy 0.3.0 replaces the Tauri window, which rendered a React interface in
WebKitGTK, with a native interface drawn by [GPUI](https://www.gpui.rs/). Both
versions were measured on the same machine, with the same script and method.

## Results

Median of 5 runs (3 for the Tauri Processes scenario). PSS is the memory
actually attributable to Computy, with shared libraries divided among the
processes that use them; it is summed over the whole process tree, so WebKit's
helper processes count for the Tauri version. CPU is a percentage of one core.

| Scenario | PSS 0.2.0 | PSS 0.3.0 | Change | CPU 0.2.0 | CPU 0.3.0 | Threads / processes 0.2.0 | 0.3.0 |
|---|---:|---:|---:|---:|---:|---:|---:|
| Background (tray only, no window) | 219 MiB | **53 MiB** | -76 % | 2.06 % | **0.95 %** | 113 / 7 | **30 / 1** |
| Window open on Overview | 277 MiB | **79 MiB** | -72 % | 1.87 % | **0.80 %** | 122 / 7 | **30 / 1** |
| Processes page (live list, 2 s refresh) | 280 MiB | **92 MiB** | -67 % | 1.36 % | **0.76 %** | 123 / 7 | **30 / 1** |

| Scenario | RSS 0.2.0 | RSS 0.3.0 | USS 0.2.0 | USS 0.3.0 |
|---|---:|---:|---:|---:|
| Background (tray only, no window) | 679 MiB | 123 MiB | 147 MiB | 39 MiB |
| Window open on Overview | 751 MiB | 140 MiB | 202 MiB | 70 MiB |
| Processes page (live list, 2 s refresh) | 767 MiB | 152 MiB | 206 MiB | 84 MiB |

| Artifact | 0.2.0 | 0.3.0 |
|---|---:|---:|
| Desktop binary | 15.7 MiB | 32.2 MiB |
| `.deb` package | 8.5 MiB | 10.1 MiB |

The 0.3.0 binary is larger because it statically contains the interface,
fonts, icons and GPU renderer that 0.2.0 loaded from the system's WebKitGTK
(about 130 MiB of WebKitGTK, JavaScriptCore and GTK libraries on this machine, not counted in its package size).

Raw results: [`benchmarks/0.2.0-tauri.json`](benchmarks/0.2.0-tauri.json) and
[`benchmarks/0.3.0-gpui.json`](benchmarks/0.3.0-gpui.json).

## Why it is lighter

- One process instead of seven: no WebKit web, network or sandbox processes.
- Starting in the background creates no window; the tray popover and main
  window exist only while they are open.
- The interface calls the Rust engine in-process instead of serializing JSON
  over IPC.
- Process icons are decoded as 40 px thumbnails and only visible rows of the
  process list are laid out.

## Method

`scripts/benchmark-desktop.py` (standard library only, Linux):

- Each run starts the binary in a private `dbus-run-session` with a temporary
  `COMPUTY_DATA_DIR` and a copy of the same report history, so the user's
  running instance and data are untouched.
- After a 15 s settle period, the whole process tree is
  sampled every 1 s for 60 s from
  `/proc/<pid>/smaps_rollup` and `/proc/<pid>/stat`; memory is the median
  sample, CPU the average over the window.
- Scenarios: `--background` (tray only), window on Overview, and window opened
  on Processes (`COMPUTY_START_PAGE=processes`).

```bash
python3 scripts/benchmark-desktop.py --binary desktop/target/release/computy \
  --label "GPUI 0.3.0" --package release/Computy_0.3.0_amd64.deb \
  --reports .computy/reports --scenario background --scenario window \
  --scenario processes --output docs/benchmarks/0.3.0-gpui.json
```

Machine: Linux 7.0.0-43-generic, 16 logical CPUs, Wayland GNOME
session, GPUs: Intel Corporation CometLake-H GT2 [UHD Graphics] (rev 05) and NVIDIA Corporation GA106M [GeForce RTX 3060 Mobile / Max-Q] (rev a1). GPUI rendered through Vulkan.

## Compatibility

GPUI needs a GPU API: Vulkan on Linux, DirectX 11 on Windows and Metal on
macOS. Integrated graphics from roughly the last decade (Intel since Skylake,
AMD GCN and later, NVIDIA Kepler and later with current drivers) qualify.
Virtual machines and older hardware without a Vulkan driver can use Mesa's
software rasterizer (`mesa-vulkan-drivers`, recommended by the package), at a
higher CPU cost while the window is open. Background monitoring does not draw
anything and is unaffected.
