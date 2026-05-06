use glyphon::{Buffer, FontSystem, SwashCache, TextAtlas, TextRenderer, Viewport};

use super::glyphon::GlyphonResources;

pub(in crate::renderer) struct GlyphResources {
    pub(in crate::renderer) text_buffer: Buffer,
    pub(in crate::renderer) viewport: Viewport,
    pub(in crate::renderer) text_renderer: TextRenderer,
    pub(in crate::renderer) font_system: FontSystem,
    pub(in crate::renderer) swash_cache: SwashCache,
    pub(in crate::renderer) atlas: TextAtlas,
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
