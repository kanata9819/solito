use anyhow::Result;
use std::sync::Arc;
use wgpu::TextureFormat;
use winit::window::Window;

use crate::{
    RendererConfig,
    pipeline::rect,
    state::{gpu::GpuContext, resources::RenderResources, surface::WindowSurface},
    terminal_view::TerminalView,
};

/// Owns the GPU, window surface, and terminal view used to draw one window.
pub struct Renderer {
    pub(super) gpu: GpuContext,
    pub(super) window_surface: WindowSurface,
    pub(super) render_resources: RenderResources,
    pub(super) terminal_view: TerminalView,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, renderer_config: RendererConfig) -> Result<Self> {
        let size = window.inner_size();
        let instance = GpuContext::create_instance();
        let surface = instance.create_surface(window.clone())?;
        let gpu = GpuContext::new(instance, &surface).await?;
        let window_surface = WindowSurface::new(window, surface, &gpu, size, &renderer_config);
        let render_resources = RenderResources::new(&gpu, &window_surface);
        let swapchain_format = TextureFormat::Bgra8UnormSrgb;
        let terminal_view = TerminalView::new(
            &gpu.device,
            &gpu.queue,
            swapchain_format,
            size,
            renderer_config,
        );

        Ok(Self {
            gpu,
            window_surface,
            render_resources,
            terminal_view,
        })
    }

    pub(crate) fn update_rect_screen_uniform(&mut self) {
        rect::RectPipeline::update_screen_uniform(rect::ScreenUniform {
            uniform_buffer: &self.render_resources.uniform_buffer,
            queue: &self.gpu.queue,
            width: self.window_surface.config.width,
            height: self.window_surface.config.height,
        });
    }
}
