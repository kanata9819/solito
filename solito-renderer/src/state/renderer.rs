use crate::terminal_view::{CopyModeSnapshot, TabBarSnapshot};
use anyhow::Result;
use solito_terminal::{ScreenSnapshot, TerminalSize};
use std::sync::Arc;
use wgpu::TextureFormat;
use winit::{dpi::PhysicalSize, window::Window};

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

    pub fn set_copy_mode(&mut self, snapshot: CopyModeSnapshot) {
        self.terminal_view.set_copy_mode(snapshot);
    }

    pub fn set_tab_bar(&mut self, snapshot: TabBarSnapshot) {
        self.terminal_view.set_tab_bar(snapshot);
    }

    pub fn set_terminal_snapshot(&mut self, snapshot: ScreenSnapshot) {
        self.terminal_view.set_snapshot(snapshot);
    }

    pub fn set_terminal_snapshot_at_bottom(&mut self, snapshot: ScreenSnapshot) {
        self.terminal_view.set_snapshot_at_bottom(snapshot);
    }

    pub fn terminal_size(&self) -> TerminalSize {
        let width = self.window_surface.config.width;
        let height = self.window_surface.config.height;

        self.terminal_size_for(PhysicalSize::new(width, height))
    }

    pub fn terminal_size_for(&self, window_size: PhysicalSize<u32>) -> TerminalSize {
        TerminalSize::new(
            self.terminal_view.visible_cols(window_size.width),
            self.terminal_view.visible_rows(window_size.height),
        )
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
