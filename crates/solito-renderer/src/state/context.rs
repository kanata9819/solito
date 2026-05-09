use std::{error::Error, sync::Arc};
use wgpu::{Instance, Surface, TextureFormat};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    pipeline::rect::{self, CaretRenderer},
    state::{gpu::GpuContext, render::RenderResources, window::WindowSurface},
    terminal_view::TerminalView,
};

pub use super::terminal::TerminalViewRenderer;
pub use super::window::WindowRenderer;

pub struct State {
    pub(super) gpu: GpuContext,
    pub(super) window_surface: WindowSurface,
    pub(super) render_resources: RenderResources,
    pub(super) terminal_view: TerminalView,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
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

    pub(crate) fn update_caret_uniform(&mut self) {
        let (caret_x, caret_y, caret_w, caret_h): (f32, f32, f32, f32) =
            self.terminal_view.caret_rect();
        let caret_color: [f32; 4] = self.terminal_view.caret_color();

        rect::RectPipeline::update_caret_uniform(
            &self.render_resources.uniform_buffer,
            &self.gpu.queue,
            self.window_surface.config.width,
            self.window_surface.config.height,
            caret_x,
            caret_y,
            caret_w,
            caret_h,
            caret_color,
        );
    }
}
