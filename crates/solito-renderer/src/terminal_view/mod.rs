use ::glyphon::{Attrs, Family, Shaping};
use solito_terminal::{ScreenCell, ScreenSnapshot};

use crate::RendererConfig;

mod glyphon;
mod resources;
mod viewport;

use self::glyphon::GlyphonResources;
use self::resources::GlyphResources;
use self::viewport::ViewportState;

pub(crate) struct TerminalView {
    pub(crate) glyphs: GlyphResources,
    snapshot: ScreenSnapshot,
    viewport: ViewportState,
}

impl TerminalView {
    pub(crate) fn new(
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
            snapshot: ScreenSnapshot::default(),
            viewport: ViewportState::new(physical_size.height),
        }
    }

    pub(crate) fn resize(&mut self, height: u32, snapshot: ScreenSnapshot) {
        self.snapshot = snapshot;
        self.viewport.resize(height, self.row_count());
        self.set_text_to_buffer();
    }

    pub(crate) fn set_snapshot(&mut self, snapshot: ScreenSnapshot) {
        self.viewport.reset();
        self.snapshot = snapshot;
        self.set_text_to_buffer();
    }

    pub(crate) fn scroll(&mut self, _x: f32, y: f32) {
        self.viewport.scroll(y, self.row_count());
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
        if self.snapshot.lines.is_empty() {
            return String::new();
        }

        let row_count: usize = self.row_count();
        self.viewport.clamp(row_count);
        let (start, end): (usize, usize) = self.viewport.visible_range(row_count);

        self.snapshot.lines[start..end]
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
        self.snapshot.lines.len().max(1)
    }

    fn cell_char(cell: &ScreenCell) -> char {
        cell.ch
    }

    pub(crate) fn visible_cols(width: u32) -> usize {
        let cell_width: f32 = (RendererConfig::FONT_SIZE * 0.62).max(1.0);
        ((width as f32 / cell_width).floor() as usize).max(1)
    }

    pub(crate) fn visible_rows(height: u32) -> usize {
        ((height as f32 / RendererConfig::LINE_HEIGHT).floor() as usize).max(1)
    }
}
