//! Native RGBA indicators. Usage colors encode utilization and battery colors
//! encode remaining charge; neither is a health score.
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

pub const SIZE: u32 = 64;
const UNKNOWN: [u8; 4] = [139, 155, 157, 255];
const BLUE: [u8; 4] = [66, 159, 235, 255];
const YELLOW: [u8; 4] = [220, 168, 66, 255];
const RED: [u8; 4] = [233, 101, 101, 255];

fn valid(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite() && (0.0..=100.0).contains(v))
}

pub fn usage_color(value: Option<f64>) -> [u8; 4] {
    match valid(value) {
        None => UNKNOWN,
        Some(v) if v >= 90.0 => RED,
        Some(v) if v >= 65.0 => YELLOW,
        _ => BLUE,
    }
}

/// Remaining charge: red at 15 % and below, yellow up to 30 %, then blue.
pub fn battery_color(value: Option<f64>) -> [u8; 4] {
    match valid(value) {
        None => UNKNOWN,
        Some(v) if v <= 15.0 => RED,
        Some(v) if v <= 30.0 => YELLOW,
        _ => BLUE,
    }
}

/// Shape drawn inside the indicator.
enum Glyph<'a> {
    /// Pixel-drawn component (RAM, CPU, GPU, DSK) with its band label.
    Hardware(&'a str),
    /// Lucide icon rasterized from the embedded SVG (wireless peripherals).
    Icon(&'a str),
}

pub fn render_style(label: &str, value: Option<f64>, style: &str, compact_text: bool) -> Vec<u8> {
    let value = valid(value);
    let text = value
        .map(|v| format!("{v:.0}%"))
        .unwrap_or_else(|| "--".into());
    render(
        Glyph::Hardware(label),
        value,
        usage_color(value),
        Some(&text),
        style,
        compact_text,
    )
}

/// Peripheral battery. `icon` is a Lucide icon name; `exact` is false when the
/// device only reports a coarse level, which is then never written as a number.
pub fn render_battery(
    icon: &str,
    value: Option<f64>,
    exact: bool,
    style: &str,
    compact_text: bool,
) -> Vec<u8> {
    let value = valid(value);
    let text = value.map(|v| format!("{v:.0}%"));
    render(
        Glyph::Icon(icon),
        value,
        battery_color(value),
        if exact || value.is_none() {
            Some(text.as_deref().unwrap_or("--"))
        } else {
            None
        },
        style,
        compact_text,
    )
}

/// Rasterized masks by (icon, filled, width, height).
type MaskCache = Mutex<HashMap<(String, bool, u32, u32), Vec<u8>>>;

/// Alpha mask of an embedded icon, fitted and centered in `width`×`height`.
fn mask(name: &str, filled: bool, width: u32, height: u32) -> Vec<u8> {
    static CACHE: OnceLock<MaskCache> = OnceLock::new();
    let key = (name.to_owned(), filled, width, height);
    let cache = CACHE.get_or_init(Default::default);
    if let Some(mask) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return mask;
    }
    let path = if filled {
        format!("icons/{name}@1.8-fill.svg")
    } else {
        format!("icons/{name}@2.svg")
    };
    let mask = crate::assets::embedded(&path)
        .and_then(|bytes| {
            let tree =
                resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).ok()?;
            let size = tree.size();
            let scale = (width as f32 / size.width()).min(height as f32 / size.height());
            let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_row(
                    scale,
                    0.,
                    0.,
                    scale,
                    (width as f32 - size.width() * scale) / 2.,
                    (height as f32 - size.height() * scale) / 2.,
                ),
                &mut pixmap.as_mut(),
            );
            Some(pixmap.data().chunks(4).map(|px| px[3]).collect())
        })
        .unwrap_or_else(|| vec![0; (width * height) as usize]);
    if let Ok(mut cache) = cache.lock() {
        cache.insert(key, mask.clone());
    }
    mask
}

fn render(
    glyph: Glyph,
    value: Option<f64>,
    color: [u8; 4],
    text: Option<&str>,
    style: &str,
    compact_text: bool,
) -> Vec<u8> {
    let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
    if style == "ring" {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let dx = f64::from(x) - 31.5;
                let dy = f64::from(y) - 26.5;
                let radius = dx.hypot(dy);
                let angle = (dx.atan2(-dy) + std::f64::consts::TAU) % std::f64::consts::TAU;
                let rgba = if (21.0..=25.0).contains(&radius) {
                    if value.is_some_and(|v| angle <= v / 100.0 * std::f64::consts::TAU) {
                        color
                    } else {
                        [90, 106, 111, 255]
                    }
                } else if radius < 21.0 || ((53..=63).contains(&y) && (4..=59).contains(&x)) {
                    [22, 36, 33, 255]
                } else {
                    [0; 4]
                };
                put(&mut pixels, x as i32, y as i32, rgba);
            }
        }
        if let Some(text) = text {
            text_at(
                &mut pixels,
                text,
                if text.len() > 3 { 21 } else { 19 },
                if text.len() > 3 { 2 } else { 3 },
                color,
            );
        } else {
            level_blocks(&mut pixels, value, 23, color);
        }
        match glyph {
            Glyph::Hardware(label) => text_at(&mut pixels, label, 53, 2, [245, 250, 250, 255]),
            Glyph::Icon(name) => {
                let small = mask(name, false, 56, 10);
                for y in 0..10 {
                    for x in 0..56 {
                        if small[(y * 56 + x) as usize] >= 110 {
                            put(&mut pixels, x + 4, y + 53, [245, 250, 250, 255]);
                        }
                    }
                }
            }
        }
        return pixels;
    }
    let icon_height = if compact_text && style == "icon_text" {
        38
    } else {
        64
    };
    let masks = match glyph {
        Glyph::Icon(name) => Some((
            mask(name, false, SIZE, icon_height as u32),
            mask(name, true, SIZE, icon_height as u32),
        )),
        Glyph::Hardware(_) => None,
    };
    for y in 0..icon_height {
        for x in 0..SIZE {
            let px = (x as f64 / 64.0 * 24.0) as i32;
            let py = (y as f64 / icon_height as f64 * 24.0) as i32;
            let (body, line) = match (&glyph, &masks) {
                (Glyph::Hardware(label), _) => hardware(label, px, py),
                (_, Some((line, fill))) => {
                    let i = (y as u32 * SIZE + x) as usize;
                    (fill[i] >= 128 || line[i] >= 128, line[i] >= 128)
                }
                _ => (false, false),
            };
            if !body && !line {
                continue;
            }
            let rgba = if style != "fill" {
                if line { color } else { [0; 4] }
            } else if value.is_none() {
                if line || (px - py).abs() < 2 {
                    UNKNOWN
                } else {
                    [50, 61, 65, 100]
                }
            } else if y as f64 >= (1.0 - value.unwrap() / 100.0) * icon_height as f64 {
                color
            } else if line {
                [170, 187, 192, 255]
            } else {
                [80, 100, 111, 55]
            };
            put(&mut pixels, x as i32, y, rgba);
        }
    }
    if compact_text && style == "icon_text" {
        for y in 40..64 {
            for x in 2..62 {
                put(&mut pixels, x, y, [22, 29, 33, 255]);
            }
        }
        match text {
            Some(text) => text_at(&mut pixels, text, 44, 3, color),
            None => level_blocks(&mut pixels, value, 46, color),
        }
    }
    pixels
}

/// Four blocks for a coarse battery level, instead of an invented number.
fn level_blocks(pixels: &mut [u8], value: Option<f64>, top: i32, color: [u8; 4]) {
    let filled = value.map_or(0, |v| ((v / 25.0).ceil() as i32).clamp(1, 4));
    for block in 0..4 {
        let rgba = if block < filled {
            color
        } else {
            [90, 106, 111, 255]
        };
        for y in top..top + 12 {
            for x in 0..8 {
                put(pixels, 13 + block * 10 + x, y, rgba);
            }
        }
    }
}
fn hardware(label: &str, x: i32, y: i32) -> (bool, bool) {
    match label {
        "RAM" => {
            let body = (2..=21).contains(&x) && (6..=17).contains(&y);
            let chips = (5..=14).contains(&y)
                && ((4..=7).contains(&x) || (10..=13).contains(&x) || (16..=19).contains(&x));
            let edge = body && (x <= 3 || x >= 20 || y <= 7 || y >= 16);
            let chip_edge = chips && (y <= 6 || y >= 13 || [4, 7, 10, 13, 16, 19].contains(&x));
            let pins = (18..=20).contains(&y) && (4..=19).contains(&x) && x % 3 != 0;
            (body || pins, edge || chip_edge || pins)
        }
        "DSK" => {
            let body = (4..=19).contains(&x) && (3..=21).contains(&y);
            (
                body,
                body && (x <= 5
                    || x >= 18
                    || y <= 4
                    || y >= 20
                    || y == 16
                    || ((x - 12).pow(2) + (y - 10).pow(2) <= 4)),
            )
        }
        "GPU" => {
            let body = (2..=21).contains(&x) && (5..=18).contains(&y);
            let circle = |cx: i32| {
                let d = (x - cx).pow(2) + (y - 11).pow(2);
                (5..=11).contains(&d)
            };
            let pins = (19..=20).contains(&y) && (5..=17).contains(&x) && x % 3 != 0;
            (
                body || pins,
                body && (x <= 3 || x >= 20 || y <= 6 || y >= 17 || circle(8) || circle(16)) || pins,
            )
        }
        _ => {
            let body = (5..=18).contains(&x) && (5..=18).contains(&y);
            let pins = ((2..=21).contains(&x) && (8..=16).contains(&y) && y % 4 == 0)
                || ((2..=21).contains(&y) && (8..=16).contains(&x) && x % 4 == 0);
            let edge = body
                && (x <= 6
                    || x >= 17
                    || y <= 6
                    || y >= 17
                    || ((9..=14).contains(&x) && (y == 9 || y == 14)));
            (body || pins, edge || pins && !body)
        }
    }
}
fn put(data: &mut [u8], x: i32, y: i32, color: [u8; 4]) {
    if (0..64).contains(&x) && (0..64).contains(&y) {
        let offset = ((y * 64 + x) * 4) as usize;
        data[offset..offset + 4].copy_from_slice(&color);
    }
}
fn glyph(c: char) -> [u8; 5] {
    match c {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 7, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 1, 1, 1],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        '%' => [5, 1, 2, 4, 5],
        '-' => [0, 0, 7, 0, 0],
        'C' => [7, 4, 4, 4, 7],
        'P' => [6, 5, 6, 4, 4],
        'U' => [5, 5, 5, 5, 7],
        'G' => [7, 4, 5, 5, 7],
        'R' => [6, 5, 6, 5, 5],
        'A' => [2, 5, 7, 5, 5],
        'M' => [5, 7, 7, 5, 5],
        'D' => [6, 5, 5, 5, 6],
        'S' => [7, 4, 7, 1, 7],
        'K' => [5, 5, 6, 5, 5],
        _ => [0; 5],
    }
}
fn text_at(data: &mut [u8], text: &str, y: i32, scale: i32, color: [u8; 4]) {
    let width = (text.len() as i32 * 4 - 1) * scale;
    let start = (64 - width) / 2;
    for (i, c) in text.chars().enumerate() {
        for (row, bits) in glyph(c).iter().enumerate() {
            for col in 0..3 {
                if bits & (1 << (2 - col)) != 0 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            put(
                                data,
                                start + i as i32 * 4 * scale + col * scale + sx,
                                y + row as i32 * scale + sy,
                                color,
                            );
                        }
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn three_styles_have_correct_color_boundaries_and_no_fake_zero() {
        assert_eq!(super::usage_color(Some(64.9)), [66, 159, 235, 255]);
        assert_eq!(super::usage_color(Some(65.0)), [220, 168, 66, 255]);
        assert_eq!(super::usage_color(Some(90.0)), [233, 101, 101, 255]);
        for style in ["icon_text", "ring", "fill"] {
            for label in ["RAM", "CPU", "GPU", "DSK"] {
                let unknown = super::render_style(label, None, style, true);
                assert_eq!(unknown.len(), 64 * 64 * 4);
                assert_ne!(unknown, super::render_style(label, Some(0.0), style, true));
                assert_eq!(
                    unknown,
                    super::render_style(label, Some(f64::NAN), style, true)
                );
                assert_ne!(
                    super::render_style(label, Some(0.0), style, true),
                    super::render_style(label, Some(100.0), style, true)
                );
            }
        }
    }

    #[test]
    fn battery_indicators_use_charge_colors_and_real_icons() {
        assert_eq!(super::battery_color(Some(15.0)), super::RED);
        assert_eq!(super::battery_color(Some(15.1)), super::YELLOW);
        assert_eq!(super::battery_color(Some(30.0)), super::YELLOW);
        assert_eq!(super::battery_color(Some(31.0)), super::BLUE);
        assert_eq!(super::battery_color(None), super::UNKNOWN);
        for style in ["icon_text", "ring", "fill"] {
            for icon in ["mouse", "keyboard", "headphones", "gamepad-2", "bluetooth"] {
                let full = super::render_battery(icon, Some(100.0), true, style, true);
                assert_eq!(full.len(), 64 * 64 * 4);
                assert!(
                    full.chunks(4).any(|px| px[3] > 0),
                    "{icon} {style} is empty"
                );
                assert_ne!(
                    full,
                    super::render_battery(icon, Some(10.0), true, style, true)
                );
                assert_ne!(full, super::render_battery(icon, None, true, style, true));
            }
        }
        // A coarse level never draws digits; fill shows no text either way.
        for style in ["icon_text", "ring"] {
            assert_ne!(
                super::render_battery("mouse", Some(55.0), false, style, true),
                super::render_battery("mouse", Some(55.0), true, style, true)
            );
        }
        assert_ne!(
            super::render_battery("mouse", Some(80.0), true, "icon_text", false),
            super::render_battery("keyboard", Some(80.0), true, "icon_text", false)
        );
    }
}
