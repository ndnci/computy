//! Native RGBA indicators. Colors encode utilization, never health.
pub const SIZE: u32 = 64;
pub fn usage_color(value: Option<f64>) -> [u8; 4] {
    match value.filter(|v| v.is_finite() && (0.0..=100.0).contains(v)) {
        None => [139, 155, 157, 255],
        Some(v) if v >= 90.0 => [233, 101, 101, 255],
        Some(v) if v >= 65.0 => [220, 168, 66, 255],
        _ => [66, 159, 235, 255],
    }
}
pub fn render_style(label: &str, value: Option<f64>, style: &str, compact_text: bool) -> Vec<u8> {
    let value = value.filter(|v| v.is_finite() && (0.0..=100.0).contains(v));
    let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
    let color = usage_color(value);
    let text = value
        .map(|v| format!("{v:.0}%"))
        .unwrap_or_else(|| "--".into());
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
        text_at(
            &mut pixels,
            &text,
            if text.len() > 3 { 21 } else { 19 },
            if text.len() > 3 { 2 } else { 3 },
            color,
        );
        text_at(&mut pixels, label, 53, 2, [245, 250, 250, 255]);
        return pixels;
    }
    let icon_height = if compact_text && style == "icon_text" {
        38
    } else {
        64
    };
    for y in 0..icon_height {
        for x in 0..SIZE {
            let px = (x as f64 / 64.0 * 24.0) as i32;
            let py = (y as f64 / icon_height as f64 * 24.0) as i32;
            let (body, line) = hardware(label, px, py);
            if !body && !line {
                continue;
            }
            let rgba = if style != "fill" {
                if line { color } else { [0; 4] }
            } else if value.is_none() {
                if line || (px - py).abs() < 2 {
                    [139, 155, 157, 255]
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
        text_at(&mut pixels, &text, 44, 3, color);
    }
    pixels
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
}
