use ::glyphon::{
    Attrs, Buffer, Cache, ColorMode, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas,
    TextRenderer, Viewport, Wrap,
};
use std::collections::HashMap;
use wgpu::MultisampleState;

use crate::RendererConfig;

pub(crate) fn new_font_system() -> FontSystem {
    let fonts = FontSystem::new();
    #[cfg(target_os = "windows")]
    {
        if is_japanese_locale(fonts.locale()) {
            let (locale, db) = fonts.into_locale_and_db();
            return FontSystem::new_with_locale_and_db_and_fallback(locale, db, JapaneseFallback);
        }
    }
    fonts
}

#[cfg(target_os = "windows")]
struct JapaneseFallback;

#[cfg(target_os = "windows")]
impl glyphon::cosmic_text::Fallback for JapaneseFallback {
    fn common_fallback(&self) -> &[&'static str] {
        use glyphon::cosmic_text::PlatformFallback;
        PlatformFallback.common_fallback()
    }

    fn forbidden_fallback(&self) -> &[&'static str] {
        use glyphon::cosmic_text::PlatformFallback;
        PlatformFallback.forbidden_fallback()
    }

    fn script_fallback(&self, script: unicode_script::Script, locale: &str) -> &[&'static str] {
        use glyphon::cosmic_text::PlatformFallback;
        use unicode_script::Script;

        if is_japanese_locale(locale)
            && matches!(script, Script::Han | Script::Hiragana | Script::Katakana)
        {
            &["Meiryo", "Yu Gothic"]
        } else {
            PlatformFallback.script_fallback(script, locale)
        }
    }
}

#[cfg(target_os = "windows")]
fn is_japanese_locale(locale: &str) -> bool {
    locale.eq_ignore_ascii_case("ja")
        || locale.get(..3).is_some_and(|prefix| {
            prefix.eq_ignore_ascii_case("ja-") || prefix.eq_ignore_ascii_case("ja_")
        })
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn recognizes_japanese_locale_variants() {
        for locale in ["ja", "ja-JP", "JA_jp"] {
            assert!(is_japanese_locale(locale), "{locale}");
        }
        for locale in ["en-US", "zh-JP", "japanese"] {
            assert!(!is_japanese_locale(locale), "{locale}");
        }
    }

    #[test]
    fn japanese_fallback_uses_meiryo_for_kana_and_han() {
        let (_, db) = FontSystem::new().into_locale_and_db();
        if !db
            .faces()
            .any(|face| face.families.iter().any(|(name, _)| name == "Meiryo"))
        {
            return;
        }
        let mut fonts = FontSystem::new_with_locale_and_db_and_fallback(
            "ja-JP".to_owned(),
            db,
            JapaneseFallback,
        );
        let mut buffer = Buffer::new(&mut fonts, Metrics::new(20.0, 30.0));
        buffer.set_text(
            "日本語あいう",
            &Attrs::new().family(Family::Name("Cascadia Mono")),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut fonts, false);
        let run = buffer.layout_runs().next().unwrap();
        assert_eq!(run.glyphs.len(), 6);
        for glyph in run.glyphs {
            let face = fonts.db().face(glyph.font_id).unwrap();
            assert!(
                face.families.iter().any(|(name, _)| name == "Meiryo"),
                "{:?}",
                face.families
            );
        }
    }

    #[test]
    fn preserves_platform_fallbacks_for_other_scripts_and_locales() {
        use glyphon::cosmic_text::{Fallback, PlatformFallback};
        use unicode_script::Script;

        for (script, locale) in [
            (Script::Han, "zh-CN"),
            (Script::Hiragana, "en-US"),
            (Script::Hangul, "ja-JP"),
            (Script::Arabic, "ja-JP"),
        ] {
            assert_eq!(
                JapaneseFallback.script_fallback(script, locale),
                PlatformFallback.script_fallback(script, locale)
            );
        }
        assert_eq!(
            JapaneseFallback.common_fallback(),
            PlatformFallback.common_fallback()
        );
        assert_eq!(
            JapaneseFallback.forbidden_fallback(),
            PlatformFallback.forbidden_fallback()
        );
    }

    #[test]
    fn explicit_japanese_font_takes_precedence() {
        let (_, db) = FontSystem::new().into_locale_and_db();
        if !db
            .faces()
            .any(|face| face.families.iter().any(|(name, _)| name == "Yu Gothic"))
        {
            return;
        }
        let mut fonts = FontSystem::new_with_locale_and_db_and_fallback(
            "ja-JP".to_owned(),
            db,
            JapaneseFallback,
        );
        let mut buffer = Buffer::new(&mut fonts, Metrics::new(20.0, 30.0));
        buffer.set_text(
            "日本語あいう",
            &Attrs::new().family(Family::Name("Yu Gothic")),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut fonts, false);
        for glyph in buffer.layout_runs().next().unwrap().glyphs {
            let face = fonts.db().face(glyph.font_id).unwrap();
            assert!(
                face.families.iter().any(|(name, _)| name == "Yu Gothic"),
                "{:?}",
                face.families
            );
        }
    }
}

pub(crate) struct GlyphonResources {
    pub(crate) text_buffer: Buffer,
    pub(crate) text_renderer: TextRenderer,
    pub(crate) font_system: FontSystem,
    pub(crate) viewport: Viewport,
    pub(crate) swash_cache: SwashCache,
    pub(crate) atlas: TextAtlas,
    pub(super) cell_width: f32,
    pub(super) glyph_widths: HashMap<char, f32>,
}

impl GlyphonResources {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain: wgpu::TextureFormat,
        config: &RendererConfig,
        mut font_system: FontSystem,
    ) -> Self {
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        // Blend coverage in encoded sRGB, as UI toolkits do. Linear-light
        // blending makes pale antialiased edges too bright on dark backgrounds.
        let mut atlas = TextAtlas::with_color_mode(
            device,
            queue,
            &cache,
            swapchain.remove_srgb_suffix(),
            ColorMode::Web,
        );
        let text_renderer =
            TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);
        let mut text_buffer = Buffer::new(
            &mut font_system,
            Metrics::new(config.font_size, config.line_height),
        );
        text_buffer.set_wrap(Wrap::None);
        let cell_width = Self::measure_font_width(&mut font_system, config).max(1.0);

        Self {
            text_buffer,
            text_renderer,
            font_system,
            viewport,
            swash_cache,
            atlas,
            cell_width,
            glyph_widths: HashMap::new(),
        }
    }

    pub(super) fn measure_font_width(font_system: &mut FontSystem, config: &RendererConfig) -> f32 {
        Self::measure_text_width(font_system, config, "M")
    }

    pub(super) fn measure_text_width(
        font_system: &mut FontSystem,
        config: &RendererConfig,
        text: &str,
    ) -> f32 {
        let mut buffer = Buffer::new(
            font_system,
            Metrics::new(config.font_size, config.line_height),
        );

        buffer.set_wrap(Wrap::None);

        buffer.set_text(
            text,
            &Attrs::new().family(Family::Name(config.font_family.as_str())),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(font_system, false);

        buffer
            .layout_runs()
            .next()
            .map(|run| run.line_w)
            .unwrap_or(config.font_size * 0.62)
            .max(1.0)
    }
}
