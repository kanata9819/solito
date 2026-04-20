use glyphon::{Color, Resolution, TextArea, TextBounds};
use std::{
    error::Error,
    sync::{Arc, mpsc::Sender},
};
use wgpu::{
    Adapter, CommandEncoder, CommandEncoderDescriptor, Device, Instance, LoadOp, Operations, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, Surface, SurfaceConfiguration, SurfaceTexture,
    TextureFormat, TextureView, TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::renderer::{pipeline::Pipeline, text::InputBuffer};

pub struct State {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipeline: Pipeline,
    buffer: InputBuffer,
    instance: Instance,
    window: Arc<Window>,
    input_tx: Sender<String>,
}

impl State {
    pub async fn new(
        window: Arc<Window>,
        input_tx: Sender<String>,
    ) -> Result<Self, Box<dyn Error>> {
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
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
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

        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
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

        let render_pipeline: Pipeline = Pipeline::new(&device, config.clone());
        let swapchain_format: TextureFormat = TextureFormat::Bgra8UnormSrgb;
        let buffer: InputBuffer =
            InputBuffer::new(&device.clone(), &queue.clone(), swapchain_format, size, 1.0);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipeline,
            buffer,
            instance,
            window,
            input_tx,
        })
    }

    /// ↓ quoted from official document. ↓
    /// If we want to support resizing in our application,
    /// we're going to need to reconfigure the surface every time the window's size changes.
    /// That's the reason we stored the physical size and the config used to configure the surface.
    /// With all of these, the resize method is very simple.
    ///
    /// This is where we configure the surface.
    /// We need the surface to be configured before we can do anything with it.
    /// We set the is_surface_configured flag to true here and we'll check it in the render() function.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let size: PhysicalSize<u32> = size;
        if size.width > 0 && size.height > 0 {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
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

        // This line creates a TextureView with default settings.
        // We need to do this because we want to control how the render code interacts with the texture.
        let view: TextureView = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // We also need to create a CommandEncoder to create the actual commands to send to the GPU.
        // Most modern graphics frameworks expect commands to be stored in a command buffer before being sent to the GPU.
        // The encoder builds a command buffer that we can then send to the GPU.
        let mut encoder: CommandEncoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            // First things first, let's talk about the extra block ({}) around encoder.begin_render_pass(...).
            // begin_render_pass() borrows encoder mutably (aka &mut self).
            // We can't call encoder.finish() until we release that mutable borrow.
            // The block tells Rust to drop any variables within it when the code leaves that scope,
            // thus releasing the mutable borrow on encoder and allowing us to finish() it.
            // If you don't like the {}, you can also use drop(render_pass) to achieve the same effect.
            // 1.
            let mut render_pass: wgpu::RenderPass<'_> =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[
                        // This is what @location(0) in the fragment shader targets
                        Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                    ],
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                    depth_stencil_attachment: None,
                });

            render_pass.set_pipeline(self.render_pipeline.render_pipeline()); // 2.
            render_pass.draw(0..3, 0..1); // 3.
        }

        // submit will accept anything that implements IntoIter
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

        self.buffer
            .text_renderer
            .prepare(
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
            )
            .unwrap();

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                // Try again later
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
                self.surface.configure(&self.device, &self.config);
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone())?;
                self.surface.configure(&self.device, &self.config);
                self.window.request_redraw();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => panic!("validation error"),
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.buffer
                .text_renderer
                .render(&self.buffer.atlas, &self.buffer.viewport, &mut pass)
                .unwrap();
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        self.buffer.atlas.trim();

        Ok(())
    }

    pub fn add_char_to_buffer(&mut self, char: char) -> Result<(), Box<dyn Error>> {
        self.buffer.set_text(char);
        self.input_tx.send(char.to_string())?;

        Ok(())
    }
}
