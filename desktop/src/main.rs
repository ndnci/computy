#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod assets;
mod backend;
mod capture;
mod components;
mod forms;
mod i18n;
mod instance;
mod intl;
mod layout;
mod model;
mod monitor_ui;
mod pages;
mod platform;
mod popover;
mod theme;
mod tray;
mod ui;

use backend::Backend;
use gpui::*;
use std::sync::OnceLock;

actions!(computy, [Quit]);

static CAPTURE: OnceLock<capture::Scenario> = OnceLock::new();

pub fn capture_mode() -> bool {
    CAPTURE.get().is_some()
}

/// Windows owned by the application. The main window exists only while it is
/// shown: closing it frees the interface entirely and leaves the tray running.
#[derive(Default)]
pub struct AppWindows {
    pub main: Option<WindowHandle<gpui_component::Root>>,
    pub popover: Option<AnyWindowHandle>,
    pub backend: Option<Backend>,
}

impl Global for AppWindows {}

static REQUESTED_METRIC: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Metric the processes page should sort by on its next opening (tray « Voir tout »).
pub fn set_requested_metric(metric: String, _cx: &mut App) {
    *REQUESTED_METRIC.lock().unwrap_or_else(|e| e.into_inner()) = Some(metric);
}

pub fn take_requested_metric() -> Option<String> {
    REQUESTED_METRIC
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}

pub fn quit(cx: &mut App) {
    tray::shutdown();
    if let Some(service) = cx
        .global::<AppWindows>()
        .backend
        .as_ref()
        .and_then(|b| b.service())
    {
        service.shutdown();
    }
    cx.quit();
}

/// Show the main window, creating it if needed.
pub fn open_main_window(page: Option<app::Page>, cx: &mut App) {
    if let Some(handle) = cx.global::<AppWindows>().main {
        let shown = handle
            .update(cx, |root, window, cx| {
                if let Some(page) = page
                    && let Ok(view) = root.view().clone().downcast::<app::Computy>()
                {
                    view.update(cx, |view, cx| view.navigate(page, window, cx));
                }
                window.activate_window();
            })
            .is_ok();
        if shown {
            return;
        }
    }
    let Some(backend) = cx.global::<AppWindows>().backend.clone() else {
        return;
    };
    let scenario = CAPTURE.get();
    let size = scenario.map_or(size(px(1440.), px(960.)), |s| {
        size(px(s.width), px(s.height))
    });
    let page = page
        .or_else(|| scenario.and_then(|s| app::Page::parse(&s.page)))
        .or_else(|| {
            std::env::var("COMPUTY_START_PAGE")
                .ok()
                .and_then(|p| app::Page::parse(&p))
        })
        .unwrap_or(app::Page::Overview);
    let options = platform::main_window_options(size, cx);
    let result = cx.open_window(options, move |window, cx| {
        platform::initialize_window(window);
        let view = cx.new(|cx| app::Computy::new(backend, page, window, cx));
        if let Some(scenario) = CAPTURE.get() {
            capture::run(view.clone(), scenario.clone(), window, cx);
        }
        cx.new(|cx| gpui_component::Root::new(view, window, cx))
    });
    match result {
        Ok(handle) => {
            cx.global_mut::<AppWindows>().main = Some(handle);
            handle
                .update(cx, |_, window, cx| {
                    window.on_window_should_close(cx, |_, cx| {
                        main_window_closed(cx);
                        true
                    });
                })
                .ok();
        }
        Err(error) => {
            eprintln!("Computy: unable to open the window: {error:#}");
        }
    }
}

/// Keep monitoring in the tray and free the interface.
fn main_window_closed(cx: &mut App) {
    cx.global_mut::<AppWindows>().main = None;
    if !tray::has_icons() {
        quit(cx);
    }
}

/// Close button of the client-drawn title bar: same path as the compositor's close.
pub fn close_main_window(window: &mut Window, cx: &mut App) {
    main_window_closed(cx);
    window.remove_window();
}

fn data_root() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("COMPUTY_DATA_DIR") {
        return path.into();
    }
    if cfg!(debug_assertions) && !capture_mode() {
        return std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.computy");
    }
    computy_engine::preferences::default_data_dir()
}

fn main() {
    env_logger::init();
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(index) = arguments.iter().position(|a| a == "--capture") {
        let scenario =
            capture::Scenario::parse(arguments.get(index + 1).map_or("", String::as_str))
                .unwrap_or_else(|error| {
                    eprintln!("Computy capture: {error}");
                    std::process::exit(2);
                });
        let _ = CAPTURE.set(scenario);
    }
    let background = computy_engine::startup::is_background(&arguments);
    let backend = match CAPTURE.get() {
        Some(scenario) => capture::backend(scenario),
        None => {
            // A second launch shows the existing window and exits.
            match instance::acquire(background) {
                instance::Outcome::Primary(listener) => {
                    instance::keep(listener);
                }
                instance::Outcome::Forwarded => return,
            }
            match Backend::live(data_root()) {
                Ok(backend) => backend,
                Err(error) => {
                    eprintln!("Computy: {error}");
                    std::process::exit(1);
                }
            }
        }
    };
    let preferences: model::Preferences = backend
        .typed("preferences", serde_json::json!({}))
        .unwrap_or_default();
    i18n::set_language(&preferences.language);
    let mode = if background {
        let engine_preferences =
            computy_engine::preferences::load(&data_root()).unwrap_or_default();
        computy_engine::startup::launch_mode(&arguments, &engine_preferences)
    } else {
        computy_engine::startup::LaunchMode::Window
    };
    if mode == computy_engine::startup::LaunchMode::Exit {
        return;
    }
    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            if let Err(error) = cx.text_system().add_fonts(
                assets::FONTS
                    .iter()
                    .map(|f| std::borrow::Cow::Borrowed(*f))
                    .collect(),
            ) {
                eprintln!("Computy: unable to load bundled fonts: {error:#}");
            }
            gpui_component::init(cx);
            cx.set_global(AppWindows {
                backend: Some(backend.clone()),
                ..Default::default()
            });
            cx.on_action(|_: &Quit, cx| quit(cx));
            cx.bind_keys([
                KeyBinding::new("ctrl-q", Quit, None),
                KeyBinding::new("cmd-q", Quit, None),
            ]);
            if !capture_mode() {
                if let Some(service) = backend.service().cloned() {
                    platform::start_housekeeping(service.clone());
                    platform::sync_login_startup(&service);
                    tray::start(service, cx);
                }
                instance::listen(cx);
            }
            if let Some(scenario) = CAPTURE.get().filter(|s| s.popover.is_some()) {
                popover::open(
                    scenario.popover.clone().unwrap_or_default(),
                    String::new(),
                    None,
                    cx,
                );
                return;
            }
            if mode == computy_engine::startup::LaunchMode::Window {
                open_main_window(None, cx);
                cx.activate(true);
            }
        });
}
