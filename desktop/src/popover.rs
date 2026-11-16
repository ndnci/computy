//! Tray popover (former MonitorPopover.tsx). The window exists only while it
//! is shown and closes when it loses focus.
use crate::{
    app::Page,
    backend::Backend,
    components::spinner,
    i18n::{tp, tx},
    intl,
    model::{MonitorSnapshot, Preferences, ProcessSample, ranked_processes},
    monitor_ui::{metric_icon, metric_visual, usage_color},
    pages::processes::avatar,
    theme::{self, c, hex},
    ui::{icon, one_line, shadow, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use serde_json::json;
use std::{collections::HashMap, time::Duration};

pub struct Popover {
    backend: Backend,
    metric: String,
    volume: String,
    snapshot: Option<MonitorSnapshot>,
    problem: bool,
    icons: HashMap<u32, Option<std::sync::Arc<RenderImage>>>,
    _poll: Task<()>,
}

impl Popover {
    fn new(backend: Backend, metric: String, volume: String, cx: &mut Context<Self>) -> Self {
        let poll = cx.spawn(async move |this, cx| {
            loop {
                let Ok(backend) = this.read_with(cx, |this, _| this.backend.clone()) else {
                    break;
                };
                let fixture = backend.is_fixture();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let snapshot = backend.typed::<MonitorSnapshot>("monitor", json!({}));
                        let preferences =
                            backend.typed::<Preferences>("preferences", json!({})).ok();
                        let icons: HashMap<u32, Option<std::sync::Arc<RenderImage>>> =
                            match (&snapshot, fixture) {
                                (Ok(s), false) => s
                                    .processes
                                    .iter()
                                    .take(64)
                                    .map(|p| (p.pid, crate::platform::process_image(p.pid)))
                                    .collect(),
                                _ => HashMap::new(),
                            };
                        (snapshot, preferences, icons)
                    })
                    .await;
                let alive = this.update(cx, |this, cx| {
                    let (snapshot, preferences, icons) = result;
                    if let Some(preferences) = preferences {
                        crate::i18n::set_language(&preferences.language);
                    }
                    match snapshot {
                        Ok(snapshot) => {
                            this.snapshot = Some(snapshot);
                            this.problem = false;
                        }
                        Err(_) => this.problem = true,
                    }
                    this.icons = icons;
                    cx.notify();
                });
                if alive.is_err() || fixture {
                    break;
                }
                cx.background_executor().timer(Duration::from_secs(2)).await;
            }
        });
        Self {
            backend,
            metric,
            volume,
            snapshot: None,
            problem: false,
            icons: HashMap::new(),
            _poll: poll,
        }
    }

    fn action(&mut self, destination: &str, window: &mut Window, cx: &mut Context<Self>) {
        let metric = self.metric.clone();
        match destination {
            "quit" => crate::quit(cx),
            "hide" => window.remove_window(),
            _ => {
                let page = match destination {
                    "settings" => Page::Settings,
                    "processes" => Page::Processes,
                    _ => Page::Overview,
                };
                window.remove_window();
                cx.defer(move |cx| {
                    if page == Page::Processes {
                        crate::set_requested_metric(metric, cx);
                    }
                    crate::open_main_window(Some(page), cx);
                });
            }
        }
    }
}

fn amount(p: &ProcessSample, metric: &str, locale: &str) -> String {
    match metric {
        "memory" => intl::bytes(locale, p.memory_bytes, 1),
        "gpu" => intl::bytes(locale, p.gpu_memory_bytes, 1),
        "storage" => format!("{}/s", intl::bytes(locale, p.value("storage"), 1)),
        _ => tp(
            "common.percent_value",
            &[(
                "value",
                ((p.cpu_percent.unwrap_or(0.) * 10.).round() / 10.).into(),
            )],
        ),
    }
}

fn share(p: &ProcessSample, metric: &str, locale: &str) -> String {
    let value = match metric {
        "memory" => p.memory_percent,
        "storage" => p.io_percent,
        _ => None,
    };
    match value {
        Some(v) => tp(
            "common.percent_value",
            &[("value", ((v * 10.).round() / 10.).into())],
        ),
        None if metric == "cpu" => intl::bytes(locale, p.memory_bytes, 1),
        None => String::new(),
    }
}

fn action_row(
    id: &'static str,
    icon_name: &str,
    label: &str,
    arrow: Option<f32>,
    highlighted: bool,
    cx: &mut Context<Popover>,
) -> impl IntoElement {
    let destination = match id {
        "popover-open" => "overview",
        "popover-settings" => "settings",
        "popover-quit" => "quit",
        _ => "processes",
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .py(px(11.))
        .px(px(10.))
        .rounded(px(8.))
        .cursor_pointer()
        .when(highlighted, |d| d.bg(c((0xeaf1e7, 0x2a3e24))))
        .when(!highlighted, |d| d.hover(|s| s.bg(c((0xf7f9f6, 0x253324)))))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(icon(icon_name, 17., "2", hex(0x6d7c70)))
                .child(text(
                    tx(label),
                    11.,
                    if highlighted { 650. } else { 400. },
                    if highlighted {
                        c(theme::GREEN)
                    } else {
                        c(theme::TEXT)
                    },
                )),
        )
        .when_some(arrow, |d, size| {
            d.child(icon("arrow-up-right", size, "2", hex(0x6d7c70)))
        })
        .on_click(cx.listener(move |this, _, window, cx| this.action(destination, window, cx)))
}

impl Render for Popover {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = match self
            .backend
            .typed::<Preferences>("preferences", json!({}))
            .map(|p| p.theme)
            .as_deref()
        {
            Ok("dark") => true,
            Ok("light") => false,
            _ => matches!(
                window.appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ),
        };
        theme::set_dark(dark);
        let locale = crate::i18n::locale();
        let metric = self.metric.as_str();
        let value = self
            .snapshot
            .as_ref()
            .and_then(|s| s.metric(metric, &self.volume));
        let top: Vec<ProcessSample> = ranked_processes(
            self.snapshot.as_ref().map_or(&[][..], |s| &s.processes),
            metric,
        )
        .into_iter()
        .filter(|p| p.value(metric).is_some())
        .take(10)
        .collect();
        let name = match metric {
            "memory" => "common.memory",
            "cpu" => "common.processor",
            "gpu" => "common.graphics_processor",
            _ => "common.storage",
        };
        let device = match metric {
            "memory" => "RAM".to_owned(),
            "storage" => "SSD / HDD".to_owned(),
            other => other.to_uppercase(),
        };
        let list = if self.snapshot.is_none() && !self.problem {
            div()
                .flex()
                .justify_center()
                .py(px(30.))
                .child(spinner(18., c(theme::MUTED)))
                .into_any_element()
        } else if top.is_empty() {
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .p(px(20.))
                .child(icon("circle-help", 20., "2", c(theme::MUTED)))
                .child(text(
                    tx("common.no_per_process_measurement_available"),
                    11.,
                    400.,
                    c(theme::MUTED),
                ))
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .children(top.iter().map(|p| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .py(px(8.))
                        .px(px(6.))
                        .rounded(px(9.))
                        .child(avatar(self.icons.get(&p.pid).and_then(Option::as_ref)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    one_line(
                                        if p.name.is_empty() {
                                            tx("common.name_unavailable")
                                        } else {
                                            p.name.clone()
                                        },
                                        11.,
                                        600.,
                                        c(theme::TEXT),
                                    )
                                    .line_height(px(16.5)),
                                )
                                .child(
                                    text(format!("PID {}", p.pid), 9., 400., hex(0x6d7c70))
                                        .line_height(px(13.5)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_end()
                                .child(
                                    text(amount(p, metric, locale), 11., 600., c(theme::TEXT))
                                        .line_height(px(16.5)),
                                )
                                .child(
                                    text(share(p, metric, locale), 9., 400., hex(0x6d7c70))
                                        .line_height(px(13.5)),
                                ),
                        )
                        .into_any_element()
                }))
                .into_any_element()
        };
        let capture = crate::capture_mode();
        div()
            .size_full()
            .when(capture, |d| d.bg(hex(0xffffff)))
            .font_family("DM Sans")
            .text_color(c(theme::TEXT))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .bg(c(theme::PANEL))
                    .border_1()
                    .border_color(c(theme::BORDER))
                    .rounded(px(18.))
                    .shadow(vec![shadow(0x000000, 0.13, 8., 25.)])
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .pt(px(10.))
                            .px(px(16.))
                            .h(px(40.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(div().size(px(6.)).rounded_full().bg(hex(0x87a66e)))
                                    .child(spaced(
                                        tx("common.live_eyebrow"),
                                        9.,
                                        650.,
                                        hex(0x6d7c70),
                                        1.3,
                                        16.,
                                    ))
                                    .child(text("Computy", 12., 900., c(theme::TEXT)).ml(px(4.))),
                            )
                            .child(
                                div()
                                    .id("popover-close")
                                    .size(px(30.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(7.))
                                    .cursor_pointer()
                                    .child(icon("x", 17., "2", hex(0x6d7c70)))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.action("hide", window, cx)
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(21.))
                            .pt(px(16.))
                            .px(px(23.))
                            .pb(px(22.))
                            .border_b_1()
                            .border_color(c(theme::BORDER))
                            .child(metric_visual(value, metric, "ring", 96., 60.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child(icon(
                                                metric_icon(metric),
                                                22.,
                                                "2",
                                                hex(0x6d7c70),
                                            ))
                                            .child(spaced(
                                                device,
                                                11.,
                                                650.,
                                                hex(0x6d7c70),
                                                1.,
                                                22.,
                                            )),
                                    )
                                    .child(
                                        spaced(tx(name), 19., 650., c(theme::HEADING), -0.6, 28.5)
                                            .family("Sora")
                                            .mt(px(4.)),
                                    )
                                    .child(
                                        text(
                                            if self.volume.is_empty() {
                                                tx("common.my_computer")
                                            } else {
                                                self.volume.clone()
                                            },
                                            11.,
                                            400.,
                                            hex(0x6d7c70),
                                        )
                                        .line_height(px(18.15))
                                        .mt(px(4.)),
                                    )
                                    .child(
                                        text(
                                            value.map_or_else(
                                                || tx("common.unavailable"),
                                                |v| {
                                                    tp(
                                                        "common.usage",
                                                        &[("value", v.round().into())],
                                                    )
                                                },
                                            ),
                                            11.,
                                            600.,
                                            usage_color(value),
                                        )
                                        .mt(px(10.)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .pt(px(16.))
                            .px(px(15.))
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(10.))
                                    .px(px(5.))
                                    .pb(px(10.))
                                    .child(spaced(
                                        tx("common.top_resource_consumers"),
                                        12.,
                                        600.,
                                        c(theme::HEADING),
                                        -0.4,
                                        19.2,
                                    ))
                                    .child(
                                        div()
                                            .py(px(3.))
                                            .px(px(7.))
                                            .rounded(px(8.))
                                            .bg(c((0xeaf1e7, 0x2a3e24)))
                                            .child(text(
                                                top.len().to_string(),
                                                9.,
                                                400.,
                                                c(theme::TEXT),
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .id("popover-list")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .child(list),
                            )
                            .when(metric == "gpu", |d| {
                                d.child(
                                    text(
                                        tx("popover.nvidia_compute_process_gpu_memory"),
                                        10.,
                                        400.,
                                        hex(0x6d7c70),
                                    )
                                    .px(px(5.))
                                    .pt(px(8.)),
                                )
                            })
                            .when(metric == "storage", |d| {
                                d.child(
                                    text(
                                        tx("popover.per_process_activity_all_volumes"),
                                        10.,
                                        400.,
                                        hex(0x6d7c70),
                                    )
                                    .px(px(5.))
                                    .pt(px(8.)),
                                )
                            })
                            .child(div().mt(px(9.)).mb(px(12.)).child(action_row(
                                "popover-all",
                                "external-link",
                                "common.view_all",
                                Some(16.),
                                true,
                                cx,
                            ))),
                    )
                    .when(self.problem, |d| {
                        d.child(
                            text(
                                tx("common.live_measurements_unavailable_computy_retries"),
                                10.,
                                400.,
                                hex(0x6d7c70),
                            )
                            .px(px(20.))
                            .pb(px(8.)),
                        )
                    })
                    .child(
                        div()
                            .pt(px(6.))
                            .px(px(15.))
                            .pb(px(12.))
                            .border_t_1()
                            .border_color(c(theme::BORDER))
                            .child(action_row(
                                "popover-open",
                                "external-link",
                                "common.open_computy",
                                Some(15.),
                                false,
                                cx,
                            ))
                            .child(action_row(
                                "popover-settings",
                                "settings-2",
                                "common.preferences",
                                Some(15.),
                                false,
                                cx,
                            ))
                            .child(action_row(
                                "popover-quit",
                                "power",
                                "common.quit_computy",
                                None,
                                false,
                                cx,
                            )),
                    ),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.action("hide", window, cx);
                }
            }))
    }
}

/// Show the popover for a tray metric. On X11 and Windows the window opens
/// next to the indicator; Wayland compositors choose the position.
pub fn open(metric: String, volume: String, anchor: Option<(i32, i32)>, cx: &mut App) {
    if let Some(handle) = cx.global::<crate::AppWindows>().popover {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
        cx.global_mut::<crate::AppWindows>().popover = None;
    }
    let Some(backend) = cx.global::<crate::AppWindows>().backend.clone() else {
        return;
    };
    let capture = crate::capture_mode();
    let display = cx.primary_display();
    let size = size(px(420.), px(720.));
    let bounds = match (anchor, display.as_ref()) {
        (Some((x, y)), Some(display)) => {
            let screen = display.bounds();
            let height = size.height.min(screen.size.height - px(48.)).max(px(300.));
            let x = (px(x as f32) - size.width / 2.).clamp(
                screen.origin.x + px(8.),
                (screen.origin.x + screen.size.width - size.width - px(8.))
                    .max(screen.origin.x + px(8.)),
            );
            let y = if px(y as f32) < screen.origin.y + screen.size.height / 2. {
                px(y as f32 + 12.)
            } else {
                px(y as f32) - height - px(12.)
            };
            Bounds::new(point(x, y), gpui::size(size.width, height))
        }
        _ => Bounds::centered(None, size, cx),
    };
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        focus: !capture,
        show: true,
        kind: if capture {
            WindowKind::Normal
        } else {
            WindowKind::PopUp
        },
        is_movable: false,
        is_resizable: false,
        window_background: if capture {
            WindowBackgroundAppearance::Opaque
        } else {
            WindowBackgroundAppearance::Transparent
        },
        window_decorations: Some(WindowDecorations::Client),
        app_id: Some(crate::platform::APP_ID.into()),
        ..Default::default()
    };
    let result = cx.open_window(options, move |window, cx| {
        let view = cx.new(|cx| {
            if !capture {
                cx.observe_window_activation(window, |_, window, cx| {
                    if !window.is_window_active() {
                        window.remove_window();
                        cx.update_global::<crate::AppWindows, _>(|windows, _| {
                            windows.popover = None
                        });
                    }
                })
                .detach();
            }
            Popover::new(backend, metric, volume, cx)
        });
        if capture {
            crate::capture::ready_after_delay(window, cx);
        }
        view
    });
    if let Ok(handle) = result {
        cx.global_mut::<crate::AppWindows>().popover = Some(handle.into());
    }
}
