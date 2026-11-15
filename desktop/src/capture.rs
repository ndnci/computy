//! Deterministic capture mode for visual regression tests. The harness
//! (`scripts/visual/capture-gpui.py`) starts the binary with a scenario, waits
//! for `ready`, takes a screenshot and sends `scroll <px>` or `quit` on stdin.
use crate::{
    app::Computy,
    backend::{Backend, Fixture},
};
use gpui::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{io::BufRead, sync::Arc, time::Duration};

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct Setup {
    #[serde(rename = "emptyHistory")]
    pub empty_history: bool,
    pub job: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct Scenario {
    pub id: String,
    pub page: String,
    pub actions: Vec<String>,
    pub setup: Setup,
    pub theme: String,
    pub locale: String,
    pub width: f32,
    pub height: f32,
    pub popover: Option<String>,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            id: String::new(),
            page: "overview".into(),
            actions: vec![],
            setup: Setup::default(),
            theme: "light".into(),
            locale: "fr".into(),
            width: 1440.,
            height: 960.,
            popover: None,
        }
    }
}

impl Scenario {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut scenario: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if scenario.popover.is_some() {
            scenario.width = 420.;
            scenario.height = 720.;
        }
        Ok(scenario)
    }
}

/// Same job states as the React harness (`jobFixture`).
fn job(
    kind: &str,
    engine: &serde_json::Map<String, Value>,
    reports: &serde_json::Map<String, Value>,
) -> Value {
    let latest = engine["history"]["reports"]
        .get(0)
        .and_then(|r| r["id"].as_str());
    let report = latest.and_then(|id| reports.get(id)).cloned();
    let checks = report
        .as_ref()
        .map(|r| r["checks"].clone())
        .unwrap_or(json!([]));
    if kind == "running" {
        return json!({
            "id": "visual-job", "running": true, "progress": 0.42,
            "phase": "Mémoire vive · motif 3 sur 8", "component": "memory",
            "checks": checks.as_array().map(|c| c.iter().take(1).cloned().collect::<Vec<_>>()).unwrap_or_default(),
            "report": null, "error": null, "cancel_requested": false
        });
    }
    json!({
        "id": report.as_ref().map(|r| r["id"].clone()).unwrap_or(json!("visual-job")),
        "running": false, "progress": 1, "phase": "Terminé", "component": "",
        "checks": checks, "report": report, "error": null, "cancel_requested": false
    })
}

pub fn backend(scenario: &Scenario) -> Backend {
    let path =
        std::env::var("COMPUTY_FIXTURE").unwrap_or_else(|_| "tests/visual/fixture.json".into());
    let data: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| {
            eprintln!("Computy capture: unable to read {path}");
            std::process::exit(2);
        });
    let mut engine = data["engine"].as_object().cloned().unwrap_or_default();
    let reports = data["reports"].as_object().cloned().unwrap_or_default();
    if let Some(preferences) = engine.get_mut("preferences").and_then(Value::as_object_mut) {
        preferences.insert("theme".into(), json!(scenario.theme));
        preferences.insert("language".into(), json!(scenario.locale));
    }
    if scenario.setup.empty_history {
        engine.insert("history".into(), json!({"reports": [], "unreadable": 0}));
    }
    let job = scenario
        .setup
        .job
        .as_deref()
        .map(|kind| job(kind, &engine, &reports));
    Backend::Fixture(Arc::new(Fixture {
        now: data["now"].as_i64().unwrap_or_default(),
        tauri: data["tauri"].as_object().cloned().unwrap_or_default(),
        engine,
        reports,
        job,
    }))
}

/// Apply the scenario's actions once data has loaded, then serve commands.
pub fn run(view: Entity<Computy>, scenario: Scenario, window: &mut Window, cx: &mut App) {
    let (sender, receiver) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let handle = window.window_handle();
    let executor = cx.background_executor().clone();
    cx.spawn(async move |cx| {
        let wait = |ms| executor.timer(Duration::from_millis(ms));
        wait(700).await;
        for action in &scenario.actions {
            let _ = handle.update(cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    crate::pages::capture_action(view, action, window, cx)
                });
            });
            wait(400).await;
        }
        wait(400).await;
        println!("ready");
        loop {
            match receiver.try_recv() {
                Ok(line) if line.starts_with("scroll ") => {
                    let offset: f32 = line[7..].trim().parse().unwrap_or(0.);
                    let _ = view.update(cx, |view, cx| {
                        view.scroll.set_offset(point(px(0.), px(-offset)));
                        cx.notify();
                    });
                    wait(300).await;
                    let actual = view
                        .read_with(cx, |view, _| -f32::from(view.scroll.offset().y))
                        .unwrap_or(0.);
                    println!("ready {actual}");
                }
                Ok(line) if line == "quit" => {
                    let _ = cx.update(|cx| cx.quit());
                    break;
                }
                Ok(_) => {}
                Err(std::sync::mpsc::TryRecvError::Empty) => wait(30).await,
                Err(_) => {
                    let _ = cx.update(|cx| cx.quit());
                    break;
                }
            }
        }
    })
    .detach();
}
