#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use computy_engine::{
    repository::FileRepository,
    service::{AppService, SystemHardware},
    transport,
};
use serde_json::Value;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
async fn engine_call(
    service: tauri::State<'_, Arc<AppService>>,
    method: String,
    params: Value,
) -> Result<Value, String> {
    if serde_json::to_vec(&params)
        .map_err(|e| e.to_string())?
        .len()
        > 16384
    {
        return Err("Requête trop volumineuse.".into());
    }
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || transport::dispatch(&service, &method, &params))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn export_report(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<AppService>>,
    id: String,
) -> Result<bool, String> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let report = service.repository.get(&id)?;
        let selected = app
            .dialog()
            .file()
            .set_file_name(format!("computy-{id}.json"))
            .add_filter("Rapport Computy JSON", &["json"])
            .blocking_save_file();
        let Some(file) = selected else {
            return Ok(false);
        };
        let path = file
            .into_path()
            .map_err(|_| "Destination locale requise.".to_owned())?;
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn startup_status(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}
#[tauri::command]
fn set_startup(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("Activez le démarrage automatique depuis une version installée de Computy, pas depuis le mode développement.".into());
    }
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|e| e.to_string())
}
#[tauri::command]
async fn open_offline_tool(service: tauri::State<'_, Arc<AppService>>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let service = service.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            computy_engine::offline::prepare(&service)?;
            std::process::Command::new("mdsched.exe")
                .spawn()
                .map_err(|e| format!("Impossible d’ouvrir le diagnostic mémoire Windows : {e}"))?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = service;
        Err("Utilisez le parcours de test autonome indiqué pour votre système.".into())
    }
}
#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let parsed = url::Url::parse(&url).map_err(|_| "Lien invalide.")?;
    let allowed = [
        "www.ndnci.com",
        "ndnci.com",
        "www.memtest.org",
        "memtest.org",
        "www.memtest86.com",
        "memtest86.com",
        "www.smartmontools.org",
        "smartmontools.org",
        "learn.microsoft.com",
        "support.apple.com",
        "kernel.org",
        "www.kernel.org",
        "github.com",
        "v2.tauri.app",
    ];
    if parsed.scheme() != "https"
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || !parsed
            .host_str()
            .is_some_and(|host| allowed.contains(&host))
    {
        return Err("Ce lien externe n’est pas autorisé.".into());
    }
    app.opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|e| e.to_string())
}
fn main() {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("Computy")
                .args(["--return-from-diagnostic"])
                .build(),
        )
        .setup(|app| {
            let root = if let Some(path) = std::env::var_os("COMPUTY_DATA_DIR") {
                std::path::PathBuf::from(path)
            } else if cfg!(debug_assertions) {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.computy")
            } else {
                app.path().app_data_dir()?
            };
            let repository =
                Arc::new(FileRepository::new(root.join("reports")).map_err(std::io::Error::other)?);
            app.manage(AppService::new(
                repository,
                Arc::new(SystemHardware),
                root.join("scratch"),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_call,
            export_report,
            startup_status,
            set_startup,
            open_offline_tool,
            open_external
        ])
        .build(tauri::generate_context!())
        .expect("Impossible de démarrer Computy");
    application.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            app.state::<Arc<AppService>>().shutdown();
        }
    });
}
