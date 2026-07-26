use super::{gpu::GpuContext, window::WindowSurface};
use crate::pipeline::rect;

pub(super) struct RenderResources {
    pub(super) rect_pipeline: rect::RectPipeline,
    pub(super) uniform_buffer: wgpu::Buffer,
}

impl RenderResources {
    pub(super) fn new(gpu: &GpuContext, window_surface: &WindowSurface) -> Self {
        let uniform_buffer = rect::RectPipeline::create_screen_uniform_buffer(&gpu.device);
        let rect_pipeline = rect::RectPipeline::new(
            &gpu.device,
            window_surface.config.clone(),
            &gpu.queue,
            &uniform_buffer,
            window_surface.config.width,
            window_surface.config.height,
        );

        Self {
            rect_pipeline,
            uniform_buffer,
        }
    }
}
