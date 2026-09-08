use bytemuck::{Pod, Zeroable};
use wgpu::Buffer;
use wgpu::wgt::SurfaceConfiguration;

pub(crate) struct RectPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    instance_buffer: Option<Buffer>,
    instance_capacity: usize,
    instances: Vec<RectInstance>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RectSpec {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) color: [f32; 4],
    pub(crate) slant: f32,
}

impl RectSpec {
    pub(crate) fn new(x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Self {
        Self::slanted(x, y, width, height, color, 0.0)
    }

    // used for tab bar design
    pub(crate) fn slanted(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        slant: f32,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            color,
            slant,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RectInstance {
    pos: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
    slant: f32,
    _pad: [f32; 3],
}

impl From<RectSpec> for RectInstance {
    fn from(rect: RectSpec) -> Self {
        Self {
            pos: [rect.x, rect.y],
            size: [rect.width, rect.height],
            color: rect.color,
            slant: rect.slant,
            _pad: [0.0; 3],
        }
    }
}

const RECT_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32];

impl RectPipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        config: &SurfaceConfiguration<Vec<wgpu::TextureFormat>>,
        queue: &wgpu::Queue,
        uniform_buffer: &Buffer,
        window_width: u32,
        window_height: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shader/rect.wgsl"));
        let bind_group_layout = Self::rect_bind_group_layout(device);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<RectInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &RECT_INSTANCE_ATTRIBUTES,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        Self::update_screen_uniform(ScreenUniform {
            uniform_buffer,
            queue,
            width: window_width,
            height: window_height,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rect Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        Self {
            pipeline: render_pipeline,
            bind_group,
            instance_buffer: None,
            instance_capacity: 0,
            instances: Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ScreenUniform<'a> {
    pub uniform_buffer: &'a Buffer,
    pub queue: &'a wgpu::Queue,
    pub width: u32,
    pub height: u32,
}

impl RectPipeline {
    pub(crate) fn update_screen_uniform(uniform: ScreenUniform) {
        let (screen_w, screen_h): (f32, f32) = (uniform.width as f32, uniform.height as f32);
        let screen_uniform: [f32; 4] = [
            screen_w, screen_h, // window size
            0.0, 0.0, // padding
        ];

        uniform.queue.write_buffer(
            uniform.uniform_buffer,
            0,
            bytemuck::bytes_of(&screen_uniform),
        );
    }

    fn rect_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rect Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    pub(crate) fn create_screen_uniform_buffer(device: &wgpu::Device) -> Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rect Screen Uniform Buffer"),
            size: size_of::<[f32; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub(crate) fn upload_rects(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rects: &[RectSpec],
    ) {
        self.instances.clear();
        self.instances
            .extend(rects.iter().copied().map(RectInstance::from));
        if rects.is_empty() {
            return;
        }
        // Grow geometrically; ordinary frames reuse the same GPU allocation.
        if rects.len() > self.instance_capacity {
            self.instance_capacity = rects.len().next_power_of_two();
            self.instance_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Rect Instance Buffer"),
                size: (self.instance_capacity * size_of::<RectInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        queue.write_buffer(
            self.instance_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&self.instances),
        );
    }

    pub(crate) fn draw_rects(&self, pass: &mut wgpu::RenderPass) {
        use std::ops::Range;
        const VERTICES_COUNT: Range<u32> = 0..6;

        if self.instances.is_empty() {
            return;
        }
        let Some(instance_buffer) = &self.instance_buffer else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, instance_buffer.slice(..));
        pass.draw(VERTICES_COUNT, 0..self.instances.len() as u32);
    }
}
