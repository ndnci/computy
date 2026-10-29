//! Shared login policy; regular launches always open the application.
use crate::preferences::Preferences;
pub const BACKGROUND_ARGUMENT: &str = "--background";
#[derive(Debug, PartialEq, Eq)]
pub enum LaunchMode {
    Window,
    Monitor,
    Exit,
}
pub fn is_background(arguments: &[String]) -> bool {
    arguments
        .iter()
        .any(|arg| [BACKGROUND_ARGUMENT, "--return-from-diagnostic"].contains(&arg.as_str()))
}
pub fn launch_mode(arguments: &[String], preferences: &Preferences) -> LaunchMode {
    if !is_background(arguments) {
        return LaunchMode::Window;
    }
    if preferences.tray_enabled
        && (!preferences.tray_components.is_empty() || !preferences.tray_disks.is_empty())
    {
        LaunchMode::Monitor
    } else {
        LaunchMode::Exit
    }
}
