use glyphon::{Color, Resolution, TextArea, TextBounds};
use std::{error::Error, sync::Arc};
use wgpu::{
    Adapter, CommandEncoder, CommandEncoderDescriptor, Device, Instance, Queue, Surface,
    SurfaceConfiguration, SurfaceTexture, TextureFormat, TextureView, TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::renderer::{pass, pipeline::rect, screen::buffer::InputBuffer};

pub struct State {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    is_surface_configured: bool,
    rect_pipeline: rect::RectPipeline,
    buffer: InputBuffer,
    instance: Instance,
    window: Arc<Window>,
    uniform_buffer: wgpu::Buffer,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let size: PhysicalSize<u32> = window.inner_size();
        let instance: Instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface: Surface<'_> = instance.create_surface(window.clone())?;
        let adapter: Adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps: wgpu::SurfaceCapabilities = surface.get_capabilities(&adapter);

        let surface_format: wgpu::TextureFormat = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config: wgpu::wgt::SurfaceConfiguration<Vec<wgpu::TextureFormat>> =
            wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: size.width,
                height: size.height,
                present_mode: surface_caps.present_modes[0],
                alpha_mode: surface_caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };

        let uniform_buffer: wgpu::Buffer = rect::RectPipeline::create_uniform_buffer(&device);
        let rect_pipeline: rect::RectPipeline = rect::RectPipeline::new(
            &device,
            config.clone(),
            &queue,
            &uniform_buffer,
            size.width,
            size.height,
        );
        let swapchain_format: TextureFormat = TextureFormat::Bgra8UnormSrgb;
        let buffer: InputBuffer =
            InputBuffer::new(&device.clone(), &queue.clone(), swapchain_format, size, 1.0);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            rect_pipeline,
            buffer,
            instance,
            window,
            uniform_buffer,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let size: PhysicalSize<u32> = size;
        if size.width > 0 && size.height > 0 {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;

            rect::RectPipeline::update_caret_uniform(
                &self.uniform_buffer,
                &self.queue,
                self.window.inner_size().width,
                self.window.inner_size().height,
            );
        }
    }

    pub fn render(&mut self) -> Result<(), Box<dyn Error>> {
        self.window.request_redraw();

        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        let output: SurfaceTexture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.surface.configure(&self.device, &self.config);
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources
                // created with it here, but we'll just bail
                todo!("Lost device");
            }
        };

        let encoder: CommandEncoder = self.create_encoder(Some("Render Encoder"));

        self.queue.submit(std::iter::once(encoder.finish()));

        // The last lines of the code tell wgpu to finish the command buffer
        // and submit it to the GPU's render queue.
        output.present();

        Ok(())
    }

    pub fn redraw(&mut self) -> Result<(), Box<dyn Error>> {
        self.buffer.viewport.update(
            &self.queue,
            Resolution {
                width: self.config.width,
                height: self.config.height,
            },
        );

        self.prepare()?;

        let frame: Option<SurfaceTexture> = match self.initialize_frame() {
            Ok(Some(frame)) => Some(frame),
            Ok(None) => None,
            Err(err) => {
                eprintln!("initialize frame failed: {}", err);
                None
            }
        };

        if let Some(frame) = frame {
            let view: TextureView = frame.texture.create_view(&TextureViewDescriptor::default());
            let mut encoder: CommandEncoder = self.create_encoder(None);
            self.render_pass(&mut encoder, view)?;
            self.queue.submit(Some(encoder.finish()));
            frame.present();
        }

        self.buffer.atlas.trim();

        Ok(())
    }

    pub fn add_char_to_buffer(&mut self, char: char) {
        self.buffer.set_text(char);
        self.buffer.forward_col();
    }

    pub fn carriage_return(&mut self) {
        self.buffer.reset_col();
    }

    pub fn line_feed(&mut self) {
        self.buffer.line_feed();
    }

    pub fn clear_line(&mut self) {
        self.buffer.clear_line();
    }

    pub fn move_cursor_to(&mut self, row: u16, col: u16) {
        self.buffer.move_cursor_to(row, col);
    }

    fn create_encoder(&self, label: Option<&str>) -> CommandEncoder {
        self.device
            .create_command_encoder(&CommandEncoderDescriptor { label })
    }

    fn prepare(&mut self) -> Result<(), Box<dyn Error>> {
        self.buffer.text_renderer.prepare(
            &self.device,
            &self.queue,
            &mut self.buffer.font_system,
            &mut self.buffer.atlas,
            &self.buffer.viewport,
            [TextArea {
                buffer: &self.buffer.text_buffer,
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
            &mut self.buffer.swash_cache,
        )?;

        Ok(())
    }

    fn render_pass(
        &mut self,
        encoder: &mut CommandEncoder,
        view: TextureView,
    ) -> Result<(), Box<dyn Error>> {
        let mut pass: wgpu::RenderPass<'_> = pass::begin_render_pass(encoder, &view);

        self.buffer
            .text_renderer
            .render(&self.buffer.atlas, &self.buffer.viewport, &mut pass)?;

        let bind_group = self
            .rect_pipeline
            .caret_bind_group(&self.device, &self.uniform_buffer);

        self.rect_pipeline.draw_rect(&mut pass, bind_group);

        Ok(())
    }

    fn initialize_frame(&mut self) -> Result<Option<SurfaceTexture>, Box<dyn Error>> {
        let frame: SurfaceTexture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                // Try again later
                self.window.request_redraw();
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
                self.surface.configure(&self.device, &self.config);
                self.window.request_redraw();
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone())?;
                self.surface.configure(&self.device, &self.config);
                self.window.request_redraw();
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Validation => panic!("validation error"),
        };

        Ok(Some(frame))
    }
}
