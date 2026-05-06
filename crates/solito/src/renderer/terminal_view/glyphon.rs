use ::glyphon::{
    Buffer, Cache, FontSystem, Metrics, SwashCache, TextAtlas, TextRenderer, Viewport,
};
use wgpu::MultisampleState;

use crate::config::BufferAttr;

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
            Metrics::new(BufferAttr::FONT_SIZE, BufferAttr::LINE_HEIGHT),
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
}
