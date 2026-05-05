use glyphon::{Buffer, FontSystem, SwashCache, TextAtlas, TextRenderer, Viewport};

use super::glyphon::GlyphonResources;

pub(in crate::renderer) struct GlyphResources {
    pub text_buffer: Buffer,
    pub viewport: Viewport,
    pub text_renderer: TextRenderer,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub atlas: TextAtlas,
}

impl GlyphResources {
    pub(super) fn new(glyphon: GlyphonResources) -> Self {
        Self {
            text_buffer: glyphon.text_buffer,
            text_renderer: glyphon.text_renderer,
            font_system: glyphon.font_system,
            viewport: glyphon.viewport,
            swash_cache: glyphon.swash_cache,
            atlas: glyphon.atlas,
        }
    }
}
