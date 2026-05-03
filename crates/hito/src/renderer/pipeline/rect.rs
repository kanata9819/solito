use wgpu::wgt::SurfaceConfiguration;
use wgpu::{Buffer, ShaderModule};

use crate::util::general_parser;

pub struct RectPipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}
pub trait Rect {
    fn new(
        device: &wgpu::Device,
        config: SurfaceConfiguration<Vec<wgpu::TextureFormat>>,
        queue: &wgpu::Queue,
        uniform_buffer: &Buffer,
        window_width: u32,
        window_height: u32,
    ) -> Self;
}

impl Rect for RectPipeline {
    fn new(
        device: &wgpu::Device,
        config: SurfaceConfiguration<Vec<wgpu::TextureFormat>>,
        queue: &wgpu::Queue,
        uniform_buffer: &Buffer,
        window_width: u32,
        window_height: u32,
    ) -> Self {
        let shader: ShaderModule =
            device.create_shader_module(wgpu::include_wgsl!("../../shader/rect.wgsl"));
        let bind_group_layout: wgpu::BindGroupLayout = Self::caret_bind_group_layout(device);

        let render_pipeline_layout: wgpu::PipelineLayout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

        let render_pipeline: wgpu::RenderPipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            });

        Self::update_caret_uniform(uniform_buffer, queue, window_width, window_height);

        Self {
            pipeline: render_pipeline,
            layout: bind_group_layout,
        }
    }
}

pub trait Caret {
    fn caret_bind_group(&self, device: &wgpu::Device, uniform_buffer: &Buffer) -> wgpu::BindGroup;
    fn update_caret_uniform(uniform_buffer: &Buffer, queue: &wgpu::Queue, width: u32, height: u32);
    fn caret_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout;
    fn create_caret_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer;
    fn draw_rect(&self, pass: &mut wgpu::RenderPass, bind_group: wgpu::BindGroup);
}

impl Caret for RectPipeline {
    fn caret_bind_group(&self, device: &wgpu::Device, uniform_buffer: &Buffer) -> wgpu::BindGroup {
        let bind_group: wgpu::BindGroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Caret Bind Group"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        bind_group
    }

    fn update_caret_uniform(uniform_buffer: &Buffer, queue: &wgpu::Queue, width: u32, height: u32) {
        let (caret_x, caret_y) = (50.0, 30.0);
        let (caret_w, caret_h) = (15.0, 30.0);
        let (padding_x, padding_y) = (0.0, 0.0);
        let (screen_w, screen_h) = (width as f32, height as f32);
        let caret_uniform: [f32; 8] = [
            caret_x, caret_y, // pos
            caret_w, caret_h, // size
            screen_w, screen_h, // window size
            padding_x, padding_y, // padding
        ];

        queue.write_buffer(uniform_buffer, 0, general_parser::as_bytes(&caret_uniform));
    }

    fn caret_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let bind_group_layout: wgpu::BindGroupLayout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Caret Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        bind_group_layout
    }

    fn create_caret_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        let caret_uniform_buffer: wgpu::Buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Caret Uniform Buffer"),
            size: std::mem::size_of::<[f32; 8]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        caret_uniform_buffer
    }

    fn draw_rect(&self, pass: &mut wgpu::RenderPass, bind_group: wgpu::BindGroup) {
        use std::ops::Range;
        const VERTICIES_COUNT: Range<u32> = 0..6;
        const INSTANCE_COUNT: Range<u32> = 0..1;

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(VERTICIES_COUNT, INSTANCE_COUNT);
    }
}
