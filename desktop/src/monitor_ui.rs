//! Live metric visuals shared by the settings page and the tray popover
//! (former monitor-components.tsx).
use crate::{
    i18n::tp,
    pages::overview::ring,
    theme::{self, c, hex},
    ui::{icon, spaced},
};
use gpui::*;

/// Utilization colors (never health): blue, then yellow from 65 %, red from 90 %.
pub fn usage_color(value: Option<f64>) -> Hsla {
    match value.filter(|v| v.is_finite()) {
        None => hex(0x8b9b9d),
        Some(v) if v >= 90. => hex(0xe96565),
        Some(v) if v >= 65. => hex(0xdca842),
        _ => hex(0x429feb),
    }
}

pub fn metric_icon(metric: &str) -> &'static str {
    match metric {
        "memory" => "memory-stick",
        "cpu" => "cpu",
        "gpu" => "monitor",
        _ => "hard-drive",
    }
}

pub fn percent_label(value: Option<f64>) -> String {
    value.map_or_else(
        || "—".to_owned(),
        |v| tp("{{value}} %", &[("value", v.round().into())]),
    )
}

/// `MetricVisual` with the three tray styles. `ring_size` is 86 by default,
/// 65 in style previews and 96 in the popover.
pub fn metric_visual(
    value: Option<f64>,
    metric: &str,
    style: &str,
    ring_size: f32,
    min_height: f32,
) -> AnyElement {
    let color = usage_color(value);
    let label = percent_label(value);
    match style {
        "ring" => div()
            .relative()
            .size(px(ring_size))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(ring(
                ring_size * 93. / 100.,
                ring_size * 7. / 100.,
                value.unwrap_or(0.).clamp(0., 100.) as f32 / 100.,
                color,
                c(theme::BORDER),
                1.,
            ))
            .map(|d| {
                d.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            spaced(
                                label,
                                ring_size * 18. / 100.,
                                750.,
                                color,
                                0.,
                                ring_size * 0.24,
                            )
                            .nowrap(),
                        ),
                )
            })
            .into_any_element(),
        "fill" => {
            let fill = value.unwrap_or(0.).clamp(0., 100.) as f32 / 100.;
            let name = metric_icon(metric);
            div()
                .relative()
                .size(px(58.))
                .flex_none()
                .child(icon(name, 58., "1.8", c((0xa2aaa4, 0x819479))).opacity(0.5))
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(relative(fill))
                        .overflow_hidden()
                        .child(
                            svg()
                                .path(SharedString::from(format!("icons/{name}@1.8-fill.svg")))
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .size(px(58.))
                                .text_color(color),
                        ),
                )
                .into_any_element()
        }
        _ => div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(9.))
            .min_h(px(min_height))
            .child(icon(metric_icon(metric), 25., "1.8", color))
            .child(spaced(label, 25., 720., color, -1., 33.).nowrap())
            .into_any_element(),
    }
}

trait MapDiv: Sized {
    fn map(self, f: impl FnOnce(Self) -> Self) -> Self {
        f(self)
    }
}
impl MapDiv for Div {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ProcessSample, ranked_processes};
    use std::prelude::v1::test;

    #[test]
    fn usage_palette_and_ranking() {
        assert_eq!(usage_color(Some(64.9)), hex(0x429feb));
        assert_eq!(usage_color(Some(65.)), hex(0xdca842));
        assert_eq!(usage_color(Some(90.)), hex(0xe96565));
        assert_eq!(usage_color(None), hex(0x8b9b9d));
        let processes = [
            ProcessSample {
                pid: 1,
                memory_bytes: None,
                ..Default::default()
            },
            ProcessSample {
                pid: 2,
                memory_bytes: Some(42.),
                ..Default::default()
            },
            ProcessSample {
                pid: 3,
                memory_bytes: Some(100.),
                ..Default::default()
            },
        ];
        let ranked: Vec<u32> = ranked_processes(&processes, "memory")
            .iter()
            .map(|p| p.pid)
            .collect();
        assert_eq!(ranked, [3, 2, 1]);
    }
}
