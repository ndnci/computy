//! Local application maintenance. No caller-selected paths or executable arguments.
use std::{path::PathBuf, process::Command, sync::Arc};
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

fn installation() -> Result<(&'static str, PathBuf), String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    if executable == std::path::Path::new("/usr/bin/computy") {
        let status = Command::new("/usr/bin/dpkg-query")
            .args(["-W", "-f=${db:Status-Status}", "computy"])
            .output()
            .map_err(|e| e.to_string())?;
        if status.status.success() && status.stdout == b"installed" {
            return Ok(("deb", executable));
        }
    }
    #[cfg(target_os = "windows")]
    if let Some(parent) = executable.parent() {
        let uninstaller = parent.join("uninstall.exe");
        if uninstaller.is_file() {
            return Ok(("nsis", uninstaller));
        }
    }
    #[cfg(target_os = "macos")]
    if let Some(bundle) = executable
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
    {
        return Ok(("app", bundle.to_owned()));
    }
    Ok(("portable", executable))
}

#[tauri::command]
pub fn maintenance_info(window: tauri::WebviewWindow) -> Result<String, String> {
    if window.label() != "main" {
        return Err("Commande non autorisée.".into());
    }
    installation().map(|(kind, _)| kind.into())
}

#[tauri::command]
pub async fn maintenance_action(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<computy_engine::service::AppService>>,
    action: String,
    confirmed: bool,
) -> Result<(), String> {
    if window.label() != "main" || !confirmed {
        return Err("Commande non autorisée.".into());
    }
    if service.active_job().is_some_and(|j| j.running) {
        return Err("Arrêtez le diagnostic avant de continuer.".into());
    }
    if action == "erase" {
        let service = service.inner().clone();
        tauri::async_runtime::spawn_blocking(move || service.erase_data(true))
            .await
            .map_err(|e| e.to_string())??;
        for view in app.webview_windows().values() {
            view.clear_all_browsing_data().map_err(|e| e.to_string())?;
            view.eval("localStorage.clear();location.reload()")
                .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    if action != "uninstall" {
        return Err("Commande non autorisée.".into());
    }
    let (kind, path) = installation()?;
    if kind == "portable" {
        return Err("Supprimez le fichier portable depuis votre gestionnaire de fichiers.".into());
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            let _ = path;
            let status = Command::new("/usr/bin/pkexec")
                .args(["/usr/bin/apt-get", "remove", "--yes", "computy"])
                .status().map_err(|e| e.to_string())?;
            if !status.success() { return Err("Désinstallation annulée ou refusée.".into()); }
        }
        #[cfg(target_os = "windows")]
        {
            Command::new(path).spawn().map_err(|e| e.to_string())?;
        }
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("/usr/bin/osascript")
                .args(["-e", "on run argv\ntell application \"Finder\" to delete POSIX file (item 1 of argv)\nend run", "--"])
                .arg(path).status().map_err(|e| e.to_string())?;
            if !status.success() { return Err("Désinstallation annulée ou refusée.".into()); }
        }
        Ok(())
    }).await.map_err(|e| e.to_string())??;
    if app.autolaunch().is_enabled().unwrap_or(false) {
        let _ = app.autolaunch().disable();
    }
    app.exit(0);
    Ok(())
}
