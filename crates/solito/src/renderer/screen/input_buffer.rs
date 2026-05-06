use glyphon::{Attrs, Family, Shaping};

use crate::{
    config::BufferAttr,
    terminal::{ScreenCell, TerminalState},
};

use super::glyph_resources::GlyphResources;
use super::glyphon::GlyphonResources;
use super::viewport::ViewportState;

pub(in crate::renderer) struct InputBuffer {
    pub glyphs: GlyphResources,
    terminal: TerminalState,
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
        let cols: usize = Self::visible_cols(physical_size.width);
        let rows: usize = Self::visible_rows(physical_size.height);

        Self {
            glyphs: GlyphResources::new(glyphon),
            terminal: TerminalState::new(cols, rows),
            viewport: ViewportState::new(physical_size.height),
        }
    }

    pub(in crate::renderer) fn resize(&mut self, width: u32, height: u32) {
        self.terminal.set_width(Self::visible_cols(width));
        self.terminal.set_height(Self::visible_rows(height));
        self.viewport.resize(height, self.row_count());
        self.set_text_to_buffer();
    }

    fn set_text_to_buffer(&mut self) {
        let text: String = self.visible_text();

        self.glyphs.text_buffer.set_text(
            &mut self.glyphs.font_system,
            text.as_ref(),
            &Attrs::new().family(Family::Name("Cascadia Mono")),
            Shaping::Advanced,
            None,
        );
    }

    fn visible_text(&mut self) -> String {
        let snapshot = self.terminal.snapshot();
        let row_count = snapshot.lines.len().max(1);
        self.viewport.clamp(row_count);
        let (start, end) = self.viewport.visible_range(row_count);

        snapshot.lines[start..end]
            .iter()
            .map(|line| {
                line.iter()
                    .filter(|cell| !cell.is_wide_continuation)
                    .map(Self::cell_char)
                    .collect::<String>()
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn row_count(&self) -> usize {
        self.terminal.snapshot().lines.len().max(1)
    }

    fn cell_char(cell: &ScreenCell) -> char {
        cell.ch
    }

    fn visible_cols(width: u32) -> usize {
        let cell_width = (BufferAttr::FONT_SIZE * 0.62).max(1.0);
        ((width as f32 / cell_width).floor() as usize).max(1)
    }

    fn visible_rows(height: u32) -> usize {
        ((height as f32 / BufferAttr::LINE_HEIGHT).floor() as usize).max(1)
    }
}

pub(in crate::renderer) trait TerminalOutputHandler {
    fn apply_terminal_output(&mut self, bytes: &[u8]);
    fn scroll(&mut self, x: f32, y: f32);
}

impl TerminalOutputHandler for InputBuffer {
    fn apply_terminal_output(&mut self, bytes: &[u8]) {
        self.viewport.reset();
        self.terminal.apply_terminal_output(bytes);
        self.set_text_to_buffer();
    }

    fn scroll(&mut self, _x: f32, y: f32) {
        self.viewport.scroll(y, self.row_count());
        self.set_text_to_buffer();
    }
}
