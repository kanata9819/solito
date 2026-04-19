use std::{error::Error, sync::Arc};
use wgpu::{
    Adapter, CommandEncoder, Device, Instance, Queue, ShaderModule, Surface, SurfaceConfiguration,
    SurfaceTexture, TextureView,
};
use winit::{dpi::PhysicalSize, event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};

pub struct State {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipeline: wgpu::RenderPipeline,
    window: Arc<Window>,
}

impl State {
    pub async fn new(windows: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let size: PhysicalSize<u32> = windows.inner_size();
        let instance: Instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        // Current window API doesn't have 'clone()'.
        // I guess it will cause panic...
        let surface: Surface<'_> = instance.create_surface(windows.clone())?;

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

        let shader: ShaderModule =
            device.create_shader_module(wgpu::include_wgsl!("../shader/shader.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        //Several things to note here:
        // 1.Here you can specify which function inside the shader should be the entry_point.
        //  These are the functions we marked with @vertex and @fragment
        //
        // 2.The buffers field tells wgpu what type of vertices we want to pass to the vertex shader.
        //  We're specifying the vertices in the vertex shader itself,
        //  so we'll leave this empty. We'll put something there in the next tutorial.
        //
        // 3.The fragment is technically optional, so you have to wrap it in Some().
        //  We need it if we want to store color data to the surface.
        //
        // 4.The targets field tells wgpu what color outputs it should set up. Currently,
        //  we only need one for the surface.
        //  We use the surface's format so that copying to it is easy,
        //  and we specify that the blending should just replace old pixel data with new data.
        //  We also tell wgpu to write to all colors: red, blue, green, and alpha.
        //  We'll talk more about color_state when we talk about textures.
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), // 1.
                buffers: &[],                 // 2.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            // The primitive field describes how to interpret our vertices when converting them into triangles.
            // 1.Using PrimitiveTopology::TriangleList means that every three vertices will correspond to one triangle.
            // 2.The front_face and cull_mode fields tell wgpu how to determine whether a given triangle is facing forward or not.
            //  FrontFace::Ccw means that a triangle is facing forward if the vertices are arranged in a counter-clockwise direction.
            //  Triangles that are not considered facing forward are culled (not included in the render) as specified by CullMode::Back.
            //  We'll cover culling a bit more when we cover Buffers.
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            // The rest of the method is pretty simple:
            // 1.We're not using a depth/stencil buffer currently,
            //  so we leave depth_stencil as None. This will change later.
            // 2. count determines how many samples the pipeline will use.
            //  Multisampling is a complex topic, so we won't get into it here.
            // 3.mask specifies which samples should be active. In this case, we are using all of them.
            // 4.alpha_to_coverage_enabled has to do with anti-aliasing.
            //  We're not covering anti-aliasing here, so we'll leave this as false now.
            //5.multiview indicates how many array layers the render attachments can have.
            //  We won't be rendering to array textures, so we can set this to None.
            //6.cache allows wgpu to cache shader compilation data. Only really useful for Android build targets.
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview_mask: None, // 5.
            cache: None,          // 6.
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipeline,
            window: windows,
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

    /// This is where we'll handle keyboard events.
    pub fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
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

            render_pass.set_pipeline(&self.render_pipeline); // 2.
            render_pass.draw(0..3, 0..1); // 3.
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));

        // The last lines of the code tell wgpu to finish the command buffer
        // and submit it to the GPU's render queue.
        output.present();

        Ok(())
    }

    pub fn update(&mut self) {
        // remove `todo!()`
    }
}
