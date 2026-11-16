//! Side-by-side report comparison (former pages/Compare.tsx).
use crate::{
    app::{Computy, Page},
    components::{
        self, ButtonKind, NoticeTone, Tone, badge, button, button_foreground, button_with,
        component_summary, empty_sized, notice, score, text_button,
    },
    i18n::{tn, tx},
    intl,
    model::Report,
    theme::{self, c, hex},
    ui::{icon, panel, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use std::collections::BTreeSet;

fn check<'a>(report: &'a Report, id: &str) -> Option<&'a crate::model::Check> {
    report.checks.iter().find(|c| c.id == id)
}

fn metric(report: &Report, id: &str, key: &str) -> Option<f64> {
    check(report, id)?.metrics.get(key)?.as_f64()
}

pub fn warnings(reports: &[Report]) -> Vec<&'static str> {
    let mut warnings = vec![];
    let machines: BTreeSet<&str> = reports
        .iter()
        .map(|r| r.inventory.machine_id.as_str())
        .collect();
    if machines.len() > 1 {
        warnings.push("compare.reports_come_different_machines_their");
    }
    let protocols: BTreeSet<String> = reports
        .iter()
        .map(|r| {
            let mut components = r.options.components.clone();
            components.sort();
            format!(
                "{}|{:?}|{}|{}|{}|{}",
                r.source.as_deref().unwrap_or("computy_live"),
                components,
                r.options.memory_mib,
                r.options.passes,
                r.options.cpu_seconds,
                r.options.disk_mib
            )
        })
        .collect();
    if protocols.len() > 1 {
        warnings.push("compare.test_paracounters_differ_performance_not");
    }
    if reports.iter().any(|r| !r.completed) {
        warnings.push("compare.diagnostic_was_interrupted_coverage_incomplete");
    }
    warnings
}

enum Value {
    Text(String),
    Score(Option<f64>, crate::model::Status),
}

struct Row {
    label: &'static str,
    get: Box<dyn Fn(&Report) -> Value>,
    numeric: Option<fn(&Report) -> f64>,
}

enum Line {
    Section(&'static str),
    Row(Row),
}

fn rows() -> Vec<Line> {
    let locale = crate::i18n::locale();
    let bytes = move |v: Option<f64>| intl::bytes(locale, v, 1);
    let text_or = |value: Option<f64>, missing: &'static str| {
        value.map_or_else(|| tx(missing), |v| intl::plain(locale, v))
    };
    let row = |label: &'static str, get: Box<dyn Fn(&Report) -> Value>| {
        Line::Row(Row {
            label,
            get,
            numeric: None,
        })
    };
    vec![
        Line::Section("compare.hardware_configuration"),
        Line::Row(Row {
            label: "compare.ram_visible_os",
            get: Box::new(move |r| Value::Text(bytes(Some(r.inventory.memory.total_bytes)))),
            numeric: Some(|r| r.inventory.memory.total_bytes),
        }),
        row(
            "compare.modules_identified",
            Box::new(|r| {
                let n = r.inventory.memory.modules.len();
                Value::Text(if n > 0 {
                    tn(n as f64)
                } else {
                    tx("common.not_reported_plural")
                })
            }),
        ),
        row(
            "compare.type_memory",
            Box::new(|r| {
                let mut kinds: Vec<&str> = vec![];
                for m in &r.inventory.memory.modules {
                    if !kinds.contains(&m.kind.as_str()) {
                        kinds.push(&m.kind);
                    }
                }
                Value::Text(if kinds.is_empty() {
                    tx("common.not_reported")
                } else {
                    kinds.join(" / ")
                })
            }),
        ),
        row(
            "compare.configured_memory_rate",
            Box::new(|r| {
                let mut values: Vec<f64> = vec![];
                for m in &r.inventory.memory.modules {
                    if let Some(v) = m.configured_speed_mts.filter(|v| *v != 0.)
                        && !values.contains(&v)
                    {
                        values.push(v);
                    }
                }
                Value::Text(if values.is_empty() {
                    tx("common.not_reported")
                } else {
                    values
                        .iter()
                        .map(|v| format!("{} MT/s", intl::plain("fr", *v)))
                        .collect::<Vec<_>>()
                        .join(" / ")
                })
            }),
        ),
        row(
            "compare.module_references",
            Box::new(|r| {
                let parts: Vec<String> = r
                    .inventory
                    .memory
                    .modules
                    .iter()
                    .map(|m| {
                        if m.part_number.is_empty() {
                            tx("common.not_reported_feminine")
                        } else {
                            m.part_number.clone()
                        }
                    })
                    .collect();
                Value::Text(if parts.is_empty() {
                    tx("compare.not_reported_feminine_plural")
                } else {
                    parts.join(" · ")
                })
            }),
        ),
        row(
            "common.processor",
            Box::new(|r| {
                Value::Text(if r.inventory.cpu.name.is_empty() {
                    tx("common.not_reported")
                } else {
                    r.inventory.cpu.name.clone()
                })
            }),
        ),
        row(
            "common.system",
            Box::new(|r| {
                Value::Text(format!(
                    "{} {} · {}",
                    r.inventory.os, r.inventory.os_version, r.inventory.arch
                ))
            }),
        ),
        Line::Section("compare.what_has_actually_been_tested"),
        row(
            "compare.checked_memory_allocation",
            Box::new(move |r| {
                Value::Text(if check(r, "memory-patterns").is_some() {
                    bytes(metric(r, "memory-patterns", "allocated_bytes"))
                } else {
                    tx("compare.not_implemented")
                })
            }),
        ),
        row(
            "compare.memory_passes_completed",
            Box::new(move |r| {
                Value::Text(text_or(
                    metric(r, "memory-patterns", "passes_completed"),
                    "compare.not_implemented",
                ))
            }),
        ),
        row(
            "compare.memory_inconsistencies",
            Box::new(move |r| {
                Value::Text(text_or(
                    metric(r, "memory-patterns", "errors"),
                    "compare.not_measured",
                ))
            }),
        ),
        row(
            "compare.memory_result",
            Box::new(|r| {
                let s = component_summary(&r.checks, "memory");
                Value::Score(s.score, s.status)
            }),
        ),
        row(
            "compare.disk_bytes_checked",
            Box::new(move |r| {
                Value::Text(if check(r, "disk-roundtrip").is_some() {
                    bytes(metric(r, "disk-roundtrip", "verified_bytes"))
                } else {
                    tx("compare.not_implemented")
                })
            }),
        ),
        row(
            "compare.disk_bytes_mismatched",
            Box::new(move |r| {
                Value::Text(text_or(
                    metric(r, "disk-roundtrip", "mismatched_bytes"),
                    "compare.not_measured_plural",
                ))
            }),
        ),
        row(
            "compare.storage_result",
            Box::new(|r| {
                let s = component_summary(&r.checks, "storage");
                Value::Score(s.score, s.status)
            }),
        ),
        row(
            "compare.cpu_calculations_verified",
            Box::new(|r| {
                Value::Text(
                    metric(r, "cpu-known-answers", "iterations")
                        .map_or_else(|| tx("compare.not_implemented"), |v| intl::plain("fr", v)),
                )
            }),
        ),
        row(
            "compare.duration_cpu_control",
            Box::new(move |r| {
                Value::Text(check(r, "cpu-known-answers").map_or_else(
                    || tx("compare.not_implemented"),
                    |c| intl::duration(locale, c.duration_ms),
                ))
            }),
        ),
        row(
            "compare.calculation_errors",
            Box::new(move |r| {
                Value::Text(text_or(
                    metric(r, "cpu-known-answers", "errors"),
                    "compare.not_measured",
                ))
            }),
        ),
        row(
            "compare.processor_result",
            Box::new(|r| {
                let s = component_summary(&r.checks, "cpu");
                Value::Score(s.score, s.status)
            }),
        ),
        Line::Section("compare.report_context"),
        row(
            "compare.total_duration",
            Box::new(move |r| Value::Text(intl::duration(locale, r.duration_ms))),
        ),
        row(
            "compare.usable_checks",
            Box::new(|r| Value::Text(format!("{} / {}", r.summary.checked, r.summary.total))),
        ),
        row(
            "common.check_score",
            Box::new(|r| Value::Score(r.summary.score, r.summary.status)),
        ),
    ]
}

const LABEL_GROW: f32 = 346.;
const COLUMN_GROW: f32 = 393.;

fn column(grow: f32) -> Div {
    div()
        .flex_grow()
        .flex_basis(relative(0.))
        .min_w_0()
        .map(move |mut d| {
            d.style().flex_grow = Some(grow);
            d
        })
}

pub fn render(app: &mut Computy, _window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    let locale = crate::i18n::locale();
    let selected: Vec<Report> = app
        .selected_ids
        .iter()
        .filter_map(|id| app.reports.iter().find(|r| &r.id == id).cloned())
        .collect();
    let title = components::section_title(
        Some("compare.before_after_eyebrow"),
        "compare.changes_side_side",
        Some("compare.compare_configurations_observed_results_without"),
        Some(
            button(
                "choose-reports",
                ButtonKind::Secondary,
                false,
                Some("plus"),
                tx("compare.choose_reports"),
                false,
            )
            .on_click(cx.listener(|this, _, window, cx| this.navigate(Page::History, window, cx)))
            .into_any_element(),
        ),
    );
    if selected.len() < 2 {
        return div()
            .flex()
            .flex_col()
            .child(title)
            .child(
                panel().mb(px(20.)).child(empty_sized(
                    "arrow-left-right",
                    tx("compare.two_moments_clearer_vision"),
                    Some(tx("compare.select_between_two_four_diagnostics").into_any_element()),
                    Some(
                        button_with(
                            "select-reports",
                            ButtonKind::Primary,
                            false,
                            Some(
                                icon("history", 16., "2", button_foreground(ButtonKind::Primary))
                                    .into_any_element(),
                            ),
                            tx("compare.select_diagnostics "),
                            false,
                        )
                        .child(icon("arrow-right", 16., "2", hex(0xffffff)))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.navigate(Page::History, window, cx)
                        }))
                        .into_any_element(),
                    ),
                    (75., 30.),
                    80.,
                    23.,
                )),
            )
            .into_any_element();
    }
    let border = c(theme::BORDER);
    let reference_bg = c((0xf8fbf2, 0x273923));
    let count = selected.len();
    let header = div()
        .flex()
        .child(
            column(LABEL_GROW)
                .pt(px(22.))
                .px(px(21.))
                .pb(px(23.))
                .border_r_1()
                .border_b_1()
                .border_color(border)
                .flex()
                .flex_col()
                .child(
                    spaced(
                        tx("compare.your_benchmarks_eyebrow"),
                        10.,
                        650.,
                        hex(0x899582),
                        1.75,
                        13.,
                    )
                    .mt(px(6.)),
                )
                .child(
                    spaced(
                        format!("{}{}", tn(count as f64), tx(" app.diagnostics_lowercase")),
                        14.,
                        650.,
                        c(theme::HEADING),
                        -0.2,
                        22.4,
                    )
                    .mt(px(14.)),
                )
                .child(
                    div()
                        .mt(px(6.))
                        .text_size(px(11.))
                        .line_height(px(18.15))
                        .text_color(hex(0xa5b596))
                        .child(tx("compare.same_questions"))
                        .child(tx("compare.different_times")),
                ),
        )
        .children(selected.iter().enumerate().map(|(index, report)| {
            let remove = report.id.clone();
            let promote = report.id.clone();
            column(COLUMN_GROW)
                .pt(px(22.))
                .px(px(21.))
                .pb(px(23.))
                .border_b_1()
                .when(index + 1 < count, |d| d.border_r_1())
                .border_color(border)
                .when(index == 0, |d| d.bg(reference_bg))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(20.))
                        .child(if index == 0 {
                            badge(Tone::Good, Some("pin"), tx("common.reference"))
                                .into_any_element()
                        } else {
                            badge(Tone::Muted, None, tx(&format!("Bilan {}", index + 1)))
                                .into_any_element()
                        })
                        .child(
                            div()
                                .id(SharedString::from(format!("remove-{remove}")))
                                .size(px(30.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(7.))
                                .cursor_pointer()
                                .child(icon("x", 14., "2", hex(0x6d7c70)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.selected_ids.retain(|id| *id != remove);
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    spaced(
                        report.label.clone(),
                        14.,
                        650.,
                        c(theme::HEADING),
                        -0.2,
                        22.4,
                    )
                    .mt(px(12.)),
                )
                .child(
                    text(
                        intl::date(locale, report.created_at as i64, true),
                        11.,
                        400.,
                        hex(0xa5b596),
                    )
                    .line_height(px(18.15))
                    .mt(px(6.)),
                )
                .child(text(tx(&report.inventory.host_name), 11., 400., hex(0xb0bda3)).mt(px(8.)))
                .when(index > 0, |d| {
                    d.child(
                        div().mt(px(14.)).flex().child(
                            div()
                                .id(SharedString::from(format!("promote-{promote}")))
                                .cursor_pointer()
                                .child(
                                    text(
                                        tx("compare.define_reference"),
                                        9.,
                                        600.,
                                        c(theme::TEXT_BUTTON),
                                    )
                                    .line_height(px(14.4)),
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.selected_ids.retain(|id| *id != promote);
                                    this.selected_ids.insert(0, promote.clone());
                                    cx.notify();
                                })),
                        ),
                    )
                })
        }));
    let body = rows().into_iter().map(|line| match line {
        Line::Section(label) => div()
            .py(px(13.))
            .px(px(21.))
            .bg(c((0xf0f5e8, 0x304329)))
            .border_b_1()
            .border_color(border)
            .child(spaced(tx(label), 11., 450., hex(0x8da473), 0.3, 14.))
            .into_any_element(),
        Line::Row(row) => {
            let base = row.numeric.map(|f| f(&selected[0]));
            div()
                .flex()
                .child(
                    column(LABEL_GROW)
                        .py(px(15.))
                        .px(px(21.))
                        .border_r_1()
                        .border_b_1()
                        .border_color(border)
                        .child(text(tx(row.label), 12., 450., hex(0x9bad89)).line_height(px(21.6))),
                )
                .children(selected.iter().enumerate().map(|(index, report)| {
                    let delta = match (row.numeric, base) {
                        (Some(f), Some(b)) => f(report) - b,
                        _ => 0.,
                    };
                    column(COLUMN_GROW)
                        .py(px(15.))
                        .px(px(21.))
                        .border_b_1()
                        .when(index + 1 < count, |d| d.border_r_1())
                        .border_color(border)
                        .when(index == 0, |d| d.bg(reference_bg))
                        .flex()
                        .flex_col()
                        .child(match (row.get)(report) {
                            Value::Text(value) => text(tx(&value), 12., 400., hex(0x7d9865))
                                .line_height(px(21.6))
                                .into_any_element(),
                            Value::Score(value, status) => {
                                score(value, status, false).into_any_element()
                            }
                        })
                        .when(index > 0 && delta != 0., |d| {
                            d.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(icon(
                                        if delta > 0. { "arrow-up" } else { "arrow-down" },
                                        11.,
                                        "2",
                                        c(theme::MUTED),
                                    ))
                                    .child(text(
                                        format!(
                                            "{}{}",
                                            intl::bytes(locale, Some(delta.abs()), 1),
                                            tx(" compare.against_reference")
                                        ),
                                        10.,
                                        400.,
                                        c(theme::MUTED),
                                    )),
                            )
                        })
                }))
                .into_any_element()
        }
    });
    let footer = div()
        .flex()
        .child(
            column(LABEL_GROW)
                .py(px(20.))
                .px(px(21.))
                .border_r_1()
                .border_color(border)
                .child(text(tx("compare.view_evidence"), 12., 450., hex(0xa0b38d))),
        )
        .children(selected.iter().enumerate().map(|(index, report)| {
            let open = report.clone();
            column(COLUMN_GROW)
                .py(px(20.))
                .px(px(21.))
                .when(index + 1 < count, |d| d.border_r_1())
                .border_color(border)
                .flex()
                .child(
                    text_button(
                        SharedString::from(format!("open-{}", report.id)),
                        tx("compare.open_report "),
                        Some(("arrow-right", 14.)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.report = Some(open.clone());
                        cx.notify();
                    })),
                )
        }));
    div()
        .flex()
        .flex_col()
        .child(title)
        .children(warnings(&selected).into_iter().map(|w| {
            notice(NoticeTone::Warn, vec![tx(w).into_any_element()])
                .text_size(px(12.))
                .line_height(px(21.))
                .mb(px(18.))
                .into_any_element()
        }))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(15.))
                .mb(px(18.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .child(div().size(px(6.)).rounded_full().bg(hex(0xa0b981)))
                        .child(text(
                            tx("compare.first_report_reference"),
                            11.,
                            400.,
                            hex(0xa0b18a),
                        )),
                )
                .child(
                    text(
                        tx("compare.gap_capacity_performance_not_proof"),
                        11.,
                        400.,
                        hex(0xa0b18a),
                    )
                    .line_height(px(18.15)),
                ),
        )
        .child(
            panel()
                .mb(px(20.))
                .flex()
                .flex_col()
                .child(header)
                .children(body)
                .child(footer),
        )
        .child(
            notice(
                NoticeTone::Info,
                vec![tx("compare.compare_performance_keep_same_paracounters").into_any_element()],
            )
            .text_size(px(12.))
            .line_height(px(21.)),
        )
        .into_any_element()
}
