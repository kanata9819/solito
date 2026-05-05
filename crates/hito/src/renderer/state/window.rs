use glyphon::{Color, Resolution, TextArea, TextBounds};
use std::{error::Error, sync::Arc};
use wgpu::{
    CommandEncoder, CommandEncoderDescriptor, Surface, SurfaceConfiguration, SurfaceTexture,
    TextureView, TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, window::Window};

use super::{context::State, gpu::GpuContext};
use crate::renderer::{
    pass,
    pipeline::rect::{self, Caret},
    screen::input_buffer::TerminalOutputHandler,
};

pub(super) struct WindowSurface {
    pub(super) surface: Surface<'static>,
    pub(super) config: SurfaceConfiguration,
    pub(super) is_configured: bool,
    pub(super) window: Arc<Window>,
}

impl WindowSurface {
    pub(super) fn new(
        window: Arc<Window>,
        surface: Surface<'static>,
        gpu: &GpuContext,
        size: PhysicalSize<u32>,
    ) -> Self {
        let surface_caps: wgpu::SurfaceCapabilities = surface.get_capabilities(&gpu.adapter);

        let surface_format: wgpu::TextureFormat = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config: SurfaceConfiguration = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        Self {
            surface,
            config,
            is_configured: false,
            window,
        }
    }
}

pub trait WindowRenderer {
    fn resize(&mut self, size: PhysicalSize<u32>);
    fn render(&mut self) -> Result<(), Box<dyn Error>>;
    fn redraw(&mut self) -> Result<(), Box<dyn Error>>;
    fn scroll(&mut self, x: f32, y: f32);
}

impl State {
    fn prepare_render(&mut self) -> Result<(), Box<dyn Error>> {
        self.buffer.glyphs.text_renderer.prepare(
            &self.gpu.device,
            &self.gpu.queue,
            &mut self.buffer.glyphs.font_system,
            &mut self.buffer.glyphs.atlas,
            &self.buffer.glyphs.viewport,
            [TextArea {
                buffer: &self.buffer.glyphs.text_buffer,
                left: 10.0,
                top: 10.0,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    ..Default::default()
                },
                default_color: Color::rgb(255, 255, 255),
                custom_glyphs: &[],
            }],
            &mut self.buffer.glyphs.swash_cache,
        )?;

        Ok(())
    }

    fn render_pass(
        &mut self,
        encoder: &mut CommandEncoder,
        view: TextureView,
    ) -> Result<(), Box<dyn Error>> {
        let mut pass: wgpu::RenderPass<'_> = pass::begin_render_pass(encoder, &view);

        self.buffer.glyphs.text_renderer.render(
            &self.buffer.glyphs.atlas,
            &self.buffer.glyphs.viewport,
            &mut pass,
        )?;

        let bind_group: wgpu::BindGroup = self
            .render_resources
            .rect_pipeline
            .caret_bind_group(&self.gpu.device, &self.render_resources.uniform_buffer);

        self.render_resources
            .rect_pipeline
            .draw_rect(&mut pass, bind_group);

        Ok(())
    }

    fn initialize_frame(&mut self) -> Result<Option<SurfaceTexture>, Box<dyn Error>> {
        let frame: SurfaceTexture = match self.window_surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                // Try again later
                self.window_surface.window.request_redraw();
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
                self.window_surface
                    .surface
                    .configure(&self.gpu.device, &self.window_surface.config);
                self.window_surface.window.request_redraw();
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.window_surface.surface = self
                    .gpu
                    .instance
                    .create_surface(self.window_surface.window.clone())?;
                self.window_surface
                    .surface
                    .configure(&self.gpu.device, &self.window_surface.config);
                self.window_surface.window.request_redraw();
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Validation => panic!("validation error"),
        };

        Ok(Some(frame))
    }

    fn create_encoder(&self, label: Option<&str>) -> CommandEncoder {
        self.gpu
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label })
    }
}

impl WindowRenderer for State {
    fn resize(&mut self, size: PhysicalSize<u32>) {
        let size: PhysicalSize<u32> = size;
        if size.width > 0 && size.height > 0 {
            self.window_surface.config.width = size.width;
            self.window_surface.config.height = size.height;
            self.window_surface
                .surface
                .configure(&self.gpu.device, &self.window_surface.config);
            self.window_surface.is_configured = true;

            rect::RectPipeline::update_caret_uniform(
                &self.render_resources.uniform_buffer,
                &self.gpu.queue,
                self.window_surface.window.inner_size().width,
                self.window_surface.window.inner_size().height,
            );

            self.buffer.resize(size.height);
        }
    }

    fn render(&mut self) -> Result<(), Box<dyn Error>> {
        self.window_surface.window.request_redraw();

        // We can't render unless the surface is configured
        if !self.window_surface.is_configured {
            return Ok(());
        }

        let output: SurfaceTexture = match self.window_surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.window_surface
                    .surface
                    .configure(&self.gpu.device, &self.window_surface.config);
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.window_surface
                    .surface
                    .configure(&self.gpu.device, &self.window_surface.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources
                // created with it here, but we'll just bail
                todo!("Lost device");
            }
        };

        let encoder: CommandEncoder = self.create_encoder(Some("Render Encoder"));

        self.gpu.queue.submit(std::iter::once(encoder.finish()));

        // The last lines of the code tell wgpu to finish the command buffer
        // and submit it to the GPU's render queue.
        output.present();

        Ok(())
    }

    fn redraw(&mut self) -> Result<(), Box<dyn Error>> {
        self.buffer.glyphs.viewport.update(
            &self.gpu.queue,
            Resolution {
                width: self.window_surface.config.width,
                height: self.window_surface.config.height,
            },
        );

        self.prepare_render()?;

        let frame: Option<SurfaceTexture> = match self.initialize_frame() {
            Ok(Some(frame)) => Some(frame),
            Ok(None) => None,
            Err(err) => {
                tracing::error!("initialize frame failed: {}", err);
                None
            }
        };

        if let Some(frame) = frame {
            let view: TextureView = frame.texture.create_view(&TextureViewDescriptor::default());
            let mut encoder: CommandEncoder = self.create_encoder(None);
            self.render_pass(&mut encoder, view)?;
            self.gpu.queue.submit(Some(encoder.finish()));
            frame.present();
        }

        self.buffer.glyphs.atlas.trim();

        Ok(())
    }

    fn scroll(&mut self, x: f32, y: f32) {
        self.buffer.scroll(x, y);
    }
}
