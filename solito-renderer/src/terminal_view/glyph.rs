use ::glyphon::{
    Attrs, Buffer, Cache, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas,
    TextRenderer, Viewport, Wrap,
};
use std::collections::HashMap;
use wgpu::MultisampleState;

use crate::RendererConfig;

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
    ) -> Self {
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, swapchain);
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

    pub(super) fn measure_font_width(
        font_system: &mut FontSystem,
        config: &RendererConfig,
    ) -> f32 {
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
