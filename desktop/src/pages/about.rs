//! About Computy and the diagnostic method (former pages/About.tsx).
use crate::{
    app::Computy,
    components::{self, ButtonKind, NoticeTone, button_with, logo, notice},
    i18n::tx,
    platform,
    theme::{self, c, hex},
    ui::{icon, panel, spaced, text},
};
use gpui::*;

fn link(
    id: &'static str,
    label: &str,
    url: &'static str,
    color: Hsla,
    weight: f32,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(6.))
        .cursor_pointer()
        .child(text(tx(label).trim().to_owned(), 11., weight, color))
        .child(icon("arrow-up-right", 13., "2", color))
        .on_click(move |_, _, cx| platform::open_external(url, cx))
}

fn paragraph(content: &str) -> Div {
    text(tx(content), 12., 400., c((0x7d9269, 0xadc29a))).line_height(px(22.8))
}

fn value(icon_name: &str, title: &str, body: &str) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .p(px(22.))
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c(theme::BORDER))
        .rounded(px(13.))
        .flex()
        .flex_col()
        .child(icon(icon_name, 23., "2", hex(0x9eb780)))
        .child(spaced(tx(title), 14., 650., c(theme::HEADING), -0.2, 18.).mt(px(16.)))
        .child(paragraph(body).mt(px(10.)))
}

fn method(
    number: &str,
    title: &str,
    paragraphs: &[&str],
    reference: Option<(&'static str, &str, &'static str)>,
    last: bool,
) -> impl IntoElement {
    let mut body = div().flex_1().min_w_0().flex().flex_col().child(spaced(
        tx(title),
        14.,
        650.,
        c(theme::HEADING),
        -0.2,
        18.,
    ));
    for p in paragraphs {
        body = body.child(paragraph(p).mt(px(11.)));
    }
    if let Some((id, label, url)) = reference {
        body = body.child(div().mt(px(15.)).flex().child(link(
            id,
            label,
            url,
            hex(0x87a56b),
            400.,
        )));
    }
    div()
        .flex()
        .gap(px(19.))
        .py(px(23.))
        .when(!last, |d| d.border_b_1().border_color(c(theme::BORDER)))
        .child(
            div()
                .pt(px(2.))
                .child(text(tx(number), 10., 400., hex(0xa4b790))),
        )
        .child(body)
}

trait When: Sized {
    fn when(self, condition: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if condition { f(self) } else { self }
    }
}
impl When for Div {}

fn rule(score: &str, small: bool, color: u32, title: &str, body: &str) -> impl IntoElement {
    let mut value =
        div()
            .flex()
            .items_baseline()
            .child(spaced(tx(score), 24., 550., hex(color), -0.6, 31.));
    if small {
        value = value.child(text(tx("/10"), 10., 400., hex(color)));
    }
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(value)
        .child(text(tx(title), 12., 550., c(theme::TEXT)).mt(px(13.)))
        .child(
            text(tx(body), 11., 400., hex(0xa8b696))
                .line_height(px(20.9))
                .mt(px(9.)),
        )
}

pub fn render(_app: &mut Computy, _window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    let _ = cx;
    div()
        .flex()
        .flex_col()
        .child(components::section_title(
            Some("about.another_way_eyebrow"),
            "about.less_mystery_more_clarity",
            Some("about.computy_makes_physical_diagnostics_accessible"),
            None,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(38.))
                .p(px(35.))
                .mb(px(24.))
                .bg(c((0xedf4e3, 0x2c4028)))
                .border_1()
                .border_color(c((0xdce7cc, 0x405531)))
                .rounded(px(17.))
                .child(div().ml(px(1.)).child(logo(138.)))
                .child(
                    div()
                        .w(px(450.))
                        .flex()
                        .flex_col()
                        .child(spaced(format!("Computy · {} {}", tx("about.version"), env!("CARGO_PKG_VERSION")), 10., 650., hex(0x899582), 1.75, 13.).mt(px(6.)))
                        .child(
                            div()
                                .mt(px(14.))
                                .flex()
                                .flex_col()
                                .child(spaced(tx("about.understand_today"), 27., 650., c(theme::HEADING), -1., 37.8))
                                .child(spaced(tx("about.better_pick_tomorrow"), 27., 650., c(theme::HEADING), -1., 37.8)),
                        )
                        .child(paragraph("about.desktop_software_observe_hardware_check").mt(px(14.)))
                        .child(
                            div().mt(px(23.)).flex().child(
                                button_with("about-ndnci", ButtonKind::Primary, false, None, tx("about.ndnci_creation ").trim().to_owned(), false)
                                    .child(icon("arrow-up-right", 16., "2", hex(0xffffff)))
                                    .on_click(|_, _, cx| platform::open_external("https://www.ndnci.com", cx)),
                            ),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .gap(px(20.))
                .mb(px(24.))
                .child(value("code-2", "about.native_engine_rust", "about.calculations_hardware_reading_diagnostics_rust"))
                .child(value("lock-keyhole", "about.local_design", "about.diagnostics_their_history_remain_machine"))
                .child(value("shield-check", "about.explicit_limits", "about.unavailability_data_remains_unknown_interrupted")),
        )
        .child(
            panel()
                .mb(px(20.))
                .flex()
                .flex_col()
                .child(
                    components::panel_header("about.our_diagnostic_method", Some(tx("about.what_controlled_what_cannot_certified")), None)
                        .border_b_1()
                        .border_color(c(theme::BORDER)),
                )
                .child(
                    div()
                        .px(px(23.))
                        .flex()
                        .flex_col()
                        .child(method(
                            "01",
                            "about.memory_write_read_compare",
                            &[
                                "about.engine_allocates_bounded_area_writes",
                                "about.physical_locking_attempted_result_recorded",
                            ],
                            Some(("ref-memtest", "about.reference_memtest86_philosophy_algorithms ", "https://github.com/memtest86plus/memtest86plus#memory-testing-philosophy")),
                            false,
                        ))
                        .child(method(
                            "02",
                            "about.storage_telemetry_integrity",
                            &[
                                "about.computy_interrogates_smartctl_json_interprets",
                                "about.dedicated_file_created_without_crushing",
                            ],
                            Some(("ref-smart", "about.reference_smartmontools_smartctl ", "https://www.smartmontools.org/")),
                            false,
                        ))
                        .child(method(
                            "03",
                            "about.processor_known_answers",
                            &["about.up_four_threads_calculate_sha"],
                            None,
                            true,
                        )),
                ),
        )
        .child(
            panel()
                .mb(px(24.))
                .flex()
                .flex_col()
                .child(components::panel_header("about.readable_note_never_guarantee", None, None))
                .child(
                    div()
                        .px(px(23.))
                        .flex()
                        .gap(px(19.))
                        .child(rule("10", true, 0x82a965, "about.passed_check", "about.no_anomalies_detected_check_performed"))
                        .child(rule("5", true, 0xb99b5a, "common.needs_review", "about.wear_history_errors_requiring_examination"))
                        .child(rule("0", true, 0xbc7c6c, "common.issue_detected", "about.verified_error_physical_alert_present"))
                        .child(rule("—", false, 0xb0bda2, "about.no_note", "about.control_not_available_absent_interrupted")),
                )
                .child(
                    div().p(px(23.)).child(notice(
                        NoticeTone::Info,
                        vec![tx("about.report_notes_lowest_score_exploitable").into_any_element()],
                    )),
                ),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(20.))
                .p(px(25.))
                .mb(px(35.))
                .bg(c((0xf0f4e9, 0x253322)))
                .border_1()
                .border_color(c((0xe0e8d4, 0x394a2e)))
                .rounded(px(13.))
                .child(icon("memory-stick", 24., "2", hex(0xa5b78c)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(spaced(tx("about.why_no_serious_software_promises"), 14., 650., c(theme::HEADING), -0.2, 18.))
                        .child(paragraph("about.test_run_os_cannot_check").mt(px(12.)))
                        .child(div().mt(px(18.)).flex().child(link("ref-duration", "about.understand_duration_intermittent_breakdowns ", "https://www.memtest86.com/tech_execution-time.html", hex(0x87a56b), 400.))),
                ),
        )
        .child(
            div()
                .flex()
                .justify_center()
                .items_center()
                .gap(px(4.))
                .child(text(tx("about.designed_for_your_machines ").trim().to_owned(), 11., 400., hex(0xa6b795)).line_height(px(18.15)))
                .child(link("signature-ndnci", "NDNCI ", "https://www.ndnci.com", hex(0x91a97b), 600.)),
        )
        .into_any_element()
}
