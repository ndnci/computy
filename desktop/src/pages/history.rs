//! Report history (former pages/History.tsx).
use crate::{
    app::{Computy, Page},
    components::{
        self, ButtonKind, NoticeTone, button, button_foreground, button_with, empty, notice,
        spinner, status_badge, text_button,
    },
    forms::{option, select, text_input},
    i18n::{tn, tx},
    intl,
    model::Report,
    theme::{self, c, hex},
    ui::{icon, one_line, panel, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::input::{InputEvent, InputState};
use serde_json::{Value, json};

pub struct HistoryState {
    pub query: Entity<InputState>,
    pub filter: String,
    pub rename: Option<Report>,
    pub name: Entity<InputState>,
    pub deleting: Option<Report>,
    pub busy: bool,
    _subscriptions: Vec<Subscription>,
}

impl HistoryState {
    pub fn new(window: &mut Window, cx: &mut Context<Computy>) -> Self {
        let query =
            cx.new(|cx| InputState::new(window, cx).placeholder(tx("history.search_diagnostics")));
        let name = cx.new(|cx| InputState::new(window, cx));
        let subscriptions = vec![
            cx.subscribe(&query, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe(&name, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
        ];
        Self {
            query,
            filter: "all".into(),
            rename: None,
            name,
            deleting: None,
            busy: false,
            _subscriptions: subscriptions,
        }
    }
}

fn state(app: &mut Computy) -> &mut HistoryState {
    app.pages.history.as_mut().expect("history state")
}

fn stat(icon_name: &str, value: String, label: &str, padding: (f32, f32), border: bool) -> Div {
    div()
        .flex_grow()
        .flex_basis(relative(0.))
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(13.))
        .pl(px(padding.0))
        .pr(px(padding.1))
        .when(border, |d| d.border_r_1().border_color(c(theme::BORDER)))
        .child(
            div()
                .size(px(39.))
                .flex_none()
                .rounded(px(10.))
                .bg(c((0xf1f6e9, 0x304129)))
                .flex()
                .items_center()
                .justify_center()
                .child(icon(icon_name, 20., "2", c((0x94ad76, 0xa4c580)))),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(7.))
                .child(text(value, 13., 550., c(theme::TEXT)))
                .child(text(tx(label), 9., 400., hex(0xa7b399))),
        )
}

const COLUMNS: [f32; 5] = [283., 199., 273., 152., 175.];

fn cell(index: usize) -> Div {
    div()
        .flex_grow()
        .flex_basis(relative(0.))
        .map(|mut d| {
            d.style().flex_grow = Some(COLUMNS[index]);
            d
        })
        .min_w_0()
        .py(px(17.))
        .pl(px(12.))
        .pr(px(if index == 4 { 16. } else { 12. }))
}

fn icon_button(id: SharedString, name: &str, size: f32, disabled: bool) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(26.))
        .h(px(27.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(7.))
        .when(disabled, |d| d.opacity(0.5))
        .when(!disabled, |d| {
            d.cursor_pointer().hover(|s| s.bg(c((0xf7f9f6, 0x253324))))
        })
        .child(icon(name, size, "2", hex(0xa8b797)))
}

fn row(app: &Computy, report: &Report, last: bool, cx: &mut Context<Computy>) -> impl IntoElement {
    let locale = crate::i18n::locale();
    let selected = app.selected_ids.contains(&report.id);
    let busy = app.launching || app.job.as_ref().is_some_and(|j| j.running);
    let mut kinds: Vec<&str> = vec![];
    for module in &report.inventory.memory.modules {
        if !kinds.contains(&module.kind.as_str()) {
            kinds.push(&module.kind);
        }
    }
    let detail = if kinds.is_empty() {
        report.inventory.os.clone()
    } else {
        kinds.join(" / ")
    };
    let id = report.id.clone();
    let (open, export, relaunch, rename, delete, toggle, open2) = (
        report.clone(),
        report.clone(),
        report.clone(),
        report.clone(),
        report.clone(),
        id.clone(),
        report.clone(),
    );
    div()
        .id(SharedString::from(format!("row-{id}")))
        .flex()
        .items_center()
        .cursor_pointer()
        .when(!last, |d| d.border_b_1().border_color(c(theme::BORDER)))
        .when(selected, |d| d.bg(c((0xf6faf0, 0x253322))))
        .hover(|s| s.bg(c((0xfbfcf8, 0x253322))))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.report = Some(open.clone());
            cx.notify();
        }))
        .child(
            div()
                .w(px(50.))
                .flex_none()
                .py(px(17.))
                .pl(px(22.))
                .pr(px(12.))
                .child(
                    div()
                        .id(SharedString::from(format!("select-{id}")))
                        .size(px(16.))
                        .rounded(px(4.))
                        .border_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .map(|d| {
                            if selected {
                                d.bg(hex(0x78975c)).border_color(hex(0x78975c)).child(icon(
                                    "check",
                                    12.,
                                    "2",
                                    hex(0xffffff),
                                ))
                            } else {
                                d.bg(c(theme::PANEL)).border_color(c((0xdce5cf, 0x6d8954)))
                            }
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_selection(&toggle, cx);
                        })),
                ),
        )
        .child(
            cell(0).child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(11.))
                    .child(
                        div()
                            .size(px(33.))
                            .flex_none()
                            .rounded(px(9.))
                            .bg(c((0xf1f5ea, 0x304129)))
                            .border_1()
                            .border_color(c((0xe8eee0, 0x405333)))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("activity", 18., "2", c((0x8da270, 0xa4c580)))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(7.))
                            .child(one_line(report.label.clone(), 12., 550., c(theme::TEXT)))
                            .child(one_line(
                                format!(
                                    "{}{}{}",
                                    tx(&report.inventory.host_name),
                                    tx(" · "),
                                    tx(&detail)
                                ),
                                10.,
                                400.,
                                hex(0xa6b497),
                            )),
                    ),
            ),
        )
        .child(
            cell(1)
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(text(
                    intl::date(locale, report.created_at as i64, true),
                    12.,
                    400.,
                    hex(0x8b9e76),
                ))
                .child(text(
                    format!(
                        "{}{}{}{}{}{}",
                        intl::duration(locale, report.duration_ms),
                        tx(" · "),
                        tn(report.summary.checked as f64),
                        tx("/"),
                        tn(report.summary.total as f64),
                        tx(" common.usable_checks")
                    ),
                    10.,
                    400.,
                    hex(0xb0bea0),
                )),
        )
        .child(cell(2).child(status_badge(
            report.summary.status,
            true,
            Some(&report.checks),
            report.completed,
        )))
        .child(
            cell(3).child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(components::mood(
                        components::score_status(report.summary.score, report.summary.status),
                        22.,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .child(text(
                                report.summary.score.map_or("—".into(), tn),
                                14.,
                                550.,
                                c(theme::HEADING),
                            ))
                            .child(text(tx("/ 10"), 9., 400., hex(0x9ca78e)).ml(px(3.))),
                    ),
            ),
        )
        .child(
            cell(4).child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(1.))
                    .child(
                        icon_button(format!("relaunch-{id}").into(), "rotate-ccw", 16., busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                if !busy {
                                    this.relaunch(relaunch.clone(), window, cx);
                                }
                            })),
                    )
                    .child(
                        icon_button(
                            format!("export-{id}").into(),
                            "arrow-down-to-line",
                            16.,
                            false,
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.export_report(&export, window, cx);
                            },
                        )),
                    )
                    .child(
                        icon_button(format!("rename-{id}").into(), "pencil", 15., false).on_click(
                            cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                let label = rename.label.clone();
                                let s = state(this);
                                s.rename = Some(rename.clone());
                                s.name
                                    .update(cx, |input, cx| input.set_value(label, window, cx));
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        icon_button(format!("delete-{id}").into(), "trash-2", 15., false).on_click(
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                state(this).deleting = Some(delete.clone());
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        icon_button(format!("open-{id}").into(), "chevron-right", 17., false)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.report = Some(open2.clone());
                                cx.notify();
                            })),
                    ),
            ),
        )
}

pub fn render(app: &mut Computy, window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    if app.pages.history.is_none() {
        let state = HistoryState::new(window, cx);
        app.pages.history = Some(state);
    }
    let locale = crate::i18n::locale();
    let s = app.pages.history.as_ref().unwrap();
    let query = s.query.read(cx).value().to_lowercase();
    let filter = s.filter.clone();
    let filtered: Vec<Report> = app
        .reports
        .iter()
        .filter(|r| {
            (query.is_empty()
                || format!(
                    "{} {} {}",
                    r.label,
                    r.inventory.host_name,
                    intl::date(locale, r.created_at as i64, false)
                )
                .to_lowercase()
                .contains(&query))
                && (filter == "all"
                    || serde_json::to_value(r.summary.status)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .as_deref()
                        == Some(&filter))
        })
        .cloned()
        .collect();
    let count = filtered.len();
    let header_cell = |index: Option<usize>, label: &str| {
        let base = match index {
            Some(i) => div()
                .flex_grow()
                .flex_basis(relative(0.))
                .map(move |mut d| {
                    d.style().flex_grow = Some(COLUMNS[i]);
                    d
                })
                .min_w_0()
                .pl(px(12.))
                .pr(px(if i == 4 { 16. } else { 12. })),
            None => div().w(px(50.)).flex_none(),
        };
        base.py(px(15.)).child(spaced(
            tx(label).to_uppercase(),
            10.,
            500.,
            hex(0xa6b299),
            0.5,
            13.,
        ))
    };
    let table: AnyElement = if app.reports.is_empty() {
        empty(
            "history",
            tx("history.first_chapter_remains_written"),
            Some(tx("history.start_diagnostic_keep_snapshot_hardware").into_any_element()),
            Some(
                button(
                    "first-report",
                    ButtonKind::Primary,
                    false,
                    Some("activity"),
                    tx("history.create_my_first_review"),
                    false,
                )
                .on_click(
                    cx.listener(|this, _, window, cx| this.navigate(Page::Diagnostics, window, cx)),
                )
                .into_any_element(),
            ),
        )
        .into_any_element()
    } else if filtered.is_empty() {
        empty(
            "search",
            tx("history.no_diagnostics_match"),
            Some(tx("history.try_another_name_date_filter").into_any_element()),
            None,
        )
        .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .bg(c((0xfbfcf8, 0x253322)))
                    .border_b_1()
                    .border_color(c(theme::BORDER))
                    .child(header_cell(None, ""))
                    .child(header_cell(Some(0), "history.diagnostic"))
                    .child(header_cell(Some(1), "history.date_duration"))
                    .child(header_cell(Some(2), "history.result"))
                    .child(header_cell(Some(3), "common.check_score"))
                    .child(header_cell(Some(4), "")),
            )
            .children(
                filtered
                    .iter()
                    .enumerate()
                    .map(|(i, report)| row(app, report, i + 1 == count, cx).into_any_element()),
            )
            .into_any_element()
    };
    let selected = app.selected_ids.len();
    let query_entity = s.query.clone();
    let has_query = !query.is_empty();
    div()
        .flex()
        .flex_col()
        .child(components::section_title(
            Some("history.benchmarks_eyebrow"),
            "history.hardware_has_history",
            Some("history.find_each_report_configuration_what"),
            Some(
                button(
                    "new-diagnostic",
                    ButtonKind::Primary,
                    false,
                    Some("plus"),
                    tx("common.new_diagnostic"),
                    false,
                )
                .on_click(
                    cx.listener(|this, _, window, cx| this.navigate(Page::Diagnostics, window, cx)),
                )
                .into_any_element(),
            ),
        ))
        .when(app.unreadable > 0, |d| {
            d.child(
                notice(
                    NoticeTone::Warn,
                    vec![
                        format!(
                            "{}{}",
                            tn(app.unreadable as f64),
                            tx(" history.unreadable_altered_file_have_been")
                        )
                        .into_any_element(),
                    ],
                )
                .mb(px(20.)),
            )
        })
        .child(
            div()
                .flex()
                .p(px(22.))
                .mb(px(23.))
                .bg(c(theme::PANEL))
                .border_1()
                .border_color(c(theme::BORDER))
                .rounded(px(13.))
                .child(
                    stat(
                        "history",
                        tn(app.reports.len() as f64),
                        "history.registered_diagnostics",
                        (0., 20.),
                        true,
                    )
                    .map(|mut d| {
                        d.style().flex_grow = Some(330.);
                        d
                    }),
                )
                .child(
                    stat(
                        "clock-3",
                        app.reports.first().map_or_else(
                            || tx("history.first_review_waiting"),
                            |r| intl::date(locale, r.created_at as i64, true),
                        ),
                        "history.last_diagnostic",
                        (20., 20.),
                        true,
                    )
                    .map(|mut d| {
                        d.style().flex_grow = Some(429.);
                        d
                    }),
                )
                .child(
                    stat(
                        "arrow-left-right",
                        tx("history.up_4_reports"),
                        "history.compared_side_side",
                        (20., 0.),
                        false,
                    )
                    .map(|mut d| {
                        d.style().flex_grow = Some(330.);
                        d
                    }),
                ),
        )
        .child(
            panel()
                .mb(px(20.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(15.))
                        .py(px(18.))
                        .px(px(22.))
                        .border_b_1()
                        .border_color(c(theme::BORDER))
                        .child(
                            div()
                                .w(px(350.))
                                .h(px(40.))
                                .flex()
                                .items_center()
                                .gap(px(9.))
                                .px(px(11.))
                                .bg(c((0xf7f9f6, 0x202d23)))
                                .border_1()
                                .border_color(c((0xe6ecde, 0x425435)))
                                .rounded(px(7.))
                                .text_size(px(13.))
                                .child(icon("search", 17., "2", c((0xb0bd9f, 0xb1c699))))
                                .child(
                                    div().flex_1().child(
                                        gpui_component::input::Input::new(&query_entity)
                                            .appearance(false),
                                    ),
                                )
                                .when(has_query, |d| {
                                    d.child(
                                        div()
                                            .id("clear-search")
                                            .cursor_pointer()
                                            .child(icon("x", 14., "2", c(theme::MUTED)))
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                state(this).query.update(cx, |input, cx| {
                                                    input.set_value("", window, cx)
                                                });
                                                cx.notify();
                                            })),
                                    )
                                }),
                        )
                        .child(div().w(px(177.)).flex().items_center().child(select(
                            "history-filter",
                            &filter,
                            vec![
                                option("all", tx("history.all_results")),
                                option("passed", tx("history.passed_tests")),
                                option("warning", tx("common.needs_review")),
                                option("failed", tx("history.anomalies_detected")),
                                option("unavailable", tx("history.unverified")),
                                option("cancelled", tx("history.interrupted")),
                            ],
                            app,
                            cx,
                            |this, value, _, _| state(this).filter = value,
                        )))
                        .child(div().flex_1())
                        .child(text(
                            format!("{} : {}", tx("history.results"), tn(count as f64)),
                            10.,
                            400.,
                            hex(0xa5b097),
                        )),
                )
                .child(table),
        )
        .child(if selected > 0 {
            div()
                .flex()
                .items_center()
                .gap(px(14.))
                .py(px(17.))
                .px(px(22.))
                .mb(px(20.))
                .rounded(px(11.))
                .bg(c((0xeaf2df, 0x304129)))
                .border_1()
                .border_color(c((0xd8e6c7, 0x405333)))
                .child(
                    div()
                        .size(px(32.))
                        .rounded(px(9.))
                        .bg(hex(0xdbe9cb))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(text(tn(selected as f64), 13., 600., hex(0x799d54))),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(text(
                            tx("history.selected_diagnostics"),
                            11.,
                            550.,
                            c(theme::TEXT),
                        ))
                        .child(
                            text(
                                tx(if selected < 2 {
                                    "history.select_second_report_compare"
                                } else {
                                    "history.first_report_selected_reference"
                                }),
                                9.,
                                400.,
                                hex(0x9caf86),
                            )
                            .mt(px(4.)),
                        ),
                )
                .child(div().flex_1())
                .child(
                    text_button("unselect", tx("history.deselect_all"), None).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.selected_ids.clear();
                            cx.notify();
                        }),
                    ),
                )
                .child(
                    button_with(
                        "compare-selected",
                        ButtonKind::Primary,
                        false,
                        Some(
                            icon(
                                "arrow-left-right",
                                16.,
                                "2",
                                button_foreground(ButtonKind::Primary),
                            )
                            .into_any_element(),
                        ),
                        tx("history.compare_reports "),
                        selected < 2,
                    )
                    .child(icon("arrow-right", 16., "2", hex(0xffffff)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if selected >= 2 {
                            this.navigate(Page::Compare, window, cx);
                        }
                    })),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .px(px(5.))
                .child(icon("arrow-left-right", 17., "2", hex(0xa1b38d)))
                .child(
                    text(
                        tx("history.change_ram_select_reports_before"),
                        11.,
                        400.,
                        hex(0xa1b38d),
                    )
                    .line_height(px(18.15)),
                )
                .into_any_element()
        })
        .into_any_element()
}

/// Rename and delete confirmations.
pub fn modal(
    app: &mut Computy,
    window: &mut Window,
    cx: &mut Context<Computy>,
) -> Option<AnyElement> {
    let s = app.pages.history.as_ref()?;
    let busy = s.busy;
    if let Some(report) = s.rename.clone() {
        let name = s.name.read(cx).value().trim().to_owned();
        let close = std::rc::Rc::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
            if !busy {
                state(this).rename = None;
                cx.notify();
            }
        }));
        let body = div()
            .flex()
            .flex_col()
            .child(text(tx("history.diagnostic_name"), 12., 400., hex(0x8d9e79)).mb(px(10.)))
            .child(text_input(&s.name))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .items_center()
                    .gap(px(10.))
                    .mt(px(22.))
                    .child(
                        button(
                            "rename-cancel",
                            ButtonKind::Secondary,
                            false,
                            None,
                            tx("common.cancel"),
                            busy,
                        )
                        .on_click({
                            let close = close.clone();
                            move |e, w, cx| close(e, w, cx)
                        }),
                    )
                    .child(
                        button_with(
                            "rename-save",
                            ButtonKind::Primary,
                            false,
                            busy.then(|| spinner(15., hex(0xffffff))),
                            tx("history.save"),
                            busy || name.is_empty(),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if busy || name.is_empty() {
                                return;
                            }
                            state(this).busy = true;
                            let id = report.id.clone();
                            this.call(
                                "rename",
                                json!({"id": id, "label": name.clone()}),
                                cx,
                                |this, result: Result<Value, String>, cx| {
                                    state(this).busy = false;
                                    match result {
                                        Ok(_) => {
                                            this.reload_history(cx);
                                            state(this).rename = None;
                                            this.notify(
                                                tx("history.diagnostic_renamed"),
                                                false,
                                                cx,
                                            );
                                        }
                                        Err(error) => this.notify(error, true, cx),
                                    }
                                },
                            );
                        })),
                    ),
            )
            .into_any_element();
        return Some(components::modal(
            "rename",
            tx("history.rename_diagnostic"),
            false,
            body,
            close,
            window,
        ));
    }
    if let Some(report) = s.deleting.clone() {
        let close = std::rc::Rc::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
            if !busy {
                state(this).deleting = None;
                cx.notify();
            }
        }));
        let body = div()
            .flex()
            .flex_col()
            .child(
                text(
                    format!(
                        "{}{}{}",
                        tx("history.report_quote_open "),
                        report.label,
                        tx(" history.deleted_computer_export_before_deleting")
                    ),
                    12.,
                    400.,
                    hex(0x95a981),
                )
                .line_height(px(22.8)),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .items_center()
                    .gap(px(10.))
                    .mt(px(22.))
                    .child(
                        button(
                            "delete-keep",
                            ButtonKind::Secondary,
                            false,
                            None,
                            tx("history.keep"),
                            busy,
                        )
                        .on_click({
                            let close = close.clone();
                            move |e, w, cx| close(e, w, cx)
                        }),
                    )
                    .child(
                        button_with(
                            "delete-confirm",
                            ButtonKind::Danger,
                            false,
                            Some(if busy {
                                spinner(15., hex(0xffffff))
                            } else {
                                icon("trash-2", 15., "2", hex(0xffffff)).into_any_element()
                            }),
                            tx("history.delete_report"),
                            busy,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if busy {
                                return;
                            }
                            state(this).busy = true;
                            let id = report.id.clone();
                            this.call(
                                "delete",
                                json!({"id": id.clone()}),
                                cx,
                                move |this, result: Result<Value, String>, cx| {
                                    state(this).busy = false;
                                    match result {
                                        Ok(_) => {
                                            this.selected_ids.retain(|v| *v != id);
                                            this.reload_history(cx);
                                            state(this).deleting = None;
                                            this.notify(
                                                tx("history.diagnostic_deleted_device"),
                                                false,
                                                cx,
                                            );
                                        }
                                        Err(error) => this.notify(error, true, cx),
                                    }
                                },
                            );
                        })),
                    ),
            )
            .into_any_element();
        return Some(components::modal(
            "delete",
            tx("history.delete_diagnostic_question"),
            false,
            body,
            close,
            window,
        ));
    }
    None
}
