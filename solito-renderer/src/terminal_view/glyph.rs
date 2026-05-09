use ::glyphon::{
    Attrs, Buffer, Cache, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas,
    TextRenderer, Viewport, Wrap,
};
use wgpu::MultisampleState;

use crate::RendererConfig;

pub(super) struct GlyphonResources {
    pub(super) text_buffer: Buffer,
    pub(super) text_renderer: TextRenderer,
    pub(super) font_system: FontSystem,
    pub(super) viewport: Viewport,
    pub(super) swash_cache: SwashCache,
    pub(super) atlas: TextAtlas,
}

impl GlyphonResources {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain: wgpu::TextureFormat,
        physical_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) -> GlyphonResources {
        let mut font_system: FontSystem = FontSystem::new();
        let swash_cache: SwashCache = SwashCache::new();
        let cache: Cache = Cache::new(device);
        let viewport: Viewport = Viewport::new(device, &cache);
        let mut atlas: TextAtlas = TextAtlas::new(device, queue, &cache, swapchain);
        let text_renderer: TextRenderer =
            TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);
        let mut text_buffer: Buffer = Buffer::new(
            &mut font_system,
            Metrics::new(RendererConfig::FONT_SIZE, RendererConfig::LINE_HEIGHT),
        );

        let physical_width: f32 = (f64::from(physical_size.width) * scale_factor) as f32;
        let physical_height: f32 = (f64::from(physical_size.height) * scale_factor) as f32;

        text_buffer.set_size(
            &mut font_system,
            Some(physical_width),
            Some(physical_height),
        );

        text_buffer.shape_until_scroll(&mut font_system, false);

        GlyphonResources {
            text_buffer,
            text_renderer,
            font_system,
            viewport,
            swash_cache,
            atlas,
        }
    }

    pub(super) fn measure_font_width(font_system: &mut glyphon::FontSystem) -> f32 {
        let mut buffer: Buffer = Buffer::new(
            font_system,
            Metrics::new(RendererConfig::FONT_SIZE, RendererConfig::LINE_HEIGHT),
        );

        buffer.set_wrap(font_system, Wrap::None);

        buffer.set_text(
            font_system,
            "M",
            &Attrs::new().family(Family::Name("Cascadia Mono")),
            Shaping::Advanced,
            None,
        );

        buffer
            .layout_runs()
            .next()
            .map(|run| run.line_w)
            .unwrap_or(RendererConfig::FONT_SIZE * 0.62)
            .max(1.0)
    }
}
