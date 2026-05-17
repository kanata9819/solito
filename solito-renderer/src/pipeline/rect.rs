use wgpu::util::DeviceExt;
use wgpu::wgt::SurfaceConfiguration;
use wgpu::{Buffer, ShaderModule};

pub(crate) struct RectPipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RectSpec {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) color: [f32; 4],
}

impl RectSpec {
    pub(crate) fn new(x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Self {
        Self {
            x,
            y,
            width,
            height,
            color,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RectInstance {
    pos: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
}

impl From<RectSpec> for RectInstance {
    fn from(rect: RectSpec) -> Self {
        Self {
            pos: [rect.x, rect.y],
            size: [rect.width, rect.height],
            color: rect.color,
        }
    }
}

pub(crate) trait Rect {
    fn new(
        device: &wgpu::Device,
        config: SurfaceConfiguration<Vec<wgpu::TextureFormat>>,
        queue: &wgpu::Queue,
        uniform_buffer: &Buffer,
        window_width: u32,
        window_height: u32,
    ) -> Self;
}

const RECT_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

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
            device.create_shader_module(wgpu::include_wgsl!("../shader/rect.wgsl"));
        let bind_group_layout: wgpu::BindGroupLayout = Self::rect_bind_group_layout(device);

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
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<RectInstance>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &RECT_INSTANCE_ATTRIBUTES,
                    }],
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

        Self {
            pipeline: render_pipeline,
            layout: bind_group_layout,
        }
    }
}

pub(crate) struct ScreenUniform<'a> {
    pub uniform_buffer: &'a Buffer,
    pub queue: &'a wgpu::Queue,
    pub width: u32,
    pub height: u32,
}

pub(crate) trait RectRenderer {
    fn rect_bind_group(&self, device: &wgpu::Device, uniform_buffer: &Buffer) -> wgpu::BindGroup;
    fn update_screen_uniform(screen_uniform: ScreenUniform);
    fn rect_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout;
    fn create_screen_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer;
    fn create_instance_buffer(device: &wgpu::Device, rects: &[RectSpec]) -> Option<wgpu::Buffer>;
    fn draw_rects(
        &self,
        pass: &mut wgpu::RenderPass,
        bind_group: wgpu::BindGroup,
        instance_buffer: &wgpu::Buffer,
        rect_count: usize,
    );
}

impl RectRenderer for RectPipeline {
    fn rect_bind_group(&self, device: &wgpu::Device, uniform_buffer: &Buffer) -> wgpu::BindGroup {
        let bind_group: wgpu::BindGroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rect Bind Group"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        bind_group
    }

    fn update_screen_uniform(uniform: ScreenUniform) {
        let (screen_w, screen_h): (f32, f32) = (uniform.width as f32, uniform.height as f32);
        let screen_uniform: [f32; 4] = [
            screen_w, screen_h, // window size
            0.0, 0.0, // padding
        ];

        uniform
            .queue
            .write_buffer(uniform.uniform_buffer, 0, as_bytes(&screen_uniform));
    }

    fn rect_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let bind_group_layout: wgpu::BindGroupLayout =
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
            });

        bind_group_layout
    }

    fn create_screen_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        let screen_uniform_buffer: wgpu::Buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rect Screen Uniform Buffer"),
            size: std::mem::size_of::<[f32; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        screen_uniform_buffer
    }

    fn create_instance_buffer(device: &wgpu::Device, rects: &[RectSpec]) -> Option<wgpu::Buffer> {
        if rects.is_empty() {
            return None;
        }

        let instances: Vec<RectInstance> = rects.iter().copied().map(RectInstance::from).collect();

        Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Rect Instance Buffer"),
                contents: slice_as_bytes(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        )
    }

    fn draw_rects(
        &self,
        pass: &mut wgpu::RenderPass,
        bind_group: wgpu::BindGroup,
        instance_buffer: &wgpu::Buffer,
        rect_count: usize,
    ) {
        use std::ops::Range;
        const VERTICES_COUNT: Range<u32> = 0..6;

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, instance_buffer.slice(..));
        pass.draw(VERTICES_COUNT, 0..rect_count as u32);
    }
}

fn as_bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of::<T>()) }
}

fn slice_as_bytes<T>(values: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr() as *const u8, std::mem::size_of_val(values))
    }
}
