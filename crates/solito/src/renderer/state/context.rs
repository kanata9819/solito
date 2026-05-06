use std::{error::Error, sync::Arc};
use wgpu::{Instance, Surface, TextureFormat};
use winit::{dpi::PhysicalSize, window::Window};

use crate::renderer::{
    state::{gpu::GpuContext, render::RenderResources, window::WindowSurface},
    terminal_view::TerminalView,
};

pub(crate) use super::terminal::TerminalViewRenderer;
pub(crate) use super::window::WindowRenderer;

pub(crate) struct State {
    pub(super) gpu: GpuContext,
    pub(super) window_surface: WindowSurface,
    pub(super) render_resources: RenderResources,
    pub(super) terminal_view: TerminalView,
}

impl State {
    pub(crate) async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let size: PhysicalSize<u32> = window.inner_size();
        let instance: Instance = GpuContext::create_instance();
        let surface: Surface<'_> = instance.create_surface(window.clone())?;
        let gpu: GpuContext = GpuContext::new(instance, &surface).await?;
        let window_surface: WindowSurface = WindowSurface::new(window, surface, &gpu, size);
        let render_resources: RenderResources =
            RenderResources::new(&gpu, &window_surface, size.width, size.height);
        let swapchain_format: TextureFormat = TextureFormat::Bgra8UnormSrgb;
        let terminal_view: TerminalView =
            TerminalView::new(&gpu.device, &gpu.queue, swapchain_format, size, 1.0);

        Ok(Self {
            gpu,
            window_surface,
            render_resources,
            terminal_view,
        })
    }
}
