//! Responsive breakpoints of the former stylesheet (window width).
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Width {
    /// `max-width: 1050px`
    Narrow = 0,
    /// `max-width: 1250px`
    Medium = 1,
    Normal = 2,
    /// `min-width: 1600px`
    Wide = 3,
}

static WIDTH: AtomicU8 = AtomicU8::new(Width::Normal as u8);

pub fn update(window_width: f32) {
    let width = if window_width <= 1050. {
        Width::Narrow
    } else if window_width <= 1250. {
        Width::Medium
    } else if window_width >= 1600. {
        Width::Wide
    } else {
        Width::Normal
    };
    WIDTH.store(width as u8, Ordering::Relaxed);
}

pub fn width() -> Width {
    match WIDTH.load(Ordering::Relaxed) {
        0 => Width::Narrow,
        1 => Width::Medium,
        3 => Width::Wide,
        _ => Width::Normal,
    }
}

pub fn narrow() -> bool {
    width() == Width::Narrow
}

/// `max-width: 1250px` (medium and narrow).
pub fn compact() -> bool {
    width() <= Width::Medium
}

pub fn wide() -> bool {
    width() == Width::Wide
}

/// Pick a value per breakpoint: (narrow, medium, normal, wide).
pub fn pick<T: Copy>(values: (T, T, T, T)) -> T {
    match width() {
        Width::Narrow => values.0,
        Width::Medium => values.1,
        Width::Normal => values.2,
        Width::Wide => values.3,
    }
}
