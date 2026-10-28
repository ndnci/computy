use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub memory_mib: usize,
    pub passes: usize,
    pub disk_mib: usize,
    pub cpu_seconds: u64,
    pub theme: String,
    pub language: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            memory_mib: 256,
            passes: 2,
            disk_mib: 64,
            cpu_seconds: 5,
            theme: "system".into(),
            language: "system".into(),
        }
    }
}
pub fn load(root: &Path) -> Result<Preferences, String> {
    let path = root.join("preferences.state");
    if !path.exists() {
        return Ok(Preferences::default());
    }
    let metadata = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if metadata.len() > 8192 || metadata.file_type().is_symlink() {
        return Err("Préférences invalides.".into());
    }
    let preferences: Preferences =
        serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    validate(&preferences)?;
    Ok(preferences)
}
pub fn validate(preferences: &Preferences) -> Result<(), String> {
    crate::domain::validate_options(&crate::domain::RunOptions {
        memory_mib: preferences.memory_mib,
        passes: preferences.passes,
        disk_mib: preferences.disk_mib,
        cpu_seconds: preferences.cpu_seconds,
        ..Default::default()
    })?;
    if !["system", "light", "dark"].contains(&preferences.theme.as_str()) {
        return Err("Thème invalide.".into());
    }
    if ![
        "system", "en", "fr", "es", "pt-BR", "de", "it", "nl", "pl", "ru", "uk", "tr", "ar", "hi",
        "bn", "ur", "id", "vi", "th", "ko", "ja", "zh-CN", "zh-TW", "fa", "he", "sw",
    ]
    .contains(&preferences.language.as_str())
    {
        return Err("Langue non prise en charge.".into());
    }
    Ok(())
}
pub fn save(root: &Path, preferences: &Preferences) -> Result<(), String> {
    validate(preferences)?;
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    use std::io::Write;
    let path = root.join("preferences.state");
    let temporary = root.join(format!("preferences-{}.tmp", crate::domain::new_id()));
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(preferences).map_err(|e| e.to_string())?)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(temporary, path).map_err(|e| e.to_string())
}
pub fn default_data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("COMPUTY_DATA_DIR") {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("com.ndnci.computy")
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Library/Application Support/com.ndnci.computy")
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir)
                    .join(".local/share")
            })
            .join("com.ndnci.computy")
    }
}
