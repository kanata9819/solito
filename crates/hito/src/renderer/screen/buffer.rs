use glyphon::{
    Attrs, Buffer, Family, FontSystem, Shaping, SwashCache, TextAtlas, TextRenderer, Viewport,
};

use super::cursor::Cursor;
use super::glyphon::GlyphonResources;

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
        let glyphon: GlyphonResources =
            GlyphonResources::new(device, queue, swapchain, physical_size, scale_factor);

        Self {
            text_buffer: glyphon.text_buffer,
            text_renderer: glyphon.text_renderer,
            font_system: glyphon.font_system,
            viewport: glyphon.viewport,
            swash_cache: glyphon.swash_cache,
            atlas: glyphon.atlas,
            inner_buffer: vec![Vec::new()],
            cursor: Cursor::new(),
        }
    }

    fn set_char_at_col(&mut self, c: char) {
        if let Some(line) = self.inner_buffer.get_mut(self.cursor.row())
            && let Some(cell) = line.get_mut(self.cursor.col())
        {
            *cell = c;
        }
    }

    fn set_text_to_buffer(&mut self) {
        // transform two-dimensional array to String
        let text: String = self.buffer_string();

        self.text_buffer.set_text(
            &mut self.font_system,
            text.as_ref(),
            &Attrs::new().family(Family::Name("Cascadia Mono")),
            Shaping::Advanced,
            None,
        );
    }

    fn buffer_string(&mut self) -> String {
        self.inner_buffer
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn ensure_line(&mut self) {
        self.ensure_row();
        self.ensure_col();
    }

    fn ensure_row(&mut self) {
        while self.inner_buffer.len() <= self.cursor.row() {
            self.inner_buffer.push(Vec::new());
        }
    }

    fn ensure_col(&mut self) {
        if let Some(line) = self.inner_buffer.get_mut(self.cursor.row()) {
            while line.len() <= self.cursor.col() {
                line.push(' ');
            }
        }
    }
}

pub(in crate::renderer) trait ScreenBufferEditor {
    fn push_char(&mut self, c: char);
    fn forward_col(&mut self);
    fn reset_col(&mut self);
    fn line_feed(&mut self);
    fn clear_line(&mut self);
    fn move_cursor_to(&mut self, row: u16, col: u16);
}

impl ScreenBufferEditor for InputBuffer {
    fn push_char(&mut self, c: char) {
        tracing::debug!(
            "print: char({:?}) row({:?}) col({:?})",
            c,
            self.cursor.row(),
            self.cursor.col()
        );

        self.ensure_line();
        self.set_char_at_col(c);
        self.set_text_to_buffer();
        self.forward_col();
    }

    fn forward_col(&mut self) {
        self.cursor.forward_col();
    }

    fn reset_col(&mut self) {
        self.cursor.reset_col();
    }

    fn line_feed(&mut self) {
        self.cursor.line_feed();
    }

    fn clear_line(&mut self) {
        self.ensure_line();
        self.inner_buffer[self.cursor.row()].clear();
        self.set_text_to_buffer();
    }

    fn move_cursor_to(&mut self, row: u16, col: u16) {
        self.cursor.move_to(row, col);
    }
}
