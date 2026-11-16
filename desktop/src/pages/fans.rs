//! Fan readings (former pages/Fans.tsx), read-only.
use crate::{
    app::Computy,
    components::{self, spinner},
    i18n::tx,
    model::{Fan, FanInventory},
    theme::{self, c, hex},
    ui::{icon, panel, spaced, text},
};
use gpui::{prelude::FluentBuilder, *};
use serde_json::json;
use std::time::Duration;

pub struct FansState {
    fans: Option<Vec<Fan>>,
    _poll: Task<()>,
}

impl FansState {
    pub fn new(cx: &mut Context<Computy>) -> Self {
        let poll = cx.spawn(async move |this, cx| {
            loop {
                let Ok(backend) = this.read_with(cx, |this, _| this.backend.clone()) else {
                    break;
                };
                let fixture = backend.is_fixture();
                let result = cx
                    .background_executor()
                    .spawn(async move { backend.typed::<FanInventory>("fans", json!({})) })
                    .await;
                let alive = this.update(cx, |this, cx| {
                    match result {
                        Ok(inventory) => {
                            if let Some(state) = this.pages.fans.as_mut() {
                                state.fans = Some(inventory.fans);
                            }
                        }
                        Err(error) => {
                            if let Some(state) = this.pages.fans.as_mut() {
                                state.fans = Some(vec![]);
                            }
                            this.notify(error, true, cx);
                        }
                    }
                    cx.notify();
                });
                if alive.is_err() || fixture {
                    break;
                }
                cx.background_executor().timer(Duration::from_secs(5)).await;
            }
        });
        Self {
            fans: None,
            _poll: poll,
        }
    }
}

pub fn utility_heading(icon_name: &str, size: f32, title: &str, body: &str) -> impl IntoElement {
    div()
        .flex()
        .items_start()
        .gap(px(14.))
        .child(icon(icon_name, size, "2", c(theme::GREEN)).mt(px(3.)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(spaced(tx(title), 17., 650., c(theme::HEADING), -0.4, 21.).family("Sora"))
                .child(
                    text(tx(body), 12., 400., hex(0x6d7c70))
                        .line_height(px(20.4))
                        .mt(px(6.)),
                ),
        )
}

pub fn tag(label: String) -> impl IntoElement {
    div()
        .py(px(2.))
        .px(px(6.))
        .border_1()
        .border_color(c(theme::BORDER))
        .rounded(px(5.))
        .child(text(label, 9., 400., hex(0x6d7c70)))
}

fn card(fan: &Fan) -> impl IntoElement {
    let rpm = fan.rpm.map_or("—".to_owned(), |v| {
        crate::intl::plain(crate::i18n::locale(), v)
    });
    panel()
        .flex_1()
        .min_w_0()
        .p(px(26.))
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .child(icon("fan", 29., "2", c(theme::TEXT)))
                .child(tag(fan.driver.clone())),
        )
        .child(spaced(fan.name.clone(), 16., 650., c(theme::HEADING), -0.4, 21.).mt(px(19.)))
        .child(
            div()
                .mt(px(19.))
                .flex()
                .items_baseline()
                .gap(px(6.))
                .child(
                    text(rpm, 33., 700., c(theme::TEXT))
                        .font_family("Sora")
                        .line_height(px(42.)),
                )
                .child(text("RPM", 12., 700., hex(0x6d7c70)).font_family("Sora")),
        )
        .child(
            text(
                format!(
                    "{} · {}",
                    tx("fans.pwm_duty_cycle"),
                    fan.pwm_percent
                        .map_or_else(|| tx("common.unavailable"), |v| format!("{} %", v.round()))
                ),
                12.,
                400.,
                hex(0x6d7c70),
            )
            .line_height(px(19.8))
            .mt(px(16.)),
        )
        .child(
            text(tx("fans.read_only"), 11., 400., hex(0x6d7c70))
                .line_height(px(18.7))
                .mt(px(13.)),
        )
}

pub fn render(app: &mut Computy, _window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    if app.pages.fans.is_none() {
        app.pages.fans = Some(FansState::new(cx));
    }
    let fans = app.pages.fans.as_ref().and_then(|s| s.fans.clone());
    let body = match fans {
        None => div()
            .flex()
            .justify_center()
            .py(px(40.))
            .child(spinner(18., c(theme::MUTED)))
            .into_any_element(),
        Some(fans) if fans.is_empty() => panel()
            .mt(px(24.))
            .p(px(26.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(10.))
            .child(icon("fan", 38., "2", c(theme::MUTED)))
            .child(text(
                tx("fans.no_fan_sensors_available"),
                14.,
                650.,
                c(theme::HEADING),
            ))
            .child(text(
                tx("fans.firmware_operating_system_provides_no"),
                12.,
                400.,
                c(theme::MUTED),
            ))
            .into_any_element(),
        Some(fans) => {
            let cards: Vec<AnyElement> = fans
                .iter()
                .map(|fan| card(fan).into_any_element())
                .collect();
            let mut rows = vec![];
            let mut iter = cards.into_iter().peekable();
            while iter.peek().is_some() {
                let row: Vec<AnyElement> = iter.by_ref().take(4).collect();
                let missing = 4 - row.len();
                rows.push(
                    div()
                        .flex()
                        .gap(px(12.))
                        .children(
                            row.into_iter()
                                .map(|c| div().flex_1().min_w_0().flex().child(c)),
                        )
                        .children((0..missing).map(|_| div().flex_1()))
                        .into_any_element(),
                );
            }
            div()
                .mt(px(24.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .children(rows)
                .into_any_element()
        }
    };
    div()
        .flex()
        .flex_col()
        .child(components::section_title(
            Some("fans.cooling_eyebrow"),
            "common.fans",
            Some("fans.measured_speed_hardware_capabilities"),
            None,
        ))
        .child(panel().mt(px(24.)).p(px(26.)).child(utility_heading(
            "shield-check",
            24.,
            "fans.automatic_control_preserved",
            "fans.manual_control_requires_validated_hardware",
        )))
        .child(body)
        .when(false, |d| d)
        .into_any_element()
}
