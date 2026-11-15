//! System tray indicators (ported in a later step).
use computy_engine::service::AppService;
use gpui::App;
use std::sync::Arc;

pub fn start(_service: Arc<AppService>, _cx: &mut App) {}
pub fn shutdown() {}
pub fn has_icons() -> bool {
    false
}
pub fn preferences_changed(_cx: &mut App) {}
