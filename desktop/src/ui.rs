//! Styling primitives shared by every view: text with the stylesheet's metrics,
//! letter spacing, icons, badges and common containers.
use crate::theme::{self, c};
use gpui::{
    App, AvailableSpace, Bounds, BoxShadow, Div, Element, ElementId, Font, FontWeight,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, ParentElement, Pixels,
    Refineable, SharedString, Style, StyleRefinement, Styled, Svg, TextRun, Window, div, point, px,
    size as size2, svg,
};

/// DM Sans ascender + descender: CSS `line-height: normal`.
pub const NORMAL: f32 = 1.302;

/// A text run with explicit size, weight, line height and color.
pub fn text(content: impl Into<SharedString>, size: f32, weight: f32, color: Hsla) -> Div {
    div()
        .text_size(px(size))
        .font_weight(FontWeight(weight))
        .line_height(px((size * NORMAL).round()))
        .text_color(color)
        .child(content.into())
}

/// Single-line text clipped to its container (CSS `nowrap` + `overflow: hidden`).
/// GPUI's ellipsis measures before layout and collapses in flexible boxes.
pub fn one_line(content: impl Into<SharedString>, size: f32, weight: f32, color: Hsla) -> Div {
    text(content, size, weight, color)
        .whitespace_nowrap()
        .overflow_hidden()
        .min_w_0()
}

/// Text with CSS `letter-spacing`. GPUI text has no tracking: this element
/// shapes words normally (kerning, ligatures, complex scripts) and offsets each
/// glyph by the spacing accumulated over the preceding characters.
pub struct Spaced {
    text: SharedString,
    size: f32,
    weight: f32,
    color: Hsla,
    spacing: f32,
    line_height: f32,
    family: Option<SharedString>,
    nowrap: bool,
    centered: bool,
    style: StyleRefinement,
}

pub fn spaced(
    content: impl Into<SharedString>,
    size: f32,
    weight: f32,
    color: Hsla,
    spacing: f32,
    line_height: f32,
) -> Spaced {
    Spaced {
        text: content.into(),
        size,
        weight,
        color,
        spacing,
        line_height,
        family: None,
        nowrap: false,
        centered: false,
        style: StyleRefinement::default(),
    }
}

impl Spaced {
    pub fn family(mut self, family: &'static str) -> Self {
        self.family = Some(family.into());
        self
    }
    pub fn nowrap(mut self) -> Self {
        self.nowrap = true;
        self
    }
}

impl Styled for Spaced {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

#[derive(Clone)]
pub struct SpacedLine {
    range: std::ops::Range<usize>,
    width: f32,
}

#[derive(Default)]
pub struct SpacedLayout {
    lines: Vec<SpacedLine>,
    width: f32,
}

impl IntoElement for Spaced {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Spaced {
    fn resolved_font(&self, window: &Window) -> Font {
        let mut font = window.text_style().font();
        font.weight = FontWeight(self.weight);
        if let Some(family) = &self.family {
            font.family = family.clone();
        }
        font
    }

    fn text_run(&self, font: &Font, len: usize) -> TextRun {
        TextRun {
            len,
            font: font.clone(),
            color: self.color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }
}

fn measure_part(text: &str, font: &Font, size: f32, spacing: f32, window: &Window) -> f32 {
    if text.is_empty() {
        return 0.;
    }
    let run = TextRun {
        len: text.len(),
        font: font.clone(),
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = window.text_system().shape_line(
        SharedString::from(text.to_owned()),
        px(size),
        &[run],
        None,
    );
    f32::from(shaped.width) + spacing * text.chars().count() as f32
}

fn layout_lines(
    text: &str,
    font: &Font,
    size: f32,
    spacing: f32,
    max: Option<f32>,
    window: &Window,
) -> SpacedLayout {
    let full = measure_part(text, font, size, spacing, window);
    if max.is_none_or(|max| full <= max + 0.5) || !text.contains(' ') {
        return SpacedLayout {
            lines: vec![SpacedLine {
                range: 0..text.len(),
                width: full,
            }],
            width: full,
        };
    }
    let max = max.unwrap_or(f32::MAX);
    let mut lines = Vec::new();
    let mut start = 0;
    let mut end = 0;
    let mut cursor = 0;
    for word in text.split_inclusive(' ') {
        let candidate_end = cursor + word.len();
        let candidate = text[start..candidate_end].trim_end();
        let width = measure_part(candidate, font, size, spacing, window);
        if width > max + 0.5 && end > start {
            let line = text[start..end].trim_end();
            lines.push(SpacedLine {
                range: start..start + line.len(),
                width: measure_part(line, font, size, spacing, window),
            });
            start = end;
        }
        end = candidate_end;
        cursor = candidate_end;
    }
    if start < text.len() {
        let line = text[start..].trim_end();
        lines.push(SpacedLine {
            range: start..start + line.len(),
            width: measure_part(line, font, size, spacing, window),
        });
    }
    let width = lines.iter().map(|l| l.width).fold(0., f32::max);
    SpacedLayout { lines, width }
}

impl Element for Spaced {
    type RequestLayoutState = std::rc::Rc<std::cell::RefCell<SpacedLayout>>;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);
        let state = std::rc::Rc::new(std::cell::RefCell::new(SpacedLayout::default()));
        let shared = state.clone();
        let text = self.text.clone();
        let font = self.resolved_font(window);
        let (size, spacing, line_height, nowrap) =
            (self.size, self.spacing, self.line_height, self.nowrap);
        let layout_id =
            window.request_measured_layout(style, move |known, available, window, _| {
                let max = if nowrap {
                    None
                } else {
                    known.width.map(f32::from).or(match available.width {
                        AvailableSpace::Definite(width) => Some(f32::from(width)),
                        _ => None,
                    })
                };
                let layout = layout_lines(&text, &font, size, spacing, max, window);
                let measured = size2(
                    known.width.unwrap_or(px(layout.width.ceil())),
                    known
                        .height
                        .unwrap_or(px(line_height * layout.lines.len().max(1) as f32)),
                );
                *shared.borrow_mut() = layout;
                measured
            });
        let _ = cx;
        (layout_id, state)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) {
        let font = self.resolved_font(window);
        let layout = layout.borrow();
        for (row, line) in layout.lines.iter().enumerate() {
            let content = &self.text[line.range.clone()];
            if content.is_empty() {
                continue;
            }
            let shaped = window.text_system().shape_line(
                SharedString::from(content.to_owned()),
                px(self.size),
                &[self.text_run(&font, content.len())],
                None,
            );
            let ascent = f32::from(shaped.ascent);
            let descent = f32::from(shaped.descent);
            let baseline = f32::from(bounds.origin.y)
                + row as f32 * self.line_height
                + (self.line_height - (ascent + descent)) / 2.
                + ascent;
            let mut left = f32::from(bounds.origin.x);
            if self.centered {
                left += (f32::from(bounds.size.width) - line.width) / 2.;
            }
            let char_starts: Vec<usize> = content.char_indices().map(|(i, _)| i).collect();
            for run in &shaped.runs {
                for glyph in &run.glyphs {
                    let before = char_starts.partition_point(|&i| i < glyph.index) as f32;
                    let origin = point(
                        px(left + f32::from(glyph.position.x) + before * self.spacing),
                        px(baseline),
                    );
                    if glyph.is_emoji {
                        let _ = window.paint_emoji(origin, run.font_id, glyph.id, px(self.size));
                    } else {
                        let _ = window.paint_glyph(
                            origin,
                            run.font_id,
                            glyph.id,
                            px(self.size),
                            self.color,
                        );
                    }
                }
            }
        }
    }
}

pub fn icon(name: &str, size: f32, stroke: &str, color: Hsla) -> Svg {
    svg()
        .path(SharedString::from(format!("icons/{name}@{stroke}.svg")))
        .size(px(size))
        .flex_none()
        .text_color(color)
}

pub fn shadow(color: u32, alpha: f32, y: f32, blur: f32) -> BoxShadow {
    BoxShadow {
        color: theme::hexa(color, alpha),
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(0.),
    }
}

/// White panel with border and soft shadow (`.panel`).
pub fn panel() -> Div {
    div()
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c(theme::BORDER))
        .rounded(px(14.))
        .shadow(vec![shadow(theme::SHADOW, 0.012, 5., 20.)])
}
