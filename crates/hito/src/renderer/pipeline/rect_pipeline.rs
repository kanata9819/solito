use wgpu::ShaderModule;
use wgpu::wgt::SurfaceConfiguration;

pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
}

struct RectUniform {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    screen_w: f32,
    screen_h: f32,
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        config: SurfaceConfiguration<Vec<wgpu::TextureFormat>>,
    ) -> Self {
        let shader: ShaderModule =
            device.create_shader_module(wgpu::include_wgsl!("../../shader/rect.wgsl"));

        let render_pipeline_layout: wgpu::PipelineLayout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
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

        Self {
            pipeline: render_pipeline,
        }
    }

    pub fn draw_rect(&self, pass: &mut wgpu::RenderPass) {
        use std::ops::Range;
        const VERTICIES_COUNT: Range<u32> = 0..6;
        const INSTANCE_COUNT: Range<u32> = 0..1;

        pass.set_pipeline(&self.pipeline);
        pass.draw(VERTICIES_COUNT, INSTANCE_COUNT)
    }
}
