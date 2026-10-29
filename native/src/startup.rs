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

/// An absent preference opts in; an explicit opt-out is never overwritten.
pub fn desired(root: &std::path::Path) -> Result<bool, String> {
    let path = root.join("startup.state");
    if !path.exists() {
        return Ok(true);
    }
    let m = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if m.file_type().is_symlink() || m.len() > 16 {
        return Err("Invalid login preference".into());
    }
    serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
pub fn save_desired(root: &std::path::Path, enabled: bool) -> Result<(), String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let temp = root.join(format!("startup-{}.tmp", crate::domain::new_id()));
    std::fs::write(&temp, if enabled { "true" } else { "false" }).map_err(|e| e.to_string())?;
    std::fs::rename(temp, root.join("startup.state")).map_err(|e| e.to_string())
}
