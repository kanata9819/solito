use glyphon::{
    Attrs, Buffer, Cache, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas,
    TextRenderer, Viewport,
};
use tracing::debug;
use wgpu::MultisampleState;

pub struct InputBuffer {
    pub text_buffer: Buffer,
    pub viewport: Viewport,
    pub text_renderer: TextRenderer,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub atlas: TextAtlas,
    pub cursor: Cursor,
    inner_buffer: String,
}

impl InputBuffer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain_format: wgpu::TextureFormat,
        physical_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) -> Self {
        const FONT_SIZE: f32 = 20.0;
        const LINE_HEIGHT: f32 = 30.0;

        let mut font_system: FontSystem = FontSystem::new();
        let swash_cache: SwashCache = SwashCache::new();
        let cache: Cache = Cache::new(device);
        let viewport: Viewport = Viewport::new(device, &cache);
        let mut atlas: TextAtlas = TextAtlas::new(device, queue, &cache, swapchain_format);
        let renderer: TextRenderer =
            TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);
        let mut text_buffer: Buffer =
            Buffer::new(&mut font_system, Metrics::new(FONT_SIZE, LINE_HEIGHT));

        let physical_width: f32 = (f64::from(physical_size.width) * scale_factor) as f32;
        let physical_height: f32 = (f64::from(physical_size.height) * scale_factor) as f32;

        text_buffer.set_size(
            &mut font_system,
            Some(physical_width),
            Some(physical_height),
        );

        text_buffer.shape_until_scroll(&mut font_system, false);

        Self {
            text_buffer,
            text_renderer: renderer,
            font_system,
            viewport,
            swash_cache,
            atlas,
            inner_buffer: String::new(),
            cursor: Cursor::new(),
        }
    }

    pub fn set_text(&mut self, c: char) {
        self.inner_buffer.push(c);
        debug!("input buffer: {}", self.inner_buffer);

        self.text_buffer.set_text(
            &mut self.font_system,
            &self.inner_buffer,
            &Attrs::new().family(Family::Name("Cascadia Mono")),
            Shaping::Advanced,
            None,
        );
    }
}

pub struct Cursor {
    col: usize,
    row: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            col: usize::default(),
            row: usize::default(),
        }
    }

    pub fn forward_col(&mut self) {
        self.col += 1;
    }

    pub fn backward_col(&mut self) {
        self.col = self.col.saturating_sub(1);
    }

    pub fn forward_row(&mut self) {
        self.row += 1;
    }

    pub fn backward_row(&mut self) {
        self.row = self.row.saturating_sub(1);
    }

    pub fn reset_col(&mut self) {
        self.col = 0;
    }

    pub fn reset_row(&mut self) {
        self.row = 0;
    }
}
