use anyhow::Result;
use glyphon::{Color, Resolution, TextArea, TextBounds};
use solito_terminal::ScreenSnapshot;
use wgpu::{
    CommandEncoder, CommandEncoderDescriptor, SurfaceTexture, TextureView, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;

use super::renderer::Renderer;
use crate::{pass, pipeline::rect, terminal_view::TerminalView, util::color::ThemeColor};

#[cfg(all(test, target_os = "windows"))]
mod preview;

impl Renderer {
    fn prepare_text(&mut self) -> Result<()> {
        // Several state updates can arrive before one redraw. Shape their
        // final text state once, immediately before glyph preparation.
        let text_changed = self.terminal_view.update_text_buffer();
        if !text_changed && !self.terminal_view.tab_bar.dirty {
            return Ok(());
        }

        let [default_r, default_g, default_b, _] = ThemeColor::TEXT;

        self.terminal_view.glyphs.text_renderer.prepare(
            &self.gpu.device,
            &self.gpu.queue,
            &mut self.terminal_view.glyphs.font_system,
            &mut self.terminal_view.glyphs.atlas,
            &self.terminal_view.glyphs.viewport,
            std::iter::once(TextArea {
                buffer: &self.terminal_view.glyphs.text_buffer,
                left: TerminalView::PADDING_X,
                top: TerminalView::terminal_row_y(0, self.terminal_view.config.line_height),
                scale: 1.0,
                bounds: TextBounds {
                    left: TerminalView::PADDING_X as i32,
                    top: TerminalView::terminal_row_y(0, self.terminal_view.config.line_height)
                        as i32,
                    right: self
                        .window_surface
                        .config
                        .width
                        .saturating_sub(TerminalView::PADDING_X as u32)
                        as i32,
                    bottom: self
                        .window_surface
                        .config
                        .height
                        .saturating_sub(TerminalView::PADDING_Y as u32)
                        as i32,
                },
                default_color: Color::rgb(default_r, default_g, default_b),
                custom_glyphs: &[],
            })
            .chain(self.terminal_view.tab_bar.text_areas()),
            &mut self.terminal_view.glyphs.swash_cache,
        )?;
        self.terminal_view.tab_bar.dirty = false;

        Ok(())
    }

    fn draw_to_view(&mut self, encoder: &mut CommandEncoder, view: &TextureView) -> Result<()> {
        let mut rects = self
            .terminal_view
            .background_rects(self.window_surface.config.format.is_srgb());
        rects.extend(
            self.terminal_view
                .tab_bar
                .rects(self.window_surface.config.format.is_srgb()),
        );
        rects.extend(self.terminal_view.copy_mode_rects());
        rects.extend(self.terminal_view.ime_rects());

        // Copy mode draws its own cursor over the scrollback. Hiding the shell
        // cursor here prevents two active cursors from appearing at once.
        if !self.terminal_view.copy_mode_active() {
            let (caret_x, caret_y, caret_w, caret_h) = self.terminal_view.caret_rect();

            if caret_w > 0.0 && caret_h > 0.0 {
                rects.push(rect::RectSpec::new(
                    caret_x,
                    caret_y,
                    caret_w,
                    caret_h,
                    self.terminal_view.caret_color(),
                ));
            }
        }

        self.render_resources
            .rect_pipeline
            .upload_rects(&self.gpu.device, &self.gpu.queue, &rects);

        let mut render_pass =
            pass::begin_render_pass(encoder, view, self.window_surface.clear_color);

        self.render_resources
            .rect_pipeline
            .draw_rects(&mut render_pass);

        self.terminal_view.glyphs.text_renderer.render(
            &self.terminal_view.glyphs.atlas,
            &self.terminal_view.glyphs.viewport,
            &mut render_pass,
        )?;

        Ok(())
    }

    fn acquire_frame(&mut self) -> Result<Option<SurfaceTexture>> {
        for _ in 0..2 {
            match self.window_surface.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => return Ok(Some(frame)),
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    // The old texture must be released before configuring again.
                    drop(frame);
                    self.reconfigure_surface();
                }
                wgpu::CurrentSurfaceTexture::Timeout => {
                    self.window_surface.window.request_redraw();
                    return Ok(None);
                }
                wgpu::CurrentSurfaceTexture::Occluded => return Ok(None),
                wgpu::CurrentSurfaceTexture::Outdated => self.reconfigure_surface(),
                wgpu::CurrentSurfaceTexture::Lost => {
                    self.window_surface.surface = self
                        .gpu
                        .instance
                        .create_surface(self.window_surface.window.clone())?;
                    self.reconfigure_surface();
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err(
                        std::io::Error::other("wgpu rejected the current surface texture").into(),
                    );
                }
            }
        }

        self.window_surface.window.request_redraw();
        Ok(None)
    }

    fn reconfigure_surface(&self) {
        self.window_surface
            .surface
            .configure(&self.gpu.device, &self.window_surface.config);
    }

    fn create_encoder(&self) -> CommandEncoder {
        self.gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Terminal frame encoder"),
            })
    }

    /// Resize the GPU surface and replace the terminal snapshot shown in it.
    pub fn resize(&mut self, window_size: PhysicalSize<u32>, snapshot: ScreenSnapshot) {
        if window_size.width == 0 || window_size.height == 0 {
            return;
        }

        self.window_surface.config.width = window_size.width;
        self.window_surface.config.height = window_size.height;
        self.reconfigure_surface();

        self.terminal_view
            .resize(window_size.width, window_size.height, snapshot);
        self.update_rect_screen_uniform();
    }

    /// Draw and present one frame.
    pub fn draw_frame(&mut self) -> Result<()> {
        // Keep pending text damage intact while the window cannot be drawn.
        let Some(frame) = self.acquire_frame()? else {
            return Ok(());
        };

        self.terminal_view.glyphs.viewport.update(
            &self.gpu.queue,
            Resolution {
                width: self.window_surface.config.width,
                height: self.window_surface.config.height,
            },
        );

        self.prepare_text()?;

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.create_encoder();
        self.draw_to_view(&mut encoder, &view)?;
        self.gpu.queue.submit(Some(encoder.finish()));
        self.gpu.queue.present(frame);

        self.terminal_view.glyphs.atlas.trim();
        Ok(())
    }

    pub fn scroll(&mut self, x: f32, y: f32) {
        self.terminal_view.scroll(x, y);
    }
}
