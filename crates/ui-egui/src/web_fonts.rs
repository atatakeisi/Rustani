//! Optional Japanese browser fallback from an external craft-fonts checkout.
//!
//! build.rs embeds only BIZ UDPGothic Regular and its OFL text for wasm. No font assets live in
//! this repository. This is Japanese coverage, not a promise of full Chinese/Korean support.

#[cfg(any(target_arch = "wasm32", test))]
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
include!(concat!(env!("OUT_DIR"), "/web_fonts.rs"));

#[cfg(any(target_arch = "wasm32", test))]
const FONT_KEY: &str = "web-japanese-biz-udpgothic";

/// Whether this build supplies a validated Japanese fallback. Native builds return false and
/// keep their existing system-font behavior. A missing optional checkout also returns false.
pub fn japanese_available() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        japanese_data().is_some()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// The embedded font's OFL text, for an About/licences view. Distributions that embed the font
/// must also carry its licence (including builds using the sibling-checkout discovery).
pub fn japanese_licence() -> Option<&'static str> {
    #[cfg(target_arch = "wasm32")]
    {
        japanese_available().then_some(JAPANESE_FONT_LICENCE)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

#[cfg(target_arch = "wasm32")]
fn japanese_data() -> Option<Arc<egui::FontData>> {
    static FONT: std::sync::OnceLock<Option<Arc<egui::FontData>>> = std::sync::OnceLock::new();
    FONT.get_or_init(|| {
        let data = validate_japanese(JAPANESE_FONT_BYTES);
        if !JAPANESE_FONT_BYTES.is_empty() && data.is_none() {
            log::warn!("ignoring optional Web Japanese font: invalid font or missing Japanese glyphs");
        }
        data
    })
    .clone()
}

#[cfg(any(target_arch = "wasm32", test))]
fn validate_japanese(bytes: &'static [u8]) -> Option<Arc<egui::FontData>> {
    use skrifa::MetadataProvider as _;
    if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 || !crate::theme::font_parses(bytes, 0) {
        return None;
    }
    let font = skrifa::FontRef::from_index(bytes, 0).ok()?;
    let map = font.charmap();
    if !['日', '本', '語', 'あ', 'ア', '漢', '字'].into_iter().all(|c| map.map(c).is_some_and(|g| g.to_u32() != 0)) {
        return None;
    }
    Some(Arc::new(egui::FontData::from_static(bytes)))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn append_fallback(fonts: &mut egui::FontDefinitions) {
    append_data(fonts, japanese_data());
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn append_fallback(_fonts: &mut egui::FontDefinitions) {}

#[cfg(any(target_arch = "wasm32", test))]
fn append_data(fonts: &mut egui::FontDefinitions, data: Option<Arc<egui::FontData>>) {
    let Some(data) = data else { return };
    fonts.font_data.insert(FONT_KEY.into(), data);
    for chain in fonts.families.values_mut() {
        if !chain.iter().any(|key| key == FONT_KEY) {
            chain.push(FONT_KEY.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{FontData, FontDefinitions, FontFamily};

    #[test]
    fn missing_invalid_and_latin_only_fonts_cannot_enable_japanese() {
        assert!(validate_japanese(&[]).is_none());
        assert!(validate_japanese(b"not a font").is_none());
        assert!(validate_japanese(epaint_default_fonts::UBUNTU_LIGHT).is_none());
        let mut fonts = FontDefinitions::default();
        let before = fonts.clone();
        append_data(&mut fonts, None);
        assert_eq!(fonts, before);
    }

    #[test]
    fn fallback_survives_rebuilt_definitions_for_every_text_role() {
        let data = Arc::new(FontData::from_static(epaint_default_fonts::UBUNTU_LIGHT));
        // A small valid font tests chain installation without requiring the optional checkout.
        // Production supplies data only after the Japanese-coverage validation above.
        for _rebuild in 0..3 {
            let mut fonts = FontDefinitions::default();
            let base = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
            for role in [
                crate::theme::UI,
                crate::theme::UI_BOLD,
                crate::theme::CELL,
                crate::theme::CELL_BOLD,
                crate::theme::CELL_ITALIC,
                crate::theme::CELL_BOLD_ITALIC,
                crate::theme::SERIF,
                crate::theme::MONO,
                "workbook-custom-font",
            ] {
                fonts.families.insert(FontFamily::Name(role.into()), base.clone());
            }
            let before = fonts.families.clone();
            append_data(&mut fonts, Some(Arc::clone(&data)));
            append_data(&mut fonts, Some(Arc::clone(&data)));
            assert!(fonts.font_data.get(FONT_KEY).is_some_and(|installed| Arc::ptr_eq(installed, &data)), "rebuilds share the original font bytes");
            for (family, original) in before {
                let chain = fonts.families.get(&family).expect("family retained");
                assert_eq!(chain.last().map(String::as_str), Some(FONT_KEY), "{family:?} has the fallback");
                assert_eq!(&chain[..chain.len() - 1], original, "{family:?} preserves its Latin-first order without duplicate fallback entries");
            }
        }
    }
}
