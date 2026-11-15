//! Page views. Each page keeps local state like its React component did; the
//! state is reset when the page is entered again.
mod about;
pub mod applications;
pub mod clean;
mod compare;
pub mod diagnostics;
pub mod fans;
mod hardware;
pub mod history;
pub mod offline;
pub mod overview;
pub mod processes;
pub mod settings;

use crate::app::{Computy, Page};
use gpui::*;
use std::collections::HashSet;

#[derive(Default)]
pub struct PageStates {
    /// Expanded `<details>` blocks (check cards, report footnotes).
    pub open_details: HashSet<String>,
    /// The select box whose option list is open.
    pub open_select: Option<SharedString>,
    pub diagnostics: Option<diagnostics::DiagnosticsState>,
    pub offline: Option<offline::OfflineState>,
    pub history: Option<history::HistoryState>,
    pub processes: Option<processes::ProcessesState>,
    pub applications: Option<applications::ApplicationsState>,
    pub clean: Option<clean::CleanState>,
    pub fans: Option<fans::FansState>,
    pub settings: Option<settings::SettingsState>,
    /// Metric requested by the tray popover's « Voir tout ».
    pub requested_metric: Option<String>,
}

/// Reset the entered page's local state, like a React remount.
pub fn enter(app: &mut Computy, _window: &mut Window, _cx: &mut Context<Computy>) {
    app.pages.open_details.clear();
    app.pages.open_select = None;
    app.pages.diagnostics = None;
    app.pages.offline = None;
    app.pages.history = None;
    app.pages.processes = None;
    app.pages.applications = None;
    app.pages.clean = None;
    app.pages.fans = None;
    app.pages.settings = None;
}

pub fn render(app: &mut Computy, window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    match app.page {
        Page::Overview => overview::render(app, window, cx),
        Page::Memory | Page::Storage | Page::Cpu => hardware::render(app, window, cx),
        Page::Diagnostics => diagnostics::render(app, window, cx),
        Page::History => history::render(app, window, cx),
        Page::Compare => compare::render(app, window, cx),
        Page::Processes => processes::render(app, window, cx),
        Page::Applications => applications::render(app, window, cx),
        Page::Clean => clean::render(app, window, cx),
        Page::Fans => fans::render(app, window, cx),
        Page::About => about::render(app, window, cx),
        Page::Settings => settings::render(app, window, cx),
    }
}

/// Page-owned dialogs (confirmations, rename…).
pub fn modal(
    app: &mut Computy,
    window: &mut Window,
    cx: &mut Context<Computy>,
) -> Option<AnyElement> {
    match app.page {
        Page::History => history::modal(app, window, cx),
        Page::Applications => applications::modal(app, window, cx),
        Page::Clean => clean::modal(app, window, cx),
        Page::Settings => settings::modal(app, window, cx),
        _ => None,
    }
}

/// Scenario actions used by the visual regression harness.
pub fn capture_action(
    app: &mut Computy,
    action: &str,
    window: &mut Window,
    cx: &mut Context<Computy>,
) {
    match action {
        "clean-preview" => clean::capture_preview(app, cx),
        "boot-mode" => {
            app.diagnostic_mode = crate::app::DiagnosticMode::Boot;
            cx.notify();
        }
        "select-two" => {
            app.selected_ids = app.reports.iter().take(2).map(|r| r.id.clone()).collect();
            cx.notify();
        }
        "open-compare" => app.navigate(Page::Compare, window, cx),
        "open-latest-report" => {
            app.report = app.reports.first().cloned();
            cx.notify();
        }
        _ => {
            let _ = window;
            eprintln!("Computy capture: unknown action {action}");
        }
    }
}
