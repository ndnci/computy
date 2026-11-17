//! Application model and shell: navigation, top bar, footer, notifications,
//! running-diagnostic banner and report dialog (former App.tsx).
use crate::{
    backend::Backend,
    components::{self, progress_bar, spinner},
    i18n::{t, tn, tx},
    model::*,
    pages,
    theme::{self, c, hex},
    ui::{icon, shadow, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Overview,
    Diagnostics,
    Memory,
    Storage,
    Cpu,
    Fans,
    Processes,
    History,
    Applications,
    Clean,
    Compare,
    Settings,
    About,
}

impl Page {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "overview" => Self::Overview,
            "diagnostics" => Self::Diagnostics,
            "memory" => Self::Memory,
            "storage" => Self::Storage,
            "cpu" => Self::Cpu,
            "fans" => Self::Fans,
            "processes" => Self::Processes,
            "history" => Self::History,
            "applications" => Self::Applications,
            "clean" => Self::Clean,
            "compare" => Self::Compare,
            "settings" => Self::Settings,
            "about" => Self::About,
            _ => return None,
        })
    }

    pub fn component(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Storage => "storage",
            _ => "cpu",
        }
    }
}

struct NavItem {
    page: Page,
    label: &'static str,
    icon: &'static str,
}

const fn nav(page: Page, label: &'static str, icon: &'static str) -> NavItem {
    NavItem { page, label, icon }
}

const NAVIGATION: [&[NavItem]; 3] = [
    &[
        nav(Page::Overview, "common.overview", "layout-dashboard"),
        nav(Page::Diagnostics, "app.diagnostics", "activity"),
    ],
    &[
        nav(Page::Memory, "common.memory", "memory-stick"),
        nav(Page::Storage, "common.storage", "hard-drive"),
        nav(Page::Cpu, "common.processor", "cpu"),
        nav(Page::Fans, "common.fans", "fan"),
        nav(Page::Processes, "common.processes", "activity"),
    ],
    &[
        nav(Page::History, "app.history", "history"),
        nav(Page::Applications, "app.applications", "app-window"),
        nav(Page::Clean, "app.cleanup", "brush-cleaning"),
        nav(Page::Compare, "app.comparison", "arrow-left-right"),
    ],
];

pub struct Toast {
    pub message: String,
    pub error: bool,
    serial: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticMode {
    Session,
    Boot,
}

pub struct Computy {
    pub backend: Backend,
    pub page: Page,
    pub inventory: Option<Inventory>,
    pub loading: bool,
    pub error: Option<String>,
    pub refreshing: bool,
    pub reports: Vec<Report>,
    pub unreadable: u64,
    pub job: Option<Job>,
    pub launching: bool,
    pub diagnostic_mode: DiagnosticMode,
    pub selected_ids: Vec<String>,
    pub report: Option<Report>,
    pub toast: Option<Toast>,
    toast_serial: u64,
    pub preferences: Preferences,
    pub scroll: ScrollHandle,
    pub pages: pages::PageStates,
    pub capture: bool,
    pub quit_available: bool,
}

impl Computy {
    pub fn new(backend: Backend, page: Page, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let capture = backend.is_fixture();
        let preferences: Preferences = backend.typed("preferences", json!({})).unwrap_or_default();
        let mut this = Self {
            backend,
            page,
            inventory: None,
            loading: true,
            error: None,
            refreshing: false,
            reports: vec![],
            unreadable: 0,
            job: None,
            launching: false,
            diagnostic_mode: DiagnosticMode::Session,
            selected_ids: vec![],
            report: None,
            toast: None,
            toast_serial: 0,
            preferences,
            scroll: ScrollHandle::new(),
            pages: pages::PageStates::default(),
            capture,
            quit_available: true,
        };
        crate::i18n::set_language(&this.preferences.language);
        this.apply_theme(window);
        pages::enter(&mut this, window, cx);
        this.refresh(false, cx);
        this.reload_history(cx);
        this.call(
            "active",
            json!({}),
            cx,
            |this, result: Result<Option<Job>, String>, cx| {
                if let Ok(job) = result {
                    this.job = job;
                    this.poll_job(cx);
                }
            },
        );
        if !capture {
            this.collect_offline(cx);
        }
        window
            .observe_window_appearance(|window, cx| {
                cx.update_global::<crate::AppWindows, _>(|_, _| {});
                window.refresh();
            })
            .detach();
        this
    }

    /// Run an engine call off the UI thread and apply the result.
    pub fn call<T, F>(&self, method: &str, params: Value, cx: &mut Context<Self>, done: F)
    where
        T: serde::de::DeserializeOwned + Send + 'static,
        F: FnOnce(&mut Self, Result<T, String>, &mut Context<Self>) + 'static,
    {
        let backend = self.backend.clone();
        let method = method.to_owned();
        let task = cx
            .background_executor()
            .spawn(async move { backend.typed::<T>(&method, params) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                done(this, result, cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub fn apply_theme(&self, window: &Window) {
        let dark = match self.preferences.theme.as_str() {
            "dark" => true,
            "light" => false,
            _ => {
                window.appearance() == WindowAppearance::Dark
                    || window.appearance() == WindowAppearance::VibrantDark
            }
        };
        theme::set_dark(dark);
    }

    pub fn navigate(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.page = page;
        self.scroll.set_offset(point(px(0.), px(0.)));
        pages::enter(self, window, cx);
        cx.notify();
    }

    pub fn notify(&mut self, message: impl Into<String>, error: bool, cx: &mut Context<Self>) {
        self.toast_serial += 1;
        let serial = self.toast_serial;
        self.toast = Some(Toast {
            message: message.into(),
            error,
            serial,
        });
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(6500))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.toast.as_ref().is_some_and(|t| t.serial == serial) {
                    this.toast = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub fn reload_history(&mut self, cx: &mut Context<Self>) {
        self.call(
            "history",
            json!({}),
            cx,
            |this, result: Result<History, String>, cx| match result {
                Ok(history) => {
                    this.reports = history.reports;
                    this.unreadable = history.unreadable;
                }
                Err(error) => this.notify(error, true, cx),
            },
        );
    }

    pub fn refresh(&mut self, announce: bool, cx: &mut Context<Self>) {
        self.refreshing = true;
        self.call(
            "inventory",
            json!({"refresh": true}),
            cx,
            move |this, result: Result<Inventory, String>, cx| {
                this.refreshing = false;
                this.loading = false;
                match result {
                    Ok(inventory) => {
                        this.inventory = Some(inventory);
                        this.error = None;
                        if announce {
                            this.notify(t("app.hardware_information_refreshed"), false, cx);
                        }
                    }
                    Err(error) => {
                        this.error = Some(error);
                        if announce {
                            this.notify(t("app.could_not_refresh_hardware_information"), true, cx);
                        }
                    }
                }
            },
        );
        cx.notify();
    }

    fn collect_offline(&mut self, cx: &mut Context<Self>) {
        self.call(
            "offline_status",
            json!({}),
            cx,
            |this, result: Result<Value, String>, cx| {
                let Ok(state) = result else { return };
                if state["pending"] != true {
                    return;
                }
                this.call(
                    "offline_collect",
                    json!({}),
                    cx,
                    |this, result: Result<Value, String>, cx| match result {
                        Ok(value) if value["imported"] == true => {
                            this.reload_history(cx);
                            if let Some(id) = value["report_id"].as_str() {
                                this.call(
                                    "report",
                                    json!({"id": id}),
                                    cx,
                                    |this, report: Result<Report, String>, cx| {
                                        if let Ok(report) = report {
                                            this.report = Some(report);
                                            this.notify(
                                                t("app.windows_memory_result_has_been"),
                                                false,
                                                cx,
                                            );
                                        }
                                    },
                                );
                            }
                        }
                        Ok(_) => {
                            cx.spawn(async move |this, cx| {
                                cx.background_executor()
                                    .timer(Duration::from_secs(15))
                                    .await;
                                let _ = this.update(cx, |this, cx| this.collect_offline(cx));
                            })
                            .detach();
                        }
                        Err(error) => this.notify(error, true, cx),
                    },
                );
            },
        );
    }

    fn poll_job(&mut self, cx: &mut Context<Self>) {
        let Some(job) = self.job.clone().filter(|job| job.running) else {
            return;
        };
        if self.capture {
            return;
        }
        let id = job.id.clone();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(350))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.job.as_ref().is_none_or(|j| j.id != id || !j.running) {
                    return;
                }
                this.call(
                    "job",
                    json!({"id": id}),
                    cx,
                    |this, result: Result<Job, String>, cx| match result {
                        Ok(next) => {
                            let finished = !next.running;
                            let error = next.error.clone();
                            let completed = next.report.as_ref().is_some_and(|r| r.completed);
                            this.job = Some(next);
                            if finished {
                                this.reload_history(cx);
                                match error {
                                    Some(error) => this.notify(error, true, cx),
                                    None => this.notify(
                                        t(if completed {
                                            "app.diagnostic_completed_recorded"
                                        } else {
                                            "app.diagnostic_interrupted_partial_result_was"
                                        }),
                                        false,
                                        cx,
                                    ),
                                }
                            } else {
                                this.poll_job(cx);
                            }
                        }
                        Err(error) => {
                            this.notify(error, true, cx);
                            this.poll_job(cx);
                        }
                    },
                );
            });
        })
        .detach();
    }

    pub fn start(&mut self, options: RunOptions, cx: &mut Context<Self>) {
        if self.launching || self.job.as_ref().is_some_and(|j| j.running) {
            return;
        }
        self.launching = true;
        self.job = None;
        self.call(
            "start",
            serde_json::to_value(&options).unwrap_or_default(),
            cx,
            |this, result: Result<Value, String>, cx| match result {
                Ok(value) => {
                    let id = value["id"].as_str().unwrap_or_default().to_owned();
                    this.call(
                        "job",
                        json!({"id": id}),
                        cx,
                        |this, job: Result<Job, String>, cx| {
                            this.launching = false;
                            match job {
                                Ok(job) => {
                                    this.job = Some(job);
                                    this.poll_job(cx);
                                }
                                Err(error) => this.notify(error, true, cx),
                            }
                        },
                    );
                }
                Err(error) => {
                    this.launching = false;
                    this.notify(error, true, cx);
                }
            },
        );
        cx.notify();
    }

    pub fn relaunch(&mut self, report: Report, window: &mut Window, cx: &mut Context<Self>) {
        if self.launching || self.job.as_ref().is_some_and(|j| j.running) {
            return;
        }
        self.report = None;
        if report
            .source
            .as_deref()
            .is_some_and(|s| s != "computy_live")
        {
            self.diagnostic_mode = DiagnosticMode::Boot;
            self.navigate(Page::Diagnostics, window, cx);
            self.notify(t("app.choose_confirm_next_pre_boot"), false, cx);
            return;
        }
        self.diagnostic_mode = DiagnosticMode::Session;
        self.navigate(Page::Diagnostics, window, cx);
        let mut options = report.options.clone();
        options.label = report.label.clone();
        self.start(options, cx);
    }

    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.job.as_ref().map(|j| j.id.clone()) else {
            return;
        };
        self.call(
            "cancel",
            json!({"id": id.clone()}),
            cx,
            move |this, result: Result<Value, String>, cx| {
                if let Err(error) = result {
                    this.notify(error, true, cx);
                    return;
                }
                this.call(
                    "job",
                    json!({"id": id}),
                    cx,
                    |this, job: Result<Job, String>, _| {
                        if let Ok(job) = job {
                            this.job = Some(job);
                        }
                    },
                );
            },
        );
    }

    pub fn toggle_selection(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(index) = self.selected_ids.iter().position(|v| v == id) {
            self.selected_ids.remove(index);
        } else if self.selected_ids.len() >= 4 {
            self.notify(t("app.compare_up_four_diagnostics"), false, cx);
        } else {
            self.selected_ids.push(id.to_owned());
        }
        cx.notify();
    }

    pub fn export_report(&mut self, report: &Report, window: &mut Window, cx: &mut Context<Self>) {
        crate::platform::export_report(self, report.clone(), window, cx);
    }

    pub fn save_preferences(
        &mut self,
        value: Preferences,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let saved = value.clone();
        self.call(
            "save_preferences",
            serde_json::to_value(&value).unwrap_or_default(),
            cx,
            move |this, result: Result<Value, String>, cx| match result {
                Ok(_) => {
                    this.preferences = saved;
                    crate::i18n::set_language(&this.preferences.language);
                    crate::tray::preferences_changed(cx);
                    this.notify(t("app.preferences_recorded_device"), false, cx);
                    cx.refresh_windows();
                }
                Err(_) => this.notify(t("app.could_not_save_preferences"), true, cx),
            },
        );
        let _ = window;
    }

    fn page_label(&self) -> &'static str {
        match self.page {
            Page::Settings => "common.preferences",
            Page::About => "app.about",
            page => NAVIGATION
                .iter()
                .flat_map(|g| g.iter())
                .find(|n| n.page == page)
                .map(|n| n.label)
                .unwrap_or(""),
        }
    }
}

// Shell colors from the computed stylesheet.
const NAV_TEXT: u32 = 0x7c8780;
const NAV_ACTIVE: theme::Pair = (0x355739, 0xb9d39c);
const NAV_ACTIVE_BG: theme::Pair = (0xedf3e8, 0x33442a);
const NAV_HOVER_BG: theme::Pair = (0xf7f9f6, 0x253324);
const NAV_COUNT_BG: theme::Pair = (0xf1f3ee, 0x303d29);
const NAV_COUNT: theme::Pair = (0x7c8780, 0x9bb086);

impl Computy {
    fn nav_item(
        &self,
        item: &NavItem,
        bottom: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let active = self.page == item.page;
        let page = item.page;
        let badge = match item.page {
            Page::Diagnostics if self.job.as_ref().is_some_and(|j| j.running) => Some(None),
            Page::History if !self.reports.is_empty() => Some(Some(self.reports.len())),
            Page::Compare if !self.selected_ids.is_empty() => Some(Some(self.selected_ids.len())),
            Page::Settings if crate::updater::available(cx) => Some(None),
            _ => None,
        };
        let (size, icon_size, stroke, height, gap) = if bottom {
            (
                12.,
                18.,
                "2",
                38.,
                if crate::layout::narrow() { 9. } else { 10. },
            )
        } else {
            (
                13.,
                19.,
                "1.7",
                42.,
                if crate::layout::narrow() { 9. } else { 11. },
            )
        };
        let color = if active { c(NAV_ACTIVE) } else { hex(NAV_TEXT) };
        div()
            .id(SharedString::from(format!("nav-{page:?}")))
            .relative()
            .flex()
            .items_center()
            .gap(px(gap))
            .w_full()
            .h(px(height))
            .px(px(12.))
            .rounded(px(8.))
            .cursor_pointer()
            .when(active, |d| d.bg(c(NAV_ACTIVE_BG)))
            .when(!active, |d| d.hover(|s| s.bg(c(NAV_HOVER_BG))))
            .child(icon(item.icon, icon_size, stroke, color))
            .child(text(
                tx(item.label),
                size,
                if active { 650. } else { 500. },
                color,
            ))
            .when(active && !bottom, |d| {
                d.child(
                    div()
                        .absolute()
                        .left(px(-crate::layout::pick((12., 16., 20., 20.))))
                        .top(px(13.))
                        .bottom(px(13.))
                        .w(px(3.))
                        .rounded_r(px(3.))
                        .bg(hex(0x6a8c45)),
                )
            })
            .when(badge.is_some(), |d| d.child(div().flex_1()))
            .when_some(badge, |d, badge| match badge {
                None => d.child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .bg(hex(0x95b46e))
                        .shadow(vec![BoxShadow {
                            color: hex(0xe2edcd),
                            offset: point(px(0.), px(0.)),
                            blur_radius: px(0.),
                            spread_radius: px(4.),
                        }]),
                ),
                Some(count) => d.child(
                    div()
                        .min_w(px(20.))
                        .px(px(4.))
                        .py(px(2.))
                        .rounded(px(4.))
                        .bg(c(if active {
                            (0xdae8cf, 0x303d29)
                        } else {
                            NAV_COUNT_BG
                        }))
                        .flex()
                        .justify_center()
                        .child(
                            text(tn(count as f64), 10., 500., c(NAV_COUNT)).line_height(px(15.)),
                        ),
                ),
            })
            .on_click(cx.listener(move |this, _, window, cx| this.navigate(page, window, cx)))
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let groups: Vec<AnyElement> = NAVIGATION
            .iter()
            .enumerate()
            .map(|(index, items)| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .mb(px(24.))
                    .when(index > 0, |d| {
                        d.child(div().px(px(13.)).mb(px(8.)).child(spaced(
                            tx(if index == 1 {
                                "common.your_hardware_eyebrow"
                            } else {
                                "app.over_time_eyebrow"
                            }),
                            10.,
                            600.,
                            hex(0x9aa39b),
                            1.5,
                            16.5,
                        )))
                    })
                    .children(
                        items
                            .iter()
                            .map(|item| self.nav_item(item, false, cx).into_any_element()),
                    )
                    .into_any_element()
            })
            .collect();
        let settings = nav(Page::Settings, "common.preferences", "settings-2");
        let about = nav(Page::About, "app.about_computy", "circle-help");
        use crate::layout::pick;
        let (brand_gap, logo, word, word_lh, small, small_ls, small_lh, small_mt) = pick((
            (5., 36., 22., 24.2, 5., 1.2, 5.5, 8.),
            (8., 44., 24., 26.4, 6., 1.3, 6.6, 8.),
            (11., 44., 26., 28.6, 7., 1.45, 7.7, 8.),
            (11., 44., 26., 28.6, 7., 1.45, 7.7, 8.),
        ));
        div()
            .id("sidebar")
            .w(px(pick((188., 211., 231., 231.))))
            .flex_none()
            .h_full()
            .bg(c(theme::SIDEBAR))
            .border_r_1()
            .border_color(c(theme::BORDER))
            .pt(px(31.))
            .px(px(pick((12., 16., 20., 20.))))
            .pb(px(16.))
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .child(
                div()
                    .id("brand")
                    .flex()
                    .items_center()
                    .gap(px(brand_gap))
                    .px(px(4.))
                    .mb(px(37.))
                    .cursor_pointer()
                    .child(components::logo(logo))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .font_family("Sora")
                                    .text_size(px(word))
                                    .font_weight(FontWeight(600.))
                                    .line_height(px(word_lh))
                                    .child(spaced_word(&tx("computy"), -1.7, c(theme::TEXT)))
                                    .child(spaced_word(&tx("."), -1.7, hex(0x91ac69))),
                            )
                            .child(div().mt(px(small_mt)).child(spaced(
                                tx("app.hardware_clearly_eyebrow"),
                                small,
                                500.,
                                hex(0x6d7c70),
                                small_ls,
                                small_lh,
                            ))),
                    )
                    .on_click(
                        cx.listener(|this, _, window, cx| {
                            this.navigate(Page::Overview, window, cx)
                        }),
                    ),
            )
            .child(div().flex().flex_col().flex_1().children(groups))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .gap(px(5.))
                    .pt(px(5.))
                    .child(self.nav_item(&settings, true, cx))
                    .child(self.nav_item(&about, true, cx))
                    .when(self.quit_available, |d| {
                        d.child(
                            div()
                                .id("quit")
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .h(px(38.))
                                .px(px(12.))
                                .rounded(px(8.))
                                .cursor_pointer()
                                .hover(|s| s.bg(c(NAV_HOVER_BG)))
                                .child(icon("power", 18., "2", hex(NAV_TEXT)))
                                .child(text(tx("common.quit_computy"), 12., 500., hex(NAV_TEXT)))
                                .on_click(cx.listener(|_, _, _, cx| crate::quit(cx))),
                        )
                    }),
            )
    }

    fn topbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let busy = self.refreshing || self.job.as_ref().is_some_and(|j| j.running);
        let monogram = self
            .inventory
            .as_ref()
            .map(|i| {
                i.host_name
                    .chars()
                    .take(2)
                    .collect::<String>()
                    .to_uppercase()
            })
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| "PC".into());
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .h(px(73.))
            .px(px(crate::layout::pick((24., 27., 37., 37.))))
            .bg(c(theme::TOPBAR))
            .border_b_1()
            .border_color(c(theme::BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(icon("monitor", 16., "2", hex(0x869386)))
                    .child(text(tx("common.my_computer"), 12., 400., hex(0xa0a8a2)))
                    .child(icon("chevron-right", 14., "2", hex(0xa0a8a2)))
                    .child(text(tx(self.page_label()), 12., 500., hex(0x657466))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(if crate::layout::narrow() { 12. } else { 18. }))
                    .when(self.loading || self.error.is_some(), |d| {
                        d.child(text(
                            tx(if self.loading {
                                "app.engine_connection"
                            } else {
                                "app.engine_not_available"
                            }),
                            11.,
                            400.,
                            hex(0x84917f),
                        ))
                    })
                    .child(
                        div()
                            .id("refresh")
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .h(px(30.))
                            .px(px(10.))
                            .rounded(px(7.))
                            .when(busy, |d| d.opacity(0.5))
                            .when(!busy, |d| {
                                d.cursor_pointer().hover(|s| s.bg(c(NAV_HOVER_BG)))
                            })
                            .child(spinner_or_icon(
                                self.refreshing,
                                "refresh-cw",
                                17.,
                                hex(0x6d7c70),
                            ))
                            .when(!crate::layout::narrow(), |d| {
                                d.child(text(
                                    tx("app.refresh_hardware_info"),
                                    11.,
                                    400.,
                                    hex(0x6d7c70),
                                ))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !busy {
                                    this.refresh(true, cx)
                                }
                            })),
                    )
                    .child(
                        div()
                            .size(px(30.))
                            .rounded_full()
                            .border_1()
                            .border_color(hex(0xe2e8db))
                            .bg(hex(0xf0f4e7))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(text(monogram, 10., 600., hex(0x7d8d63))),
                    ),
            )
    }

    fn footer(&self) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .pt(px(20.))
            .px(px(crate::layout::pick((27., 27., 37., 37.))))
            .pb(px(22.))
            .border_t_1()
            .border_color(c(theme::BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(text(tx("Computy"), 10., 400., hex(0xa4ad9b)))
                    .child(text(env!("CARGO_PKG_VERSION"), 10., 500., hex(0x929d87)))
                    .child(div().size(px(3.)).rounded_full().bg(hex(0xb9c2af)))
                    .child(text(tx("app.carefully_done"), 10., 400., hex(0xa4ad9b)))
                    .child(
                        div()
                            .id("ndnci")
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .cursor_pointer()
                            .child(spaced(tx("NDNCI"), 10., 600., hex(0x879776), 0.3, 13.))
                            .child(icon("arrow-up-right", 12., "2", hex(0x879776)))
                            .on_click(|_, _, cx| {
                                crate::platform::open_external("https://www.ndnci.com", cx)
                            }),
                    ),
            )
    }

    fn floating_job(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let job = self
            .job
            .as_ref()
            .filter(|j| j.running && self.page != Page::Diagnostics)?;
        Some(
            div()
                .id("floating-job")
                .absolute()
                .bottom(px(28.))
                .left(px(257.))
                .min_w(px(350.))
                .flex()
                .items_center()
                .gap(px(15.))
                .py(px(15.))
                .px(px(19.))
                .bg(c(theme::GREEN))
                .border_1()
                .border_color(hex(0x3f6043))
                .rounded(px(12.))
                .shadow(vec![shadow(0x173123, 0.19, 10., 35.)])
                .cursor_pointer()
                .child(spinner(18., hex(0xeef6e7)))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(text(
                            tx("app.diagnostics_progress"),
                            11.,
                            550.,
                            hex(0xeef6e7),
                        ))
                        .child(text(tx(&job.phase), 9., 400., hex(0xc0d4a7)).mt(px(5.)))
                        .child(div().mt(px(9.)).child(progress_bar(
                            job.progress,
                            3.,
                            c((0x517044, 0x39492f)),
                            hex(0xc0d990),
                        ))),
                )
                .child(text(
                    format!("{}{}", tn((job.progress * 100.).round()), tx(" %")),
                    11.,
                    550.,
                    hex(0xeef6e7),
                ))
                .child(icon("chevron-right", 18., "2", hex(0xeef6e7)))
                .on_click(
                    cx.listener(|this, _, window, cx| this.navigate(Page::Diagnostics, window, cx)),
                ),
        )
    }

    fn toast_view(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let toast = self.toast.as_ref()?;
        let error = toast.error;
        Some(
            div()
                .absolute()
                .bottom(px(25.))
                .right(px(30.))
                .max_w(px(380.))
                .flex()
                .items_center()
                .gap(px(11.))
                .pt(px(12.))
                .pr(px(12.))
                .pb(px(12.))
                .pl(px(15.))
                .bg(c(theme::PANEL))
                .border_1()
                .border_color(if error {
                    hex(0xecd4c3)
                } else {
                    c((0xd7e4c8, 0x57723e))
                })
                .rounded(px(11.))
                .shadow(vec![shadow(0x304623, 0.1, 10., 35.)])
                .child(
                    div()
                        .size(px(29.))
                        .flex_none()
                        .rounded(px(8.))
                        .bg(if error {
                            hex(0xfcf1e8)
                        } else {
                            c((0xf0f6e8, 0x30452a))
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(
                            if error { "circle-help" } else { "check" },
                            18.,
                            "2",
                            hex(if error { 0xbd9981 } else { 0x8aaa6c }),
                        )),
                )
                .child(
                    div().flex_1().child(
                        text(
                            tx(&toast.message),
                            11.,
                            400.,
                            hex(if error { 0xaf937b } else { 0x91a779 }),
                        )
                        .line_height(px(18.7)),
                    ),
                )
                .child(
                    div()
                        .id("toast-close")
                        .size(px(30.))
                        .flex_none()
                        .rounded(px(7.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(icon("x", 16., "2", c(theme::MUTED)))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toast = None;
                            cx.notify();
                        })),
                ),
        )
    }

    fn report_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let report = self.report.clone()?;
        let disabled = self.launching || self.job.as_ref().is_some_and(|j| j.running);
        let details = components::report_details(self, &report, true, disabled, window, cx);
        Some(components::modal(
            "report",
            tx("app.diagnostic_report"),
            true,
            details,
            std::rc::Rc::new(cx.listener(|this, _, _, cx| {
                this.report = None;
                cx.notify();
            })),
            window,
        ))
    }
}

fn spinner_or_icon(spin: bool, name: &str, size: f32, color: Hsla) -> AnyElement {
    if spin {
        spinner(size, color).into_any_element()
    } else {
        icon(name, size, "2", color).into_any_element()
    }
}

/// A single word with letter spacing (the brand wordmark).
fn spaced_word(word: &str, spacing: f32, color: Hsla) -> Div {
    div()
        .flex()
        .text_color(color)
        .children(word.chars().enumerate().map(move |(i, ch)| {
            let glyph = div().child(SharedString::from(ch.to_string()));
            if i > 0 { glyph.ml(px(spacing)) } else { glyph }
        }))
        .mr(px(spacing))
}

impl Render for Computy {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_theme(window);
        crate::layout::update(f32::from(window.viewport_size().width));
        let page = pages::render(self, window, cx);
        let floating = self.floating_job(cx).map(|e| e.into_any_element());
        let toast = self.toast_view(cx).map(|e| e.into_any_element());
        let modal = self.report_modal(window, cx);
        let modal = modal.or_else(|| pages::modal(self, window, cx));
        let title_bar = matches!(window.window_decorations(), Decorations::Client { .. })
            .then(|| title_bar(window));
        let content = div()
            .id("computy")
            .flex_1()
            .min_h_0()
            .w_full()
            .relative()
            .flex()
            .flex_row()
            .font_family("DM Sans")
            .text_color(c(theme::TEXT))
            .bg(c(theme::BG))
            .child(self.sidebar(cx))
            .child(
                div()
                    .id("main-scroll")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_h(window.viewport_size().height)
                            .child(self.topbar(cx))
                            .child(
                                div()
                                    .flex_1()
                                    .w_full()
                                    .min_w_0()
                                    .max_w(px(1610.))
                                    .mx_auto()
                                    .pt(px(crate::layout::pick((28., 33., 33., 40.))))
                                    .px(px(crate::layout::pick((24., 27., 37., 37.))))
                                    .pb(px(crate::layout::pick((22., 24., 24., 24.))))
                                    .when_some(self.error.clone(), |d, error| {
                                        d.child(components::engine_error(error, cx))
                                    })
                                    .child(page),
                            )
                            .child(self.footer()),
                    ),
            )
            .children(floating)
            .children(toast)
            .children(modal);
        div()
            .key_context("Computy")
            .size_full()
            .flex()
            .flex_col()
            .font_family("DM Sans")
            .bg(c(theme::BG))
            .children(title_bar)
            .child(content)
            .on_action(cx.listener(|_, _: &crate::Quit, _, cx| crate::quit(cx)))
    }
}

/// Linux title bar drawn by the application: drag to move, double-click to
/// maximize, right-click for the compositor's window menu, and window buttons.
fn title_bar(window: &Window) -> impl IntoElement + use<> {
    let maximized = window.is_maximized();
    let control = |id: &'static str, name: &'static str, danger: bool| {
        div()
            .id(id)
            .w(px(46.))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .hover(move |s| {
                if danger {
                    s.bg(hex(0xe5484d))
                } else {
                    s.bg(c(theme::BORDER))
                }
            })
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .child(icon(name, 15., "1.8", c(theme::MUTED)))
    };
    div()
        .id("title-bar")
        .flex_shrink_0()
        .h(px(crate::platform::TITLE_BAR_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .bg(c(theme::TOPBAR))
        .border_b_1()
        .border_color(c(theme::BORDER))
        .on_mouse_down(MouseButton::Left, |event, window, _| {
            if event.click_count == 2 {
                window.zoom_window();
            } else {
                window.start_window_move();
            }
        })
        .on_mouse_down(MouseButton::Right, |event, window, _| {
            window.show_window_menu(event.position)
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pl(px(14.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(img("brand/logo-96.png").size(px(16.)))
                .child(text("Computy", 12.5, 550., c(theme::MUTED))),
        )
        .child(
            control("window-minimize", "minus", false)
                .on_click(|_, window, _| window.minimize_window()),
        )
        .child(
            control(
                "window-maximize",
                if maximized { "copy" } else { "square" },
                false,
            )
            .on_click(|_, window, _| window.zoom_window()),
        )
        .child(
            control("window-close", "x", true)
                .on_click(|_, window, cx| crate::close_main_window(window, cx)),
        )
}
