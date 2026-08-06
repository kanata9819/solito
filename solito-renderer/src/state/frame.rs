//! One rendered frame: prepare text, draw rectangles and glyphs, then present.

use glyphon::{Color, Resolution, TextArea, TextBounds};
use solito_terminal::ScreenSnapshot;
use std::error::Error;
use wgpu::{
    CommandEncoder, CommandEncoderDescriptor, SurfaceTexture, TextureView, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;

use super::renderer::Renderer;
use crate::{pass, pipeline::rect, terminal_view::TerminalView, util::color::ThemeColor};

impl Renderer {
    fn prepare_text(&mut self) -> Result<(), Box<dyn Error>> {
        let [default_r, default_g, default_b, _] = ThemeColor::WHITE;

        self.terminal_view.glyphs.text_renderer.prepare(
            &self.gpu.device,
            &self.gpu.queue,
            &mut self.terminal_view.glyphs.font_system,
            &mut self.terminal_view.glyphs.atlas,
            &self.terminal_view.glyphs.viewport,
            [TextArea {
                buffer: &self.terminal_view.glyphs.text_buffer,
                left: TerminalView::PADDING_X,
                top: TerminalView::PADDING_Y,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    ..Default::default()
                },
                default_color: Color::rgb(default_r, default_g, default_b),
                custom_glyphs: &[],
            }],
            &mut self.terminal_view.glyphs.swash_cache,
        )?;

        Ok(())
    }

    fn draw_to_view(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
    ) -> Result<(), Box<dyn Error>> {
        self.update_rect_screen_uniform();

        let mut rects = self.terminal_view.tab_bar_rects();
        rects.extend(self.terminal_view.copy_mode_rects());

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

        let rect_instance_buffer =
            rect::RectPipeline::create_instance_buffer(&self.gpu.device, &rects);

        let rect_bind_group = self
            .render_resources
            .rect_pipeline
            .rect_bind_group(&self.gpu.device, &self.render_resources.uniform_buffer);

        let mut render_pass =
            pass::begin_render_pass(encoder, view, self.window_surface.clear_color);

        if let Some(rect_instance_buffer) = rect_instance_buffer.as_ref() {
            self.render_resources.rect_pipeline.draw_rects(
                &mut render_pass,
                &rect_bind_group,
                rect_instance_buffer,
                rects.len(),
            );
        }

        self.terminal_view.glyphs.text_renderer.render(
            &self.terminal_view.glyphs.atlas,
            &self.terminal_view.glyphs.viewport,
            &mut render_pass,
        )?;

        Ok(())
    }

    fn acquire_frame(&mut self) -> Result<Option<SurfaceTexture>, Box<dyn Error>> {
        for _ in 0..2 {
            match self.window_surface.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => return Ok(Some(frame)),
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    self.reconfigure_surface();
                    return Ok(Some(frame));
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    self.window_surface.window.request_redraw();
                    return Ok(None);
                }
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
    pub fn draw_frame(&mut self) -> Result<(), Box<dyn Error>> {
        self.terminal_view.glyphs.viewport.update(
            &self.gpu.queue,
            Resolution {
                width: self.window_surface.config.width,
                height: self.window_surface.config.height,
            },
        );

        self.prepare_text()?;

        let frame = match self.acquire_frame() {
            Ok(frame) => frame,
            Err(err) => {
                tracing::error!("acquire frame failed: {err}");
                None
            }
        };

        if let Some(frame) = frame {
            let view = frame.texture.create_view(&TextureViewDescriptor::default());
            let mut encoder = self.create_encoder();
            self.draw_to_view(&mut encoder, &view)?;
            self.gpu.queue.submit(Some(encoder.finish()));
            self.gpu.queue.present(frame);
        }

        self.terminal_view.glyphs.atlas.trim();
        Ok(())
    }

    pub fn scroll(&mut self, x: f32, y: f32) {
        self.terminal_view.scroll(x, y);
    }
}
