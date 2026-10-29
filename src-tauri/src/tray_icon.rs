//! Tiny high-contrast, resolution-independent ring rasterizer. No webview or fonts.
pub const SIZE: u32 = 64;
pub fn render(label: &str, value: Option<f64>) -> Vec<u8> {
    let value = value.filter(|v| v.is_finite() && (0.0..=100.0).contains(v));
    let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
    let color = if value.is_some_and(|v| v >= 90.0) {
        [255, 184, 95, 255]
    } else {
        [162, 224, 143, 255]
    };
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = f64::from(x) - 31.5;
            let dy = f64::from(y) - 26.5;
            let distance = dx.hypot(dy);
            let angle = (dx.atan2(-dy) + std::f64::consts::TAU) % std::f64::consts::TAU;
            let rgba = if (21.0..=25.0).contains(&distance) {
                if value.is_some_and(|v| angle <= v / 100.0 * std::f64::consts::TAU) {
                    color
                } else {
                    [87, 104, 99, 255]
                }
            } else if distance < 21.0 || ((53..=63).contains(&y) && (4..=59).contains(&x)) {
                [22, 36, 33, 255]
            } else {
                [0, 0, 0, 0]
            };
            put(&mut pixels, x as i32, y as i32, rgba);
        }
    }
    let text = value
        .map(|v| format!("{:.0}%", v))
        .unwrap_or_else(|| "--".into());
    draw_text(
        &mut pixels,
        &text,
        if text.len() > 3 { 21 } else { 19 },
        if text.len() > 3 { 2 } else { 3 },
        [255, 255, 255, 255],
    );
    draw_text(&mut pixels, label, 53, 2, [255, 255, 255, 255]);
    pixels
}
fn put(data: &mut [u8], x: i32, y: i32, color: [u8; 4]) {
    if (0..SIZE as i32).contains(&x) && (0..SIZE as i32).contains(&y) {
        let i = ((y as u32 * SIZE + x as u32) * 4) as usize;
        data[i..i + 4].copy_from_slice(&color);
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
fn draw_text(data: &mut [u8], text: &str, y: i32, scale: i32, color: [u8; 4]) {
    let width = (text.len() as i32 * 4 - 1) * scale;
    let start = (SIZE as i32 - width) / 2;
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
    fn unknown_is_visibly_different_from_zero_and_icons_fit() {
        for label in ["RAM", "CPU", "GPU", "DSK"] {
            let unknown = super::render(label, None);
            assert_eq!(unknown.len(), 64 * 64 * 4);
            assert_ne!(unknown, super::render(label, Some(0.0)));
            assert_ne!(
                super::render(label, Some(0.0)),
                super::render(label, Some(100.0))
            );
            assert_eq!(super::render(label, Some(f64::NAN)), unknown);
        }
    }
}
