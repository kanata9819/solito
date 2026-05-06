use ::glyphon::{Buffer, FontSystem, SwashCache, TextAtlas, TextRenderer, Viewport};

use super::glyphon::GlyphonResources;

pub(crate) struct GlyphResources {
    pub(crate) text_buffer: Buffer,
    pub(crate) viewport: Viewport,
    pub(crate) text_renderer: TextRenderer,
    pub(crate) font_system: FontSystem,
    pub(crate) swash_cache: SwashCache,
    pub(crate) atlas: TextAtlas,
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
