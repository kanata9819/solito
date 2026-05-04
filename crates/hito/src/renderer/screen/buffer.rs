use glyphon::{
    Attrs, Buffer, Family, FontSystem, Shaping, SwashCache, TextAtlas, TextRenderer, Viewport,
};

use crate::config::BufferAttr;

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
    scroll_offset: usize,
    scroll_accumulator: f32,
    visible_rows: usize,
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
            text_buffer: glyphon.text_buffer,
            text_renderer: glyphon.text_renderer,
            font_system: glyphon.font_system,
            viewport: glyphon.viewport,
            swash_cache: glyphon.swash_cache,
            atlas: glyphon.atlas,
            inner_buffer: vec![Vec::new()],
            cursor: Cursor::new(),
            scroll_offset: 0,
            scroll_accumulator: 0.0,
            visible_rows: Self::visible_rows(physical_size.height),
        }
    }

    pub(in crate::renderer) fn resize(&mut self, height: u32) {
        self.visible_rows = Self::visible_rows(height);
        self.clamp_scroll_offset();
        self.set_text_to_buffer();
    }

    fn visible_rows(height: u32) -> usize {
        // Calculates how many full rows can fit on the screen
        // by dividing the screen height by the line height.
        ((height as f32 / BufferAttr::LINE_HEIGHT).floor() as usize).max(1)
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
        self.clamp_scroll_offset();

        let end: usize = self.inner_buffer.len().saturating_sub(self.scroll_offset);
        let start: usize = end.saturating_sub(self.visible_rows);

        self.inner_buffer[start..end]
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn max_scroll_offset(&self) -> usize {
        let row_count: usize = self.inner_buffer.len();
        row_count.saturating_sub(self.visible_rows)
    }

    fn clamp_scroll_offset(&mut self) {
        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
    }

    fn reset_scroll(&mut self) {
        self.scroll_offset = 0;
        self.scroll_accumulator = 0.0;
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

pub trait ScreenBufferEditor {
    fn push_char(&mut self, c: char);
    fn forward_col(&mut self);
    fn reset_col(&mut self);
    fn line_feed(&mut self);
    fn clear_line(&mut self);
    fn move_cursor_to(&mut self, row: u16, col: u16);
    fn scroll(&mut self, x: f32, y: f32);
}

impl ScreenBufferEditor for InputBuffer {
    fn push_char(&mut self, c: char) {
        tracing::debug!(
            "print: char({:?}) row({:?}) col({:?})",
            c,
            self.cursor.row(),
            self.cursor.col()
        );

        self.reset_scroll();
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
        self.reset_scroll();
        self.cursor.line_feed();
        self.ensure_row();
        self.set_text_to_buffer();
    }

    fn clear_line(&mut self) {
        self.reset_scroll();
        self.ensure_line();
        // ESC[K with mode 0 erases from the cursor to the end of the line.
        self.inner_buffer[self.cursor.row()].truncate(self.cursor.col());
        self.set_text_to_buffer();
    }

    fn move_cursor_to(&mut self, row: u16, col: u16) {
        self.cursor.move_to(row, col);
    }

    fn scroll(&mut self, _x: f32, y: f32) {
        self.scroll_accumulator += y;

        while self.scroll_accumulator >= 1.0 {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
            self.scroll_accumulator -= 1.0;
        }

        while self.scroll_accumulator <= -1.0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
            self.scroll_accumulator += 1.0;
        }

        self.clamp_scroll_offset();
        self.set_text_to_buffer();
    }
}
