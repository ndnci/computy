//! Light and dark colors. Each pair is (light, dark), taken from the computed
//! styles of the former stylesheet so both themes keep their exact look.
use gpui::{Hsla, Rgba};
use std::sync::atomic::{AtomicBool, Ordering};

static DARK: AtomicBool = AtomicBool::new(false);

pub fn set_dark(dark: bool) {
    DARK.store(dark, Ordering::Relaxed);
}

pub fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

pub type Pair = (u32, u32);

pub fn hex(value: u32) -> Hsla {
    Rgba {
        r: ((value >> 16) & 0xff) as f32 / 255.,
        g: ((value >> 8) & 0xff) as f32 / 255.,
        b: (value & 0xff) as f32 / 255.,
        a: 1.,
    }
    .into()
}

pub fn hexa(value: u32, alpha: f32) -> Hsla {
    let mut color = hex(value);
    color.a = alpha;
    color
}

/// Color for the active theme.
pub fn c(pair: Pair) -> Hsla {
    hex(if is_dark() { pair.1 } else { pair.0 })
}

// Shared surface tokens.
pub const BG: Pair = (0xf6f8f6, 0x17221b);
pub const PANEL: Pair = (0xffffff, 0x202d23);
pub const SIDEBAR: Pair = (0xffffff, 0x1c281f);
pub const TOPBAR: Pair = (0xffffff, 0x17221b);
pub const BORDER: Pair = (0xe8ece7, 0x34432e);
pub const TEXT: Pair = (0x263d34, 0xc8d6bf);
pub const HEADING: Pair = (0x1d3329, 0xe0ead6);
pub const MUTED: Pair = (0x7e8982, 0x91a487);
pub const SOFT_TEXT: Pair = (0x7a8b6a, 0xadc29a);
pub const GREEN: Pair = (0x234d3c, 0x607d47);
pub const TEXT_BUTTON: Pair = (0x647952, 0xa8c989);

// Badges and states.
pub const GOOD: Pair = (0x527346, 0xa0c17d);
pub const GOOD_BG: Pair = (0xedf4e9, 0x2e4124);
pub const WARN: Pair = (0xa1762a, 0xc5ab72);
pub const WARN_BG: Pair = (0xfbf2df, 0x403a24);
pub const BAD: Pair = (0xb55350, 0xd39180);
pub const BAD_BG: Pair = (0xfbedeb, 0x442e26);
pub const MUTED_BADGE: Pair = (0x7c8b70, 0xb3c69f);
pub const MUTED_BADGE_BG: Pair = (0xf3f5ef, 0x303d29);

pub const SHADOW: u32 = 0x213b2a;
