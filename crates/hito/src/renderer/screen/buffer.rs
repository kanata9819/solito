use glyphon::{
    Attrs, Buffer, Cache, Family, FontSystem, Metrics, Shaping, SwashCache, TextAtlas,
    TextRenderer, Viewport,
};
use wgpu::MultisampleState;

use super::cursor::Cursor;

const FONT_SIZE: f32 = 20.0;
const LINE_HEIGHT: f32 = 30.0;

pub struct InputBuffer {
    pub text_buffer: Buffer,
    pub viewport: Viewport,
    pub text_renderer: TextRenderer,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub atlas: TextAtlas,
    cursor: Cursor,
    inner_buffer: Vec<Vec<char>>,
}

impl InputBuffer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain: wgpu::TextureFormat,
        physical_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) -> Self {
        let mut font_system: FontSystem = FontSystem::new();
        let swash_cache: SwashCache = SwashCache::new();
        let cache: Cache = Cache::new(device);
        let viewport: Viewport = Viewport::new(device, &cache);
        let mut atlas: TextAtlas = TextAtlas::new(device, queue, &cache, swapchain);
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
            inner_buffer: vec![Vec::new()],
            cursor: Cursor::new(),
        }
    }

    pub fn set_text(&mut self, c: char) {
        self.ensure_row();

        // add char to current row
        self.inner_buffer[self.cursor.row()].push(c);

        // transform two-dimensional array to String
        let text: String = self
            .inner_buffer
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("\n");

        self.text_buffer.set_text(
            &mut self.font_system,
            text.as_ref(),
            &Attrs::new().family(Family::Name("Cascadia Mono")),
            Shaping::Advanced,
            None,
        );
    }

    pub fn ensure_row(&mut self) {
        while self.inner_buffer.len() <= self.cursor.row() {
            self.inner_buffer.push(Vec::new());
        }
    }

    pub fn forward_col(&mut self) {
        self.cursor.forward_col();
    }

    pub fn reset_col(&mut self) {
        self.cursor.reset_col();
    }

    pub fn line_feed(&mut self) {
        self.cursor.line_feed();
    }

    pub fn clear_line(&mut self) {
        self.ensure_row();
        self.inner_buffer[self.cursor.row()].clear();
    }

    pub fn move_cursor_to(&mut self, row: u16, col: u16) {
        self.cursor.move_to(row, col);
    }
}
