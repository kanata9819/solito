use super::{gpu::GpuContext, window::WindowSurface};
use crate::pipeline::rect::{self, Caret, Rect};

pub(super) struct RenderResources {
    pub(super) rect_pipeline: rect::RectPipeline,
    pub(super) uniform_buffer: wgpu::Buffer,
}

impl RenderResources {
    pub(super) fn new(
        gpu: &GpuContext,
        window_surface: &WindowSurface,
        width: u32,
        height: u32,
    ) -> Self {
        let uniform_buffer: wgpu::Buffer =
            rect::RectPipeline::create_caret_uniform_buffer(&gpu.device);
        let rect_pipeline: rect::RectPipeline = rect::RectPipeline::new(
            &gpu.device,
            window_surface.config.clone(),
            &gpu.queue,
            &uniform_buffer,
            width,
            height,
        );

        Self {
            rect_pipeline,
            uniform_buffer,
        }
    }
}
