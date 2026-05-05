use glyphon::{Attrs, Family, Shaping};

use super::glyph_resources::GlyphResources;
use super::glyphon::GlyphonResources;
use super::screen_buffer::ScreenBuffer;
use super::viewport::ViewportState;

pub struct InputBuffer {
    pub(in crate::renderer) glyphs: GlyphResources,
    screen: ScreenBuffer,
    viewport: ViewportState,
}

impl InputBuffer {
    pub(in crate::renderer) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        swapchain: wgpu::TextureFormat,
        physical_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) -> Self {
        let glyphon: GlyphonResources =
            GlyphonResources::new(device, queue, swapchain, physical_size, scale_factor);

        Self {
            glyphs: GlyphResources::new(glyphon),
            screen: ScreenBuffer::new(),
            viewport: ViewportState::new(physical_size.height),
        }
    }

    pub(in crate::renderer) fn resize(&mut self, height: u32) {
        self.viewport.resize(height, self.screen.row_count());
        self.set_text_to_buffer();
    }

    fn set_text_to_buffer(&mut self) {
        let text: String = self.screen.visible_text(&mut self.viewport);

        self.glyphs.text_buffer.set_text(
            &mut self.glyphs.font_system,
            text.as_ref(),
            &Attrs::new().family(Family::Name("Cascadia Mono")),
            Shaping::Advanced,
            None,
        );
    }
}

pub trait TerminalOutputHandler {
    fn push_char(&mut self, c: char);
    fn reset_col(&mut self);
    fn line_feed(&mut self);
    fn clear_line(&mut self);
    fn move_cursor_to(&mut self, row: u16, col: u16);
    fn scroll(&mut self, x: f32, y: f32);
}

impl TerminalOutputHandler for InputBuffer {
    fn push_char(&mut self, c: char) {
        tracing::debug!(
            "print: char({:?}) row({:?}) col({:?})",
            c,
            self.screen.cursor_row(),
            self.screen.cursor_col()
        );

        self.viewport.reset();
        self.screen.push_char(c);
        self.set_text_to_buffer();
    }

    fn reset_col(&mut self) {
        self.screen.reset_col();
    }

    fn line_feed(&mut self) {
        self.viewport.reset();
        self.screen.line_feed();
        self.set_text_to_buffer();
    }

    fn clear_line(&mut self) {
        self.viewport.reset();
        self.screen.clear_line();
        self.set_text_to_buffer();
    }

    fn move_cursor_to(&mut self, row: u16, col: u16) {
        self.screen.move_cursor_to(row, col);
    }

    fn scroll(&mut self, _x: f32, y: f32) {
        self.viewport.scroll(y, self.screen.row_count());
        self.set_text_to_buffer();
    }
}
