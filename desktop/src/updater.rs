//! Update checks and installation. The engine queries GitHub releases and
//! verifies installers; this module schedules daily checks, keeps the state
//! shown by the interface, installs the verified package and relaunches.
use crate::model::UpdateStatus;
use computy_engine::service::AppService;
use gpui::{App, AsyncApp, Global};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Idle,
    Checking,
    Downloading,
    Installing,
}

/// Shared update state, read by the preferences page and the sidebar.
pub struct Updates {
    /// `None` in capture mode, where actions are inert.
    service: Option<Arc<AppService>>,
    pub status: UpdateStatus,
    pub phase: Phase,
    /// Last download or installation failure (engine text, translated by `tx`).
    pub error: Option<String>,
    /// The installer was opened for a manual step (macOS disk image).
    pub manual: bool,
}

impl Global for Updates {}

/// An update is ready to install (sidebar badge).
pub fn available(cx: &App) -> bool {
    cx.try_global::<Updates>()
        .is_some_and(|u| u.status.available)
}

fn status(service: &AppService, check: bool) -> UpdateStatus {
    let root = service.data_root();
    let status = if check {
        computy_engine::updates::check(&root)
    } else {
        computy_engine::updates::load(&root)
    };
    serde_json::to_value(status)
        .ok()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

/// Capture mode: show a fixture state without any network access.
pub fn show(status: UpdateStatus, cx: &mut App) {
    cx.set_global(Updates {
        service: None,
        status,
        phase: Phase::Idle,
        error: None,
        manual: false,
    });
}

/// Load the saved state and check once a day while automatic checks are on.
pub fn start(service: Arc<AppService>, cx: &mut App) {
    cx.set_global(Updates {
        status: status(&service, false),
        service: Some(service.clone()),
        phase: Phase::Idle,
        error: None,
        manual: false,
    });
    remove_previous_executable();
    cx.spawn(async move |cx: &mut AsyncApp| {
        // Let startup finish before the first network request.
        cx.background_executor()
            .timer(Duration::from_secs(20))
            .await;
        loop {
            let root = service.data_root();
            let enabled = computy_engine::preferences::load(&root)
                .map(|p| p.update_check)
                .unwrap_or(true);
            let idle = cx
                .update(|cx| cx.global::<Updates>().phase == Phase::Idle)
                .unwrap_or(false);
            if enabled
                && idle
                && computy_engine::updates::due(&root, computy_engine::domain::timestamp_ms())
            {
                run_check(cx).await;
            } else {
                // Keep in sync with checks made by the CLI.
                let saved = status(&service, false);
                let _ = cx.update(|cx| {
                    let updates = cx.global_mut::<Updates>();
                    if updates.phase == Phase::Idle && updates.status != saved {
                        updates.status = saved;
                        cx.refresh_windows();
                    }
                });
            }
            cx.background_executor()
                .timer(Duration::from_secs(3600))
                .await;
        }
    })
    .detach();
}

async fn run_check(cx: &mut AsyncApp) {
    let Ok(Some(service)) = cx.update(|cx| {
        let service = cx.global::<Updates>().service.clone()?;
        let updates = cx.global_mut::<Updates>();
        updates.phase = Phase::Checking;
        updates.error = None;
        cx.refresh_windows();
        Some(service)
    }) else {
        return;
    };
    let result = cx
        .background_executor()
        .spawn(async move { status(&service, true) })
        .await;
    let _ = cx.update(|cx| {
        let updates = cx.global_mut::<Updates>();
        updates.status = result;
        updates.phase = Phase::Idle;
        cx.refresh_windows();
    });
}

/// "Check now" in the preferences.
pub fn check_now(cx: &mut App) {
    if cx
        .try_global::<Updates>()
        .is_none_or(|u| u.phase != Phase::Idle)
    {
        return;
    }
    cx.spawn(async move |cx: &mut AsyncApp| run_check(cx).await)
        .detach();
}

/// Download, verify and install the latest release, then relaunch.
pub fn install(cx: &mut App) {
    let Some(updates) = cx.try_global::<Updates>() else {
        return;
    };
    let Some(asset) = updates
        .status
        .latest
        .as_ref()
        .filter(|_| updates.status.available && updates.phase == Phase::Idle)
        .and_then(|r| r.asset.clone())
    else {
        return;
    };
    let Some(service) = updates.service.clone() else {
        return;
    };
    let updates = cx.global_mut::<Updates>();
    updates.phase = Phase::Downloading;
    updates.error = None;
    updates.manual = false;
    cx.refresh_windows();
    cx.spawn(async move |cx: &mut AsyncApp| {
        let directory = service.data_root().join("updates");
        let asset = computy_engine::updates::Asset {
            name: asset.name,
            url: asset.url,
            size: asset.size,
            sha256: asset.sha256,
        };
        let downloaded = cx
            .background_executor()
            .spawn(async move { computy_engine::updates::download(&asset, &directory) })
            .await;
        let path = match downloaded {
            Ok(path) => path,
            Err(error) => return fail(cx, error),
        };
        let _ = cx.update(|cx| {
            cx.global_mut::<Updates>().phase = Phase::Installing;
            cx.refresh_windows();
        });
        let installed = cx
            .background_executor()
            .spawn(async move { crate::platform::install_update(&path) })
            .await;
        match installed {
            // Installed: start the new version once this process has exited.
            Ok(false) => {
                let _ = cx.update(|cx| {
                    if crate::platform::relaunch_after_exit().is_ok() {
                        crate::quit(cx);
                    } else {
                        let updates = cx.global_mut::<Updates>();
                        updates.phase = Phase::Idle;
                        updates.error = Some("Mise à jour installée : redémarrez Computy.".into());
                        cx.refresh_windows();
                    }
                });
            }
            // The installer was opened for the user to finish (macOS).
            Ok(true) => {
                let _ = cx.update(|cx| {
                    let updates = cx.global_mut::<Updates>();
                    updates.phase = Phase::Idle;
                    updates.manual = true;
                    cx.refresh_windows();
                });
            }
            Err(error) => fail(cx, error),
        }
    })
    .detach();
}

fn fail(cx: &mut AsyncApp, error: String) {
    let _ = cx.update(|cx| {
        let updates = cx.global_mut::<Updates>();
        updates.phase = Phase::Idle;
        updates.error = Some(error);
        cx.refresh_windows();
    });
}

/// Windows keeps the replaced executable until the next start.
fn remove_previous_executable() {
    #[cfg(target_os = "windows")]
    if let Ok(executable) = std::env::current_exe() {
        let _ = std::fs::remove_file(executable.with_extension("old.exe"));
    }
}
