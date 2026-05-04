use std::{error::Error, sync::Arc};
use wgpu::{Adapter, Device, Instance, Queue, Surface, SurfaceConfiguration, TextureFormat};
use winit::{dpi::PhysicalSize, window::Window};

use crate::renderer::{
    pipeline::rect::{self, Caret, Rect},
    screen::buffer::InputBuffer,
};

pub use super::terminal::TerminalOutputSink;
pub use super::window::WindowRenderer;

pub struct State {
    pub(super) surface: Surface<'static>,
    pub(super) device: Device,
    pub(super) queue: Queue,
    pub(super) config: SurfaceConfiguration,
    pub(super) is_surface_configured: bool,
    pub(super) rect_pipeline: rect::RectPipeline,
    pub(super) buffer: InputBuffer,
    pub(super) instance: Instance,
    pub(super) window: Arc<Window>,
    pub(super) uniform_buffer: wgpu::Buffer,
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

        let uniform_buffer: wgpu::Buffer = rect::RectPipeline::create_caret_uniform_buffer(&device);
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
}
