//! Diagnostics: configuration, live progress and result (former Diagnostics.tsx).
use crate::{
    app::{Computy, DiagnosticMode},
    components::{
        self, ButtonKind, NoticeTone, button, button_with, check_card, notice, report_details,
        spinner,
    },
    forms::{checkbox_visual, field, option, radio_visual, select, text_input},
    i18n::{tn, tp, tx},
    model::RunOptions,
    theme::{self, c, hex},
    ui::{icon, panel, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::input::InputState;

pub struct DiagnosticsState {
    pub selected: Vec<&'static str>,
    pub profile: &'static str,
    pub memory_mib: u64,
    pub passes: u64,
    pub disk_mib: u64,
    pub cpu_seconds: u64,
    pub label: Entity<InputState>,
    pub show_configuration: bool,
    pub advanced_open: bool,
}

impl DiagnosticsState {
    pub fn new(app: &Computy, window: &mut Window, cx: &mut Context<Computy>) -> Self {
        let p = &app.preferences;
        let running = app.launching || app.job.as_ref().is_some_and(|j| j.running);
        Self {
            selected: vec!["memory", "storage", "cpu"],
            profile: "standard",
            memory_mib: p.memory_mib,
            passes: p.passes,
            disk_mib: p.disk_mib,
            cpu_seconds: p.cpu_seconds,
            label: cx.new(|cx| {
                InputState::new(window, cx).placeholder(tx("diagnostics.label_placeholder"))
            }),
            show_configuration: !running,
            advanced_open: false,
        }
    }
}

const COMPONENTS: [(&str, &str, &str, &str); 3] = [
    (
        "memory",
        "common.memory",
        "diagnostics.patterns_moving_inversions_available_ecc",
        "memory-stick",
    ),
    (
        "storage",
        "common.storage",
        "diagnostics.smart_nvme_temporary_file_integrity",
        "hard-drive",
    ),
    (
        "cpu",
        "common.processor",
        "diagnostics.parallel_calculations_checked_against_known",
        "cpu",
    ),
];

fn state_mut(app: &mut Computy) -> &mut DiagnosticsState {
    app.pages.diagnostics.as_mut().expect("diagnostics state")
}

fn mode_choice(app: &Computy, mode: DiagnosticMode, cx: &mut Context<Computy>) -> impl IntoElement {
    let selected = app.diagnostic_mode == mode;
    let (name, detail, icon_name) = match mode {
        DiagnosticMode::Session => (
            "diagnostics.my_session",
            "diagnostics.ram_storage_processor_without_restart",
            "activity",
        ),
        DiagnosticMode::Boot => (
            "diagnostics.before_system_startup",
            "diagnostics.memory_only_reboot_selected_confirmed",
            "power",
        ),
    };
    div()
        .id(if mode == DiagnosticMode::Session {
            "mode-session"
        } else {
            "mode-boot"
        })
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(14.))
        .p(px(19.))
        .rounded(px(14.))
        .border_1()
        .cursor_pointer()
        .map(|d| {
            if selected {
                d.bg(c((0xeaf1e7, 0x2a3e24))).border_color(c(theme::GREEN))
            } else {
                d.bg(c(theme::PANEL)).border_color(c(theme::BORDER))
            }
        })
        .child(icon(icon_name, 22., "2", c(theme::TEXT)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(5.))
                .child(text(tx(name), 14., 700., c(theme::TEXT)))
                .child(text(tx(detail), 12., 400., hex(0x6d7c70)).line_height(px(18.))),
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            this.diagnostic_mode = mode;
            if mode == DiagnosticMode::Boot {
                state_mut(this).show_configuration = true;
            }
            cx.notify();
        }))
}

fn running(app: &Computy, cx: &mut Context<Computy>) -> Vec<AnyElement> {
    let job = app.job.clone().unwrap_or_default();
    let percent = (job.progress * 100.).round();
    let checks: Vec<AnyElement> = job
        .checks
        .iter()
        .map(|check| check_card(app, check, cx).into_any_element())
        .collect();
    vec![
        div()
            .flex()
            .flex_col()
            .p(px(30.))
            .mb(px(23.))
            .bg(c((0xf0f6e7, 0x2c4028)))
            .border_1()
            .border_color(c((0xdce7cf, 0x405531)))
            .rounded(px(16.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(20.))
                    .child(
                        div()
                            .size(px(62.))
                            .flex_none()
                            .rounded(px(17.))
                            .bg(c((0xe5efd6, 0x31472a)))
                            .border_1()
                            .border_color(c((0xd5e3c2, 0x475c34)))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("scan-line", 29., "2", c((0x8ca669, 0xa3c17f)))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(spaced(
                                tx("diagnostics.rust_engine_working_eyebrow"),
                                8.,
                                650.,
                                hex(0x899582),
                                1.75,
                                10.,
                            ))
                            .child(
                                spaced(
                                    tx(if job.cancel_requested {
                                        "diagnostics.stopping_diagnostic"
                                    } else {
                                        "diagnostics.we_re_looking_all"
                                    }),
                                    23.,
                                    600.,
                                    c(theme::HEADING),
                                    -0.7,
                                    30.,
                                )
                                .mt(px(10.)),
                            )
                            .child(
                                text(tx(&job.phase), 11., 400., hex(0x8b9f74))
                                    .line_height(px(18.15))
                                    .mt(px(9.)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .child(spaced(
                                tn(percent),
                                43.,
                                500.,
                                c((0x628446, 0xa8c989)),
                                -1.,
                                56.,
                            ))
                            .child(spaced(
                                tx("%"),
                                16.,
                                500.,
                                c((0x628446, 0xa8c989)),
                                -1.,
                                21.,
                            )),
                    ),
            )
            .child(
                div()
                    .mt(px(26.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .mb(px(9.))
                            .child(text(
                                tx("diagnostics.measured_progression"),
                                10.,
                                400.,
                                hex(0x99aa86),
                            ))
                            .child(text(
                                format!("{}{}", tn(percent), tx(" %")),
                                10.,
                                500.,
                                hex(0x809964),
                            )),
                    )
                    .child(components::progress_bar(
                        job.progress,
                        6.,
                        c((0xe2ebd8, 0x39492f)),
                        hex(0x91ae6e),
                    )),
            )
            .child(
                div()
                    .mt(px(21.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(15.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .child(icon("lock-keyhole", 14., "2", hex(0x9bad88)))
                            .child(text(
                                tx("diagnostics.personal_files_not_modified"),
                                10.,
                                400.,
                                hex(0x9bad88),
                            )),
                    )
                    .child({
                        let color = c((0x65715f, 0xb1c699));
                        let lead = if job.cancel_requested {
                            spinner(14., color)
                        } else {
                            icon("square", 13., "2", color).into_any_element()
                        };
                        button_with(
                            "cancel-job",
                            ButtonKind::Secondary,
                            true,
                            Some(lead),
                            tx(if job.cancel_requested {
                                "diagnostics.stopping"
                            } else {
                                "diagnostics.stop_diagnostic"
                            }),
                            job.cancel_requested,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.cancel(cx)))
                    }),
            )
            .into_any_element(),
        panel()
            .mb(px(20.))
            .flex()
            .flex_col()
            .child(components::panel_header(
                "diagnostics.results_arrive_over_checks",
                Some(tx("diagnostics.progress_follows_actual_operations")),
                None,
            ))
            .child(if checks.is_empty() {
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(23.))
                    .pb(px(23.))
                    .child(spinner(18., c(theme::MUTED)))
                    .child(text(
                        tx("diagnostics.preparation_inventory_first_check"),
                        12.,
                        400.,
                        c(theme::MUTED),
                    ))
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .px(px(23.))
                    .pb(px(23.))
                    .children(checks)
                    .into_any_element()
            })
            .into_any_element(),
        notice(
            NoticeTone::Info,
            vec![tx("diagnostics.continue_browse_computy_stop_releases").into_any_element()],
        )
        .text_size(px(12.))
        .line_height(px(21.))
        .into_any_element(),
    ]
}

fn component_option(
    app: &Computy,
    entry: (&'static str, &str, &str, &str),
    cx: &mut Context<Computy>,
) -> impl IntoElement {
    let (id, name, description, icon_name) = entry;
    let selected = app
        .pages
        .diagnostics
        .as_ref()
        .is_some_and(|s| s.selected.contains(&id));
    let (icon_bg, icon_color) = match id {
        "memory" => ((0xeef3e8, 0x304129), (0x7e9b64, 0xa4c580)),
        "storage" => ((0xfaf3e6, 0x403a29), (0xb29a6a, 0xbcad80)),
        _ => ((0xf1edf6, 0x3a3445), (0xa290b3, 0xb4a4c8)),
    };
    div()
        .id(SharedString::from(format!("component-option-{id}")))
        .flex()
        .items_center()
        .gap(px(13.))
        .p(px(15.))
        .rounded(px(10.))
        .border_1()
        .cursor_pointer()
        .map(|d| {
            if selected {
                d.bg(c((0xf6f9f1, 0x304129)))
                    .border_color(c((0xd9e5c9, 0x405333)))
            } else {
                d.bg(c(theme::PANEL)).border_color(c(theme::BORDER))
            }
        })
        .child(checkbox_visual(selected))
        .child(
            div()
                .size(px(39.))
                .flex_none()
                .rounded(px(10.))
                .bg(c(icon_bg))
                .flex()
                .items_center()
                .justify_center()
                .child(icon(icon_name, 23., "2", c(icon_color))),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(5.))
                .child(text(
                    tx(name),
                    13.,
                    600.,
                    if selected {
                        c((0x263d34, 0xa4c580))
                    } else {
                        c(theme::TEXT)
                    },
                ))
                .child(text(tx(description), 11., 400., hex(0x788d65)).line_height(px(18.7))),
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            let state = state_mut(this);
            if let Some(index) = state.selected.iter().position(|v| *v == id) {
                state.selected.remove(index);
            } else {
                state.selected.push(id);
            }
            cx.notify();
        }))
}

fn profile_card(app: &Computy, id: &'static str, cx: &mut Context<Computy>) -> impl IntoElement {
    let p = &app.preferences;
    let (name, icon_name, detail, tag) = match id {
        "quick" => (
            "diagnostics.quick_check",
            "zap",
            tx("64 Mio · 1 passe"),
            "diagnostics.light_badge",
        ),
        "standard" => (
            "diagnostics.essentials",
            "scan-line",
            tp(
                "diagnostics.mio_passes",
                &[
                    ("v0", (p.memory_mib as f64).into()),
                    ("v1", (p.passes as f64).into()),
                ],
            ),
            "diagnostics.recommended_badge",
        ),
        _ => (
            "diagnostics.thorough_session",
            "check-check",
            tx("512 Mio · 4 passes"),
            "diagnostics.thorough_badge",
        ),
    };
    let selected = app
        .pages
        .diagnostics
        .as_ref()
        .is_some_and(|s| s.profile == id);
    div()
        .id(SharedString::from(format!("profile-{id}")))
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .py(px(16.))
        .px(px(14.))
        .rounded(px(10.))
        .border_1()
        .cursor_pointer()
        .map(|d| {
            if selected {
                d.bg(c((0xf2f7ea, 0x304129)))
                    .border_color(c((0xb6cc99, 0x6d8954)))
                    .shadow(vec![BoxShadow {
                        color: c((0xd1dfbd, 0x405333)),
                        offset: point(px(0.), px(0.)),
                        blur_radius: px(0.),
                        spread_radius: px(1.),
                    }])
            } else {
                d.bg(c(theme::PANEL)).border_color(c(theme::BORDER))
            }
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(icon(icon_name, 20., "2", hex(0x9bb27d)))
                .child(radio_visual(selected)),
        )
        .child(
            spaced(
                tx(tag),
                8.,
                400.,
                if selected {
                    hex(0x7d985f)
                } else {
                    hex(0xa5b68d)
                },
                1.,
                10.,
            )
            .mt(px(17.)),
        )
        .child(text(tx(name), 12., 600., c(theme::TEXT)).mt(px(7.)))
        .child(text(detail, 10., 400., hex(0xa1ad92)).mt(px(13.)))
        .on_click(cx.listener(move |this, _, _, cx| {
            let preferences = this.preferences.clone();
            let state = state_mut(this);
            state.profile = id;
            let (m, p, d, s) = match id {
                "quick" => (64, 1, 16, 2),
                "extended" => (512, 4, 128, 15),
                _ => (
                    preferences.memory_mib,
                    preferences.passes,
                    preferences.disk_mib,
                    preferences.cpu_seconds,
                ),
            };
            state.memory_mib = m;
            state.passes = p;
            state.disk_mib = d;
            state.cpu_seconds = s;
            cx.notify();
        }))
}

fn configuration(app: &Computy, cx: &mut Context<Computy>) -> AnyElement {
    let state = app.pages.diagnostics.as_ref().unwrap();
    let locale = crate::i18n::locale();
    let selected = state.selected.clone();
    let budget = app.inventory.as_ref().map_or(0, |i| {
        (i.memory
            .test_budget_bytes
            .unwrap_or_else(|| (i.memory.available_bytes / 3.).min(i.memory.total_bytes / 4.))
            / 1024f64.powi(2))
        .floor() as u64
    });
    let exceeds =
        selected.contains(&"memory") && app.inventory.is_some() && state.memory_mib > budget;
    let disabled = app.launching
        || selected.is_empty()
        || app.error.is_some()
        || app.inventory.is_none()
        || exceeds;
    let mib = |v: u64| {
        format!(
            "{}{}",
            crate::intl::plain(locale, v as f64),
            tx(" common.unit_mio")
        )
    };
    let edit = |key: &'static str| {
        move |this: &mut Computy, value: String, _: &mut Window, _: &mut Context<Computy>| {
            let value: u64 = value.parse().unwrap_or(0);
            let state = state_mut(this);
            match key {
                "memory_mib" => state.memory_mib = value,
                "passes" => state.passes = value,
                "disk_mib" => state.disk_mib = value,
                _ => state.cpu_seconds = value,
            }
            state.profile = "custom";
        }
    };
    let summary_row = |label: &str, value: String| {
        div()
            .flex()
            .justify_between()
            .gap(px(9.))
            .py(px(13.))
            .border_b_1()
            .border_color(c(theme::BORDER))
            .child(text(tx(label), 11., 400., hex(0xa0ac92)))
            .child(text(value, 11., 500., hex(0x7d9267)))
    };
    let advanced_open = state.advanced_open;
    let form = div()
        .flex_grow()
        .flex_basis(relative(0.))
        .map(|mut d| {
            d.style().flex_grow = Some(1.95);
            d
        })
        .min_w_0()
        .flex()
        .flex_col()
        .child(
            panel()
                .mb(px(20.))
                .flex()
                .flex_col()
                .child(components::panel_header(
                    "diagnostics.step1_what_to_check",
                    Some(tx("diagnostics.each_component_retains_results_limits")),
                    None,
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(11.))
                        .px(px(22.))
                        .pb(px(22.))
                        .children(
                            COMPONENTS
                                .iter()
                                .map(|entry| component_option(app, *entry, cx).into_any_element()),
                        ),
                ),
        )
        .child(
            panel()
                .mb(px(20.))
                .flex()
                .flex_col()
                .child(components::panel_header(
                    "diagnostics.step2_choose_depth",
                    Some(tx("diagnostics.all_these_profiles_run_session")),
                    None,
                ))
                .child(
                    div()
                        .flex()
                        .gap(px(10.))
                        .px(px(22.))
                        .pb(px(22.))
                        .child(profile_card(app, "quick", cx))
                        .child(profile_card(app, "standard", cx))
                        .child(profile_card(app, "extended", cx)),
                )
                .child(
                    div()
                        .px(px(23.))
                        .border_t_1()
                        .border_color(c(theme::BORDER))
                        .child(
                            div()
                                .id("advanced")
                                .flex()
                                .items_center()
                                .justify_between()
                                .py(px(15.))
                                .cursor_pointer()
                                .child(text(
                                    tx("diagnostics.customize_settings "),
                                    12.,
                                    400.,
                                    hex(0x8b9d77),
                                ))
                                .child(text(
                                    tx(if state.profile == "custom" {
                                        "diagnostics.custom_settings"
                                    } else {
                                        "diagnostics.optional"
                                    }),
                                    10.,
                                    400.,
                                    hex(0xb0bba4),
                                ))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let s = state_mut(this);
                                    s.advanced_open = !s.advanced_open;
                                    cx.notify();
                                })),
                        )
                        .when(advanced_open, |d| {
                            d.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(17.))
                                    .pb(px(20.))
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(17.))
                                            .child(field(
                                                "common.memory_to_allocate",
                                                select(
                                                    "memory_mib",
                                                    &state.memory_mib.to_string(),
                                                    [16, 32, 64, 128, 256, 512, 1024, 2048]
                                                        .map(|v| option(v, mib(v)))
                                                        .into_iter()
                                                        .collect(),
                                                    app,
                                                    cx,
                                                    edit("memory_mib"),
                                                ),
                                            ))
                                            .child(field(
                                                "common.memory_passes",
                                                select(
                                                    "passes",
                                                    &state.passes.to_string(),
                                                    [1, 2, 3, 4, 6, 8]
                                                        .map(|v| option(v, tn(v as f64)))
                                                        .into_iter()
                                                        .collect(),
                                                    app,
                                                    cx,
                                                    edit("passes"),
                                                ),
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(17.))
                                            .child(field(
                                                "diagnostics.integrity_file",
                                                select(
                                                    "disk_mib",
                                                    &state.disk_mib.to_string(),
                                                    [16, 32, 64, 128, 256, 512]
                                                        .map(|v| option(v, mib(v)))
                                                        .into_iter()
                                                        .collect(),
                                                    app,
                                                    cx,
                                                    edit("disk_mib"),
                                                ),
                                            ))
                                            .child(field(
                                                "diagnostics.duration_cpu_calculations",
                                                select(
                                                    "cpu_seconds",
                                                    &state.cpu_seconds.to_string(),
                                                    [1, 2, 5, 10, 15, 30, 60]
                                                        .map(|v| {
                                                            option(
                                                                v,
                                                                format!(
                                                                    "{}{}",
                                                                    tn(v as f64),
                                                                    tx(" common.seconds")
                                                                ),
                                                            )
                                                        })
                                                        .into_iter()
                                                        .collect(),
                                                    app,
                                                    cx,
                                                    edit("cpu_seconds"),
                                                ),
                                            )),
                                    ),
                            )
                        }),
                ),
        )
        .child(
            panel()
                .pb(px(22.))
                .flex()
                .flex_col()
                .child(components::panel_header(
                    "diagnostics.step3_name_moment",
                    Some(tx("diagnostics.simple_benchmark_future_comparisons")),
                    None,
                ))
                .child(
                    div()
                        .px(px(23.))
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .mb(px(10.))
                        .child(text(
                            tx("history.diagnostic_name "),
                            12.,
                            400.,
                            hex(0x8d9e79),
                        ))
                        .child(text(
                            tx("diagnostics.optional_lowercase"),
                            9.,
                            400.,
                            hex(0xb2bca7),
                        )),
                )
                .child(div().px(px(23.)).child(text_input(&state.label))),
        );
    let summary = div()
        .flex_grow()
        .flex_basis(relative(0.))
        .min_w(px(300.))
        .flex()
        .flex_col()
        .child(
            panel()
                .flex()
                .flex_col()
                .pt(px(24.))
                .px(px(23.))
                .pb(px(24.))
                .mb(px(19.))
                .child(
                    div()
                        .size(px(46.))
                        .rounded(px(12.))
                        .bg(c((0xf0f5e8, 0x304129)))
                        .border_1()
                        .border_color(c((0xe1ead4, 0x405333)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon("shield-check", 25., "2", c((0x91a978, 0xa4c580)))),
                )
                .child(
                    spaced(
                        tx("diagnostics.ready_take_stock"),
                        18.,
                        650.,
                        c(theme::HEADING),
                        -0.5,
                        24.,
                    )
                    .mt(px(19.)),
                )
                .child(
                    text(
                        tx("diagnostics.snapshot_hardware_results_recorded_computer"),
                        12.,
                        400.,
                        hex(0x7c9068),
                    )
                    .line_height(px(22.8))
                    .mt(px(10.)),
                )
                .child(
                    div()
                        .mt(px(22.))
                        .border_t_1()
                        .border_color(c(theme::BORDER))
                        .flex()
                        .flex_col()
                        .child(summary_row(
                            "diagnostics.components",
                            tn(selected.len() as f64),
                        ))
                        .when(selected.contains(&"memory"), |d| {
                            d.child(summary_row(
                                "diagnostics.tested_memory",
                                mib(state.memory_mib),
                            ))
                            .child(summary_row("common.memory_passes", tn(state.passes as f64)))
                        })
                        .when(selected.contains(&"storage"), |d| {
                            d.child(summary_row("common.temporary_file", mib(state.disk_mib)))
                        })
                        .when(selected.contains(&"cpu"), |d| {
                            d.child(summary_row(
                                "diagnostics.cpu_calculations",
                                format!(
                                    "{}{}",
                                    tn(state.cpu_seconds as f64),
                                    tx(" diagnostics.up_4_threads")
                                ),
                            ))
                        }),
                )
                .when(exceeds, |d| {
                    d.child(
                        notice(
                            NoticeTone::Warn,
                            vec![
                                format!(
                                    "{}{}{}",
                                    tx("diagnostics.too_high_allowance_available_memory "),
                                    tn(budget as f64),
                                    tx(" diagnostics.mib_in_settings")
                                )
                                .into_any_element(),
                            ],
                        )
                        .mt(px(18.)),
                    )
                })
                .child(
                    div().mt(px(19.)).child(
                        button_with(
                            "launch",
                            ButtonKind::Primary,
                            false,
                            Some(if app.launching {
                                spinner(18., hex(0xffffff))
                            } else {
                                icon("play", 17., "2", hex(0xffffff)).into_any_element()
                            }),
                            tx(if app.launching {
                                "diagnostics.preparation"
                            } else {
                                "diagnostics.run_diagnostic"
                            }),
                            disabled,
                        )
                        .w_full()
                        .child(icon("arrow-right", 17., "2", hex(0xffffff)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if disabled {
                                return;
                            }
                            let state = state_mut(this);
                            state.show_configuration = false;
                            let options = RunOptions {
                                components: state.selected.iter().map(|s| s.to_string()).collect(),
                                memory_mib: state.memory_mib,
                                passes: state.passes,
                                disk_mib: state.disk_mib,
                                cpu_seconds: state.cpu_seconds,
                                label: state.label.read(cx).value().to_string(),
                            };
                            this.start(options, cx);
                        })),
                    ),
                ),
        )
        .child(
            div()
                .py(px(3.))
                .px(px(11.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(icon("circle-help", 20., "2", hex(0xa0ae8f)))
                        .child(spaced(
                            tx("diagnostics.what_we_actually_verify"),
                            11.,
                            600.,
                            hex(0x8e9f78),
                            -0.2,
                            14.,
                        )),
                )
                .child(
                    text(
                        tx("diagnostics.os_memory_test_covers_limited"),
                        12.,
                        400.,
                        hex(0x7c9068),
                    )
                    .line_height(px(22.8))
                    .mt(px(12.)),
                )
                .child(
                    text(
                        tx("diagnostics.note_summarizes_observed_controls_without"),
                        12.,
                        400.,
                        hex(0x7c9068),
                    )
                    .line_height(px(22.8))
                    .mt(px(10.)),
                ),
        );
    div()
        .flex()
        .items_start()
        .gap(px(22.))
        .child(form)
        .child(summary)
        .into_any_element()
}

pub fn render(app: &mut Computy, window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    if app.pages.diagnostics.is_none() {
        let state = DiagnosticsState::new(app, window, cx);
        app.pages.diagnostics = Some(state);
    }
    let running = app.job.as_ref().is_some_and(|j| j.running);
    if running || app.launching {
        state_mut(app).show_configuration = false;
    }
    let show_configuration = app
        .pages
        .diagnostics
        .as_ref()
        .is_some_and(|s| s.show_configuration);
    let body: Vec<AnyElement> = if app.diagnostic_mode == DiagnosticMode::Boot && !running {
        vec![super::offline::render(app, window, cx)]
    } else if running {
        running_view(app, cx)
    } else if let Some(report) = app
        .job
        .as_ref()
        .and_then(|j| j.report.clone())
        .filter(|_| !show_configuration)
    {
        let error = app.job.as_ref().and_then(|j| j.error.clone());
        let launching = app.launching;
        vec![
            panel()
                .flex()
                .flex_col()
                .child(components::panel_header(
                    if report.completed {
                        "diagnostics.report_ready"
                    } else {
                        "diagnostics.diagnostic_was_interrupted"
                    },
                    None,
                    Some(
                        button(
                            "new-run",
                            ButtonKind::Primary,
                            true,
                            Some("rotate-ccw"),
                            tx("common.new_diagnostic"),
                            false,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            state_mut(this).show_configuration = true;
                            cx.notify();
                        }))
                        .into_any_element(),
                    ),
                ))
                .when_some(error, |d, error| {
                    d.child(
                        notice(NoticeTone::Warn, vec![tx(&error).into_any_element()])
                            .mx(px(23.))
                            .mb(px(18.)),
                    )
                })
                .child(
                    div()
                        .px(px(23.))
                        .pb(px(23.))
                        .child(report_details(app, &report, true, launching, window, cx)),
                )
                .into_any_element(),
        ]
    } else {
        vec![configuration(app, cx)]
    };
    div()
        .flex()
        .flex_col()
        .child(components::section_title(
            Some("diagnostics.take_care_eyebrow"),
            "diagnostics.diagnostics_make_sense",
            Some("diagnostics.real_controls_explained_results_history"),
            None,
        ))
        .when(!running && !app.launching, |d| {
            d.child(
                div()
                    .flex()
                    .gap(px(16.))
                    .mb(px(25.))
                    .child(mode_choice(app, DiagnosticMode::Session, cx))
                    .child(mode_choice(app, DiagnosticMode::Boot, cx)),
            )
        })
        .children(body)
        .into_any_element()
}

fn running_view(app: &Computy, cx: &mut Context<Computy>) -> Vec<AnyElement> {
    running(app, cx)
}
