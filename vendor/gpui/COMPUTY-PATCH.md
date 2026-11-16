# Computy patch

This is `gpui` 0.2.2 from crates.io, retained under its original Apache-2.0 license. Examples and integration tests were left out; the library sources, resources and documentation are unchanged except as described below.

On Linux, upstream stops the event loop when the last window is dropped (`src/platform/linux/x11/client.rs` and `src/platform/linux/wayland/client.rs`). Computy keeps running in the system tray after its main window or the tray popover closes, so both checks are removed. The application quits explicitly through `cx.quit()` (tray menu, Ctrl+Q, or closing the window when background running is disabled). macOS and Windows already keep the application alive without windows.

Remove the Cargo patch and this directory once upstream GPUI offers a way to keep the application alive without windows on Linux. Original source: <https://crates.io/crates/gpui/0.2.2>.
