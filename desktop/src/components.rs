//! Shared interface components (former components.tsx).
use crate::{
    app::Computy,
    i18n::{tn, tp, tx},
    intl,
    model::*,
    theme::{self, Pair, c, hex},
    ui::{icon, shadow, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use std::time::Duration;

pub fn logo(size: f32) -> impl IntoElement {
    img(if size > 60. {
        "brand/logo-300.png"
    } else {
        "brand/logo-96.png"
    })
    .size(px(size))
    .flex_none()
}

pub fn spinner(size: f32, color: Hsla) -> AnyElement {
    let base = icon("loader-circle", size, "2", color);
    if crate::capture_mode() {
        return base.into_any_element();
    }
    base.with_animation(
        "spin",
        Animation::new(Duration::from_millis(1100)).repeat(),
        |svg, delta| svg.with_transformation(Transformation::rotate(percentage(delta))),
    )
    .into_any_element()
}

pub fn progress_bar(value: f64, height: f32, track: Hsla, fill: Hsla) -> impl IntoElement {
    let value = value.clamp(0., 1.) as f32;
    div()
        .w_full()
        .min_w(px(70.))
        .h(px(height))
        .rounded(px(10.))
        .bg(track)
        .overflow_hidden()
        .child(div().h_full().w(relative(value)).rounded(px(10.)).bg(fill))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Good,
    Warn,
    Bad,
    Muted,
}

pub struct StatusInfo {
    pub label: &'static str,
    pub short: &'static str,
    pub tone: Tone,
}

pub fn status_info(status: Status) -> StatusInfo {
    let (label, short, tone) = match status {
        Status::Passed => ("common.test_passed", "common.passed", Tone::Good),
        Status::Warning => ("common.needs_review", "common.needs_review", Tone::Warn),
        Status::Failed => ("common.issue_detected", "common.issue", Tone::Bad),
        Status::Cancelled => ("common.interrupted", "common.interrupted", Tone::Muted),
        Status::Unavailable => ("common.not_verified", "common.not_verified", Tone::Muted),
    };
    StatusInfo { label, short, tone }
}

/// A score describes executed checks; missing telemetry is coverage.
pub fn score_status(score: Option<f64>, status: Status) -> Status {
    if matches!(
        status,
        Status::Failed | Status::Cancelled | Status::Unavailable
    ) {
        return status;
    }
    match score {
        Some(10.) => Status::Passed,
        Some(0.) => Status::Failed,
        _ => status,
    }
}

pub struct Outcome {
    pub status: Status,
    pub unavailable: Vec<String>,
}

pub fn check_outcome(checks: &[Check], completed: bool) -> Outcome {
    let has = |s: Status| checks.iter().any(|c| c.status == s);
    let status = if has(Status::Failed) {
        Status::Failed
    } else if !completed || has(Status::Cancelled) {
        Status::Cancelled
    } else if has(Status::Warning) {
        Status::Warning
    } else if has(Status::Passed) {
        Status::Passed
    } else {
        Status::Unavailable
    };
    Outcome {
        status,
        unavailable: checks
            .iter()
            .filter(|c| c.status == Status::Unavailable)
            .map(|c| c.name.clone())
            .collect(),
    }
}

pub struct ComponentSummary {
    pub score: Option<f64>,
    pub status: Status,
}

pub fn component_summary(checks: &[Check], component: &str) -> ComponentSummary {
    let relevant: Vec<&Check> = checks.iter().filter(|c| c.component == component).collect();
    let known: Vec<&&Check> = relevant
        .iter()
        .filter(|c| matches!(c.status, Status::Passed | Status::Warning | Status::Failed))
        .collect();
    let score = known
        .iter()
        .map(|c| match c.status {
            Status::Passed => 10.,
            Status::Warning => 5.,
            _ => 0.,
        })
        .reduce(f64::min);
    let status = match score {
        Some(0.) => Status::Failed,
        None => Status::Unavailable,
        Some(s) if s == 5. || known.len() < relevant.len() => Status::Warning,
        _ => Status::Passed,
    };
    ComponentSummary { score, status }
}

pub fn tone_colors(tone: Tone) -> (Pair, Pair) {
    match tone {
        Tone::Good => (theme::GOOD, theme::GOOD_BG),
        Tone::Warn => (theme::WARN, theme::WARN_BG),
        Tone::Bad => (theme::BAD, theme::BAD_BG),
        Tone::Muted => (theme::MUTED_BADGE, theme::MUTED_BADGE_BG),
    }
}

pub fn badge(tone: Tone, icon_name: Option<&str>, label: impl Into<SharedString>) -> Div {
    let (fg, bg) = tone_colors(tone);
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(px(4.))
        .px(px(if crate::layout::compact() { 4. } else { 6. }))
        .py(px(if crate::layout::compact() { 3. } else { 4. }))
        .rounded(px(5.))
        .bg(c(bg))
        .when_some(icon_name, |d, name| d.child(icon(name, 12., "2", c(fg))))
        .child(
            text(
                label,
                if crate::layout::compact() { 9. } else { 10. },
                550.,
                c(fg),
            )
            .line_height(px(if crate::layout::compact() { 12.6 } else { 14. })),
        )
}

pub fn neutral_badge(label: impl Into<SharedString>) -> Div {
    badge(Tone::Muted, None, label)
}

pub fn status_badge(
    status: Status,
    compact: bool,
    checks: Option<&[Check]>,
    completed: bool,
) -> impl IntoElement {
    let outcome = checks.map(|checks| check_outcome(checks, completed));
    let status = outcome.as_ref().map_or(status, |o| o.status);
    let info = status_info(status);
    let icon_name = match status {
        Status::Passed => "check",
        Status::Failed | Status::Warning => "triangle-alert",
        _ => "minus",
    };
    let label = if outcome.is_some() && status == Status::Passed {
        "common.no_errors_detected"
    } else if compact {
        info.short
    } else {
        info.label
    };
    let missing = outcome.map_or(0, |o| o.unavailable.len());
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .child(badge(info.tone, Some(icon_name), tx(label)))
        .when(missing > 0, |d| {
            d.child(badge(
                Tone::Muted,
                None,
                tp("common.unverified_count", &[("count", missing.into())]),
            ))
        })
}

pub fn mood(status: Status, size: f32) -> impl IntoElement {
    let name = match status {
        Status::Passed => "smile",
        Status::Failed => "frown",
        _ => "meh",
    };
    let color = match status_info(status).tone {
        Tone::Good => hex(0x83a65d),
        Tone::Warn => hex(0xb39252),
        Tone::Bad => hex(0xbf7367),
        Tone::Muted => c((0xa7b294, 0x889e74)),
    };
    icon(name, size, "1.7", color)
}

pub fn score(score: Option<f64>, status: Status, large: bool) -> impl IntoElement {
    let value = score.map_or_else(|| "—".to_owned(), tn);
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(if large { 13. } else { 8. }))
        .child(mood(
            score_status(score, status),
            if large { 38. } else { 22. },
        ))
        .child(
            div()
                .flex()
                .items_baseline()
                .child(if large {
                    spaced(value, 35., 550., c(theme::HEADING), -1., 46.).into_any_element()
                } else {
                    text(value, 16., 550., c(theme::HEADING))
                        .line_height(px(21.))
                        .into_any_element()
                })
                .child(
                    text(
                        tx("/ 10"),
                        if large { 13. } else { 9. },
                        400.,
                        hex(0x9ca78e),
                    )
                    .ml(px(3.)),
                ),
        )
}

pub fn empty(
    icon_name: &str,
    title: impl Into<SharedString>,
    body: Option<AnyElement>,
    action: Option<AnyElement>,
) -> Div {
    empty_sized(icon_name, title, body, action, (44., 24.), 57., 16.)
}

/// Empty state with explicit padding and icon box (compare page variant).
pub fn empty_sized(
    icon_name: &str,
    title: impl Into<SharedString>,
    body: Option<AnyElement>,
    action: Option<AnyElement>,
    padding: (f32, f32),
    icon_box: f32,
    radius: f32,
) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .py(px(padding.0))
        .px(px(padding.1))
        .child(
            div()
                .size(px(icon_box))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(radius))
                .bg(c((0xf2f5eb, 0x304129)))
                .border_1()
                .border_color(c((0xe8edde, 0x405333)))
                .mb(px(18.))
                .child(icon(icon_name, 27., "1.5", c((0xa4b68e, 0xa4c580)))),
        )
        .child(spaced(title.into(), 16., 650., c(theme::HEADING), -0.5, 21.).mb(px(9.)))
        .children(body.map(|body| {
            div()
                .max_w(px(450.))
                .text_center()
                .text_size(px(12.))
                .line_height(px(21.6))
                .text_color(c(theme::MUTED))
                .child(body)
        }))
        .children(action.map(|a| div().mt(px(22.)).child(a)))
}

#[derive(Clone, Copy)]
pub enum NoticeTone {
    Info,
    Warn,
}

pub fn notice(tone: NoticeTone, children: Vec<AnyElement>) -> Div {
    let (bg, border, color, icon_name) = match tone {
        NoticeTone::Info => (
            c((0xf0f5eb, 0x304129)),
            c((0xe2ebd8, 0x405333)),
            c((0x748666, 0xa4c580)),
            "info",
        ),
        NoticeTone::Warn => (
            c(theme::WARN_BG),
            c((0xeee0c0, 0x54482b)),
            c((0x9c8454, 0xadc29a)),
            "triangle-alert",
        ),
    };
    div()
        .flex()
        .items_start()
        .gap(px(10.))
        .py(px(14.))
        .px(px(16.))
        .bg(bg)
        .border_1()
        .border_color(border)
        .rounded(px(9.))
        .text_size(px(12.))
        .line_height(px(21.))
        .text_color(color)
        .child(icon(icon_name, 17., "2", color).mt(px(1.)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .children(children),
        )
}

pub fn notice_bold(tone: NoticeTone) -> Hsla {
    match tone {
        NoticeTone::Info => hex(0x6b8058),
        NoticeTone::Warn => hex(0x94763e),
    }
}

/// Inline text with bold runs, as in `<p>text <b>bold</b> text</p>`.
pub fn rich(parts: &[(&str, bool)], bold: Hsla) -> StyledText {
    let mut content = String::new();
    let mut highlights = Vec::new();
    for (part, strong) in parts {
        let start = content.len();
        content.push_str(part);
        if *strong {
            highlights.push((
                start..content.len(),
                HighlightStyle {
                    font_weight: Some(FontWeight(600.)),
                    color: Some(bold),
                    ..Default::default()
                },
            ));
        }
    }
    StyledText::new(content).with_highlights(highlights)
}

pub fn section_title(
    eyebrow: Option<&str>,
    title: &str,
    description: Option<&str>,
    action: Option<AnyElement>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(20.))
        .mb(px(27.))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .min_w_0()
                .when_some(eyebrow, |d, e| {
                    d.child(div().mb(px(10.)).child(spaced(
                        tx(e),
                        10.,
                        650.,
                        hex(0x899582),
                        1.75,
                        16.5,
                    )))
                })
                .child(spaced(
                    tx(title),
                    if crate::layout::narrow() { 28. } else { 32. },
                    650.,
                    c(theme::HEADING),
                    -1.2,
                    if crate::layout::narrow() { 36.4 } else { 41.6 },
                ))
                .when_some(description, |d, text_value| {
                    d.child(
                        text(
                            tx(text_value),
                            if crate::layout::narrow() { 12. } else { 13. },
                            400.,
                            hex(0x6d7c70),
                        )
                        .line_height(px(if crate::layout::narrow() { 19.8 } else { 21.45 }))
                        .mt(px(8.)),
                    )
                }),
        )
        .children(action)
}

pub fn panel_header(title: &str, subtitle: Option<String>, action: Option<AnyElement>) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.))
        .pt(px(23.))
        .px(px(23.))
        .pb(px(20.))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .min_w_0()
                .child(spaced(tx(title), 16., 650., c(theme::HEADING), -0.4, 21.))
                .when_some(subtitle, |d, s| {
                    d.child(
                        text(s, 12., 400., c(theme::SOFT_TEXT))
                            .line_height(px(19.2))
                            .mt(px(6.)),
                    )
                }),
        )
        .children(action)
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
}

pub fn button(
    id: impl Into<ElementId>,
    kind: ButtonKind,
    small: bool,
    icon_name: Option<&str>,
    label: impl Into<SharedString>,
    disabled: bool,
) -> Stateful<Div> {
    let fg = button_foreground(kind);
    let size = if small { 15. } else { 17. };
    button_with(
        id,
        kind,
        small,
        icon_name.map(|name| icon(name, size, "2", fg).into_any_element()),
        label,
        disabled,
    )
}

pub fn button_foreground(kind: ButtonKind) -> Hsla {
    match kind {
        ButtonKind::Secondary => c((0x65715f, 0xb1c699)),
        _ => hex(0xffffff),
    }
}

/// Button with an arbitrary leading element (spinner, filled icon…).
pub fn button_with(
    id: impl Into<ElementId>,
    kind: ButtonKind,
    small: bool,
    lead: Option<AnyElement>,
    label: impl Into<SharedString>,
    disabled: bool,
) -> Stateful<Div> {
    let (bg, border, fg) = match kind {
        ButtonKind::Primary => (c(theme::GREEN), c(theme::GREEN), hex(0xffffff)),
        ButtonKind::Secondary => (
            c(theme::PANEL),
            c((0xe1e6dd, 0x425435)),
            c((0x65715f, 0xb1c699)),
        ),
        ButtonKind::Danger => (hex(0xae5149), hex(0xae5149), hex(0xffffff)),
    };
    let hover = match kind {
        ButtonKind::Primary => c((0x193d2e, 0x75945a)),
        ButtonKind::Secondary => c((0xf1f5eb, 0x35492c)),
        ButtonKind::Danger => hex(0x9b463f),
    };
    let (size, line, py, px_, gap) = if small {
        (11., 13.2, 8., 11., 7.)
    } else {
        (12., 14.4, 12., 17., 9.)
    };
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(px(gap))
        .py(px(py))
        .px(px(px_))
        .when(small, |d| d.min_h(px(32.)))
        .rounded(px(8.))
        .bg(bg)
        .border_1()
        .border_color(border)
        .when(disabled, |d| d.opacity(0.5))
        .when(!disabled, |d| {
            d.cursor_pointer().hover(move |s| s.bg(hover))
        })
        .children(lead)
        .child(text(label, size, 600., fg).line_height(px(line)))
}

pub fn text_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    trailing: Option<(&str, f32)>,
) -> Stateful<Div> {
    let color = c(theme::TEXT_BUTTON);
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .cursor_pointer()
        .child(text(label, 11., 600., color).line_height(px(17.6)))
        .when_some(trailing, |d, (name, size)| {
            d.child(icon(name, size, "2", color))
        })
}

/// Handler shared by a modal's close button and backdrop.
pub type CloseHandler = std::rc::Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

pub fn modal(
    id: &'static str,
    title: String,
    wide: bool,
    body: AnyElement,
    on_close: CloseHandler,
    window: &Window,
) -> AnyElement {
    let viewport = window.viewport_size();
    let close = on_close.clone();
    div()
        .id(SharedString::from(format!("{id}-backdrop")))
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p(px(30.))
        .bg(theme::hexa(0x18301d, 0.33))
        .occlude()
        .on_click(move |event, window, cx| close(event, window, cx))
        .child(
            div()
                .id(id)
                .w_full()
                .max_w(px(if wide { 850. } else { 480. }))
                .max_h(viewport.height * 0.9)
                .overflow_y_scroll()
                .p(px(25.))
                .bg(c(theme::PANEL))
                .border_1()
                .border_color(c(theme::BORDER))
                .rounded(px(17.))
                .shadow(vec![shadow(0x112d30, 0.21, 30., 100.)])
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(20.))
                        .pb(px(16.))
                        .mb(px(23.))
                        .border_b_1()
                        .border_color(c(theme::BORDER))
                        .child(spaced(title, 16., 650., c(theme::HEADING), -0.4, 21.))
                        .child(
                            div()
                                .id(SharedString::from(format!("{id}-close")))
                                .size(px(30.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(7.))
                                .cursor_pointer()
                                .child(icon("x", 20., "2", hex(0x6d7c70)))
                                .on_click(move |event, window, cx| on_close(event, window, cx)),
                        ),
                )
                .child(body),
        )
        .into_any_element()
}

fn metric_label(key: &str) -> &str {
    match key {
        "allocated_bytes" => "common.area_allocated_bytes",
        "verified_bytes" => "common.audited_bytes",
        "errors" => "common.errors",
        "patterns_completed" => "common.patterns_completed",
        "patterns_planned" => "common.patterns_planned",
        "passes_completed" => "common.passes_completed",
        "locked" => "common.pages_locked_ram",
        "corrected" => "common.corrected_ecc_errors",
        "uncorrected" => "common.uncorrected_ecc_errors",
        "smartctl_exit_status" => "common.smart_code_bit_mask",
        "temperature_c" => "common.temperature_c",
        "power_on_hours" => "common.operating_hours",
        "device" => "common.device",
        "model" => "common.model",
        "file_size_bytes" => "common.file_size_bytes",
        "mismatched_bytes" => "common.divergent_bytes",
        "write_ms" => "common.writing_syncing_ms",
        "read_ms" => "common.reading_comparison_ms",
        "directory" => "common.test_volume",
        "cache_eviction_advised" => "common.cache_eviction_requested",
        "workers" => "common.calculation_threads",
        "iterations" => "common.verified_calculations",
        "hashed_bytes" => "common.hashed_bytes",
        "algorithm" => "common.algorithm",
        "first_error" => "common.first_inconsistency",
        other => other,
    }
}

fn check_icon(status: Status) -> (&'static str, Hsla, Hsla) {
    match status_info(status).tone {
        Tone::Good => ("shield-check", hex(0x83a463), c((0xf0f6e9, 0x30452a))),
        Tone::Warn => ("circle-help", hex(0xb4985f), c((0xfbf5e9, 0x473e27))),
        Tone::Bad => ("triangle-alert", hex(0xc18374), c((0xfbefeb, 0x4b3228))),
        Tone::Muted => ("circle-help", hex(0xadbaa0), c((0xf7f9f6, 0x30452a))),
    }
}

pub fn check_card(app: &Computy, check: &Check, cx: &mut Context<Computy>) -> impl IntoElement {
    let key = format!("{}:{}", check.id, check.name);
    let open = app.pages.open_details.contains(&key);
    let (icon_name, icon_color, icon_bg) = check_icon(check.status);
    let icon_name = if check.status == Status::Failed {
        "triangle-alert"
    } else {
        icon_name
    };
    let locale = crate::i18n::locale();
    let metrics: Vec<(String, String)> = check
        .metrics
        .iter()
        .filter(|(_, v)| !v.is_null() && !v.is_object() && !v.is_array())
        .map(|(k, v)| {
            let value = match v {
                serde_json::Value::Bool(b) => tx(if *b { "common.yes" } else { "common.no" }),
                serde_json::Value::Number(n) => intl::plain(locale, n.as_f64().unwrap_or(0.)),
                serde_json::Value::String(s) => tx(s),
                other => other.to_string(),
            };
            (tx(metric_label(k)), value)
        })
        .collect();
    let raw = check
        .metrics
        .values()
        .any(|v| v.is_object() || v.is_array())
        .then(|| serde_json::to_string_pretty(&check.metrics).unwrap_or_default());
    let toggle_key = key.clone();
    div()
        .id(SharedString::from(format!("check-{key}")))
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c(theme::BORDER))
        .rounded(px(10.))
        .overflow_hidden()
        .child(
            div()
                .id(SharedString::from(format!("check-summary-{key}")))
                .flex()
                .items_center()
                .gap(px(13.))
                .p(px(16.))
                .cursor_pointer()
                .child(
                    div()
                        .size(px(34.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(9.))
                        .bg(icon_bg)
                        .child(icon(icon_name, 20., "2", icon_color)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(text(tx(&check.name), 13., 600., c(theme::TEXT)))
                        .child(
                            text(tx(&check.message), 12., 400., c((0x7c9168, 0xadc29a)))
                                .line_height(px(20.4)),
                        ),
                )
                .child(status_badge(check.status, false, None, true))
                .child(icon(
                    if open {
                        "chevron-down"
                    } else {
                        "chevron-right"
                    },
                    17.,
                    "2",
                    hex(0xb5c0aa),
                ))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !this.pages.open_details.remove(&toggle_key) {
                        this.pages.open_details.insert(toggle_key.clone());
                    }
                    cx.notify();
                })),
        )
        .when(open, |d| {
            d.child(
                div()
                    .pt(px(4.))
                    .pr(px(20.))
                    .pb(px(17.))
                    .pl(px(62.))
                    .border_t_1()
                    .border_color(c(theme::BORDER))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .children(metrics.into_iter().map(|(label, value)| {
                                div()
                                    .flex()
                                    .justify_between()
                                    .gap(px(15.))
                                    .py(px(10.))
                                    .border_b_1()
                                    .border_color(c(theme::BORDER))
                                    .child(text(label, 12., 400., hex(0x9dac8b)))
                                    .child(text(value, 12., 400., hex(0x7c9364)))
                            }))
                            .when(check.duration_ms > 0., |d| {
                                d.child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .gap(px(15.))
                                        .py(px(10.))
                                        .border_b_1()
                                        .border_color(c(theme::BORDER))
                                        .child(text(
                                            tx("common.measured_duration"),
                                            12.,
                                            400.,
                                            hex(0x9dac8b),
                                        ))
                                        .child(text(
                                            intl::duration(locale, check.duration_ms),
                                            12.,
                                            400.,
                                            hex(0x7c9364),
                                        )),
                                )
                            }),
                    )
                    .when(!check.limitations.is_empty(), |d| {
                        d.child(
                            div()
                                .mt(px(16.))
                                .child(spaced(
                                    tx("common.check_scope"),
                                    11.,
                                    600.,
                                    hex(0x91a37b),
                                    0.4,
                                    20.35,
                                ))
                                .child(div().mt(px(5.)).pl(px(16.)).flex().flex_col().children(
                                    check.limitations.iter().map(|note| {
                                        bullet(tx(note), 11., 20.35, c((0x81956d, 0xadc29a)))
                                    }),
                                )),
                        )
                    })
                    .when_some(raw, |d, raw| {
                        d.child(
                            div()
                                .mt(px(15.))
                                .child(text(
                                    tx("common.full_technical_data"),
                                    9.,
                                    400.,
                                    hex(0xa6b596),
                                ))
                                .child(text(raw, 9., 400., hex(0x8da775)).font_family("monospace")),
                        )
                    }),
            )
        })
}

/// `<li>` with a disc marker.
pub fn bullet(content: String, size: f32, line_height: f32, color: Hsla) -> Div {
    div()
        .flex()
        .items_start()
        .gap(px(8.))
        .text_size(px(size))
        .line_height(px(line_height))
        .text_color(color)
        .child(
            div()
                .flex_none()
                .size(px(size * 0.36))
                .mt(px((line_height - size * 0.36) / 2.))
                .ml(px(-12.))
                .rounded_full()
                .bg(color),
        )
        .child(div().flex_1().child(content))
}

pub fn report_details(
    app: &mut Computy,
    report: &Report,
    relaunch: bool,
    relaunch_disabled: bool,
    _window: &mut Window,
    cx: &mut Context<Computy>,
) -> AnyElement {
    let locale = crate::i18n::locale();
    let memory = report.checks.iter().find(|c| c.id == "memory-patterns");
    let report_for_relaunch = report.clone();
    let report_for_export = report.clone();
    let unavailable: Vec<&Check> = report
        .checks
        .iter()
        .filter(|c| c.status == Status::Unavailable)
        .collect();
    let info_bold = notice_bold(NoticeTone::Info);
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(20.))
                .mb(px(23.))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .child(div().mb(px(10.)).child(spaced(
                            intl::date(locale, report.created_at as i64, false),
                            10.,
                            650.,
                            hex(0x899582),
                            1.75,
                            16.5,
                        )))
                        .child(spaced(
                            &report.label,
                            20.,
                            650.,
                            c(theme::HEADING),
                            -0.5,
                            30.,
                        ))
                        .child(
                            text(
                                format!(
                                    "{}{}{}{}{}",
                                    tx(&report.inventory.host_name),
                                    tx(" · "),
                                    tx(&report.inventory.os),
                                    tx(" · "),
                                    intl::duration(locale, report.duration_ms)
                                ),
                                12.,
                                400.,
                                hex(0x9aaa88),
                            )
                            .line_height(px(19.8))
                            .mt(px(8.)),
                        ),
                )
                .child(score(report.summary.score, report.summary.status, true)),
        )
        .when_some(memory, |d, memory| {
            let allocated = memory
                .metrics
                .get("allocated_bytes")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.);
            let passes = memory
                .metrics
                .get("passes_completed")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.);
            d.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(7.))
                    .p(px(18.))
                    .mb(px(20.))
                    .border_1()
                    .border_color(c(theme::BORDER))
                    .rounded(px(12.))
                    .child(text(
                        tx("common.memory_actually_tested"),
                        13.,
                        700.,
                        c(theme::TEXT),
                    ))
                    .child(text(
                        format!(
                            "{} / {} · {}",
                            intl::bytes(locale, Some(allocated), 1),
                            intl::bytes(locale, Some(report.inventory.memory.total_bytes), 1),
                            tp("common.passes_completed_count", &[("count", passes.into())])
                        ),
                        13.,
                        400.,
                        c(theme::TEXT),
                    ))
                    .child(
                        text(
                            tx("common.test_runs_session_without_restarting"),
                            13.,
                            400.,
                            hex(0x6d7c70),
                        )
                        .line_height(px(20.8)),
                    ),
            )
        })
        .when(!unavailable.is_empty(), |d| {
            d.child(
                notice(
                    NoticeTone::Info,
                    vec![
                        text(
                            tx("common.missing_information_not_detected_errors"),
                            10.,
                            600.,
                            info_bold,
                        )
                        .line_height(px(17.5))
                        .into_any_element(),
                        div()
                            .line_height(px(16.5))
                            .child(tx("common.these_controls_not_noted_their"))
                            .into_any_element(),
                        div()
                            .mt(px(8.))
                            .pl(px(18.))
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .children(unavailable.iter().map(|check| {
                                let name = tx(&check.name);
                                let message = format!(" — {}", tx(&check.message));
                                let color = c((0x748666, 0xa4c580));
                                div()
                                    .flex()
                                    .items_start()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .flex_none()
                                            .size(px(4.))
                                            .mt(px(7.))
                                            .ml(px(-12.))
                                            .rounded_full()
                                            .bg(color),
                                    )
                                    .child(div().flex_1().child(rich(
                                        &[(&name, true), (&message, false)],
                                        info_bold,
                                    )))
                            }))
                            .into_any_element(),
                    ],
                )
                .text_size(px(10.))
                .line_height(px(17.5))
                .mb(px(18.)),
            )
        })
        .child({
            let coverage = format!(
                "{}{}{}{}",
                tn(report.summary.checked as f64),
                tx("/"),
                tn(report.summary.total as f64),
                tx(" common.usable_checks")
            );
            let lead = tx("common.note_summarizes_controls_performed ");
            let tail = tx("common.not_probability_proper_functioning");
            notice(
                NoticeTone::Info,
                vec![
                    rich(
                        &[(&lead, false), (&coverage, true), (&tail, false)],
                        info_bold,
                    )
                    .into_any_element(),
                ],
            )
            .text_size(px(10.))
            .line_height(px(17.5))
            .mb(px(18.))
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(15.))
                .mb(px(20.))
                .child(status_badge(
                    report.summary.status,
                    false,
                    Some(&report.checks),
                    report.completed,
                ))
                .when(relaunch, |d| {
                    d.child(
                        button(
                            "relaunch",
                            ButtonKind::Primary,
                            true,
                            Some("rotate-ccw"),
                            tx("common.run_again"),
                            relaunch_disabled,
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                if !relaunch_disabled {
                                    this.relaunch(report_for_relaunch.clone(), window, cx)
                                }
                            },
                        )),
                    )
                })
                .child(
                    button(
                        "export",
                        ButtonKind::Secondary,
                        true,
                        Some("arrow-down-to-line"),
                        tx("common.export_json"),
                        false,
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.export_report(&report_for_export, window, cx)
                    })),
                ),
        )
        .child(
            div().flex().flex_col().gap(px(12.)).children(
                report
                    .checks
                    .iter()
                    .map(|check| check_card(app, check, cx).into_any_element()),
            ),
        )
        .child(
            div()
                .mt(px(18.))
                .child(text(tx("common.report_details"), 10., 400., hex(0xa6b596)))
                .when(app.pages.open_details.contains("report-footnote"), |d| {
                    d.child(text(
                        format!(
                            "{}{}{}{}",
                            tx("common.rust_engine_schema_version"),
                            tn(report.schema_version as f64),
                            tx(" common.id_separator "),
                            report.id
                        ),
                        10.,
                        400.,
                        hex(0xa6b596),
                    ))
                }),
        )
        .into_any_element()
}

pub fn engine_error(error: String, cx: &mut Context<Computy>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(18.))
        .bg(hex(0xfcf1e5))
        .border_1()
        .border_color(hex(0xedddc3))
        .rounded(px(11.))
        .py(px(17.))
        .px(px(20.))
        .mb(px(25.))
        .child(
            div()
                .flex()
                .flex_col()
                .child(text(
                    tx("common.native_engine_connection_impossible"),
                    12.,
                    700.,
                    hex(0xae9167),
                ))
                .child(text(tx(&error), 10., 400., hex(0xae9167)).mt(px(5.))),
        )
        .child(
            button(
                "retry",
                ButtonKind::Secondary,
                true,
                None,
                tx("common.try_again"),
                false,
            )
            .on_click(cx.listener(|this, _, _, cx| this.refresh(true, cx))),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::prelude::v1::test;

    fn check(component: &str, status: Status) -> Check {
        Check {
            component: component.into(),
            status,
            ..Default::default()
        }
    }

    #[test]
    fn scores_describe_executed_checks() {
        assert_eq!(score_status(Some(10.), Status::Warning), Status::Passed);
        assert_eq!(score_status(Some(0.), Status::Warning), Status::Failed);
        assert_eq!(score_status(None, Status::Unavailable), Status::Unavailable);
        let summary = component_summary(
            &[
                check("memory", Status::Passed),
                check("memory", Status::Unavailable),
            ],
            "memory",
        );
        assert_eq!(
            (summary.score, summary.status),
            (Some(10.), Status::Warning)
        );
        assert_eq!(component_summary(&[], "cpu").status, Status::Unavailable);
        let outcome = check_outcome(
            &[
                check("cpu", Status::Passed),
                check("cpu", Status::Unavailable),
            ],
            true,
        );
        assert_eq!(outcome.status, Status::Passed);
        assert_eq!(outcome.unavailable.len(), 1);
        assert_eq!(
            check_outcome(&[check("cpu", Status::Passed)], false).status,
            Status::Cancelled
        );
    }
}
