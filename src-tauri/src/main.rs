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
#[cfg(target_os = "linux")]
mod linux_indicator;
#[cfg(target_os = "linux")]
mod linux_menu;
mod monitor_tray;
mod process_icons;
mod tray_icon;

#[tauri::command]
async fn engine_call(
    window: tauri::WebviewWindow,
    service: tauri::State<'_, Arc<AppService>>,
    method: String,
    params: Value,
) -> Result<Value, String> {
    if window.label() == "monitor-popover" && !["monitor", "preferences"].contains(&method.as_str())
    {
        return Err("Commande non autorisée.".into());
    }
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
fn monitor_panel_target(app: tauri::AppHandle) -> monitor_tray::PanelTarget {
    monitor_tray::panel_target(&app)
}
#[tauri::command]
fn monitor_panel_action(app: tauri::AppHandle, destination: String) -> Result<(), String> {
    monitor_tray::panel_action(&app, &destination)
}
#[tauri::command]
fn monitor_tray_status(app: tauri::AppHandle) -> Value {
    app.state::<monitor_tray::TrayState>().status()
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
    #[cfg(target_os = "linux")]
    {
        let service = service.inner().clone();
        tauri::async_runtime::spawn_blocking(move || computy_engine::offline::prepare(&service))
            .await
            .map_err(|error| error.to_string())?
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = service;
        Err("Utilisez le parcours de test autonome indiqué pour votre système.".into())
    }
}
#[tauri::command]
async fn boot_action(action: String) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        tauri::async_runtime::spawn_blocking(move || match action.as_str() {
            "verify" | "cancel" => computy_engine::linux_boot::request(&action),
            "reboot" => computy_engine::linux_boot::reboot().map(|_| serde_json::json!(true)),
            _ => Err("Commande non autorisée.".into()),
        })
        .await
        .map_err(|error| error.to_string())?
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = action;
        Err("Ce parcours automatique est disponible sous Linux.".into())
    }
}
#[tauri::command]
async fn usb_targets() -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        tauri::async_runtime::spawn_blocking(|| {
            computy_engine::usb_boot::targets().map(|targets| serde_json::json!(targets))
        })
        .await
        .map_err(|error| error.to_string())?
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(serde_json::json!([]))
    }
}
#[tauri::command]
async fn prepare_usb(id: String) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        tauri::async_runtime::spawn_blocking(move || {
            computy_engine::usb_boot::prepare(&id).map(|target| serde_json::json!(target))
        })
        .await
        .map_err(|error| error.to_string())?
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = id;
        Err("Préparation USB automatique disponible sous Linux.".into())
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
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("Computy")
                .args(["--return-from-diagnostic"])
                .build(),
        )
        .on_window_event(|window, event| {
            if window.label() == "monitor-popover" {
                if matches!(event, tauri::WindowEvent::Focused(false)) {
                    let _ = window.hide();
                }
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.state::<monitor_tray::TrayState>().has_icons() && window.hide().is_ok() {
                    api.prevent_close();
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
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
            let service =
                AppService::new(repository, Arc::new(SystemHardware), root.join("scratch"));
            app.manage(service.clone());
            app.manage(monitor_tray::TrayState::default());
            monitor_tray::setup_panel(app.handle())?;
            monitor_tray::start(app.handle().clone(), service);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_call,
            export_report,
            startup_status,
            set_startup,
            open_offline_tool,
            boot_action,
            usb_targets,
            prepare_usb,
            open_external,
            monitor_panel_target,
            monitor_panel_action,
            monitor_tray_status,
            process_icons::process_icons
        ])
        .build(tauri::generate_context!())
        .expect("Impossible de démarrer Computy");
    application.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            app.state::<monitor_tray::TrayState>()
                .stopped
                .store(true, std::sync::atomic::Ordering::Relaxed);
            app.state::<Arc<AppService>>().shutdown();
        }
    });
}
