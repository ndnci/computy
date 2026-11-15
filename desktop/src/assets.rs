//! Embedded assets: installed binaries never depend on the source checkout.
use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct Assets;

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

fn bytes(path: &str) -> Option<&'static [u8]> {
    Some(match path {
        "brand/logo-96.png" => include_bytes!("../assets/brand/logo-96.png"),
        "brand/logo-300.png" => include_bytes!("../assets/brand/logo-300.png"),
        "brand/logo-512.png" => include_bytes!("../assets/brand/logo-512.png"),
        _ => return icon(path),
    })
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        Ok(bytes(path).map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .map(|(name, _)| *name)
            .filter(|name| name.starts_with(path))
            .map(SharedString::from)
            .collect())
    }
}

pub const FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/DMSans-400.ttf"),
    include_bytes!("../assets/fonts/DMSans-450.ttf"),
    include_bytes!("../assets/fonts/DMSans-500.ttf"),
    include_bytes!("../assets/fonts/DMSans-550.ttf"),
    include_bytes!("../assets/fonts/DMSans-600.ttf"),
    include_bytes!("../assets/fonts/DMSans-650.ttf"),
    include_bytes!("../assets/fonts/DMSans-700.ttf"),
    include_bytes!("../assets/fonts/DMSans-720.ttf"),
    include_bytes!("../assets/fonts/DMSans-750.ttf"),
    include_bytes!("../assets/fonts/Sora-400.ttf"),
    include_bytes!("../assets/fonts/Sora-500.ttf"),
    include_bytes!("../assets/fonts/Sora-600.ttf"),
    include_bytes!("../assets/fonts/Sora-650.ttf"),
    include_bytes!("../assets/fonts/Sora-700.ttf"),
];
