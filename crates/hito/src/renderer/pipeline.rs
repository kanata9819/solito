use wgpu::ShaderModule;
use wgpu::wgt::SurfaceConfiguration;

pub struct Pipeline {
    render_pipeline: wgpu::RenderPipeline,
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        config: SurfaceConfiguration<Vec<wgpu::TextureFormat>>,
    ) -> Self {
        let shader: ShaderModule =
            device.create_shader_module(wgpu::include_wgsl!("../shader/shader.wgsl"));

        let render_pipeline_layout: wgpu::PipelineLayout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        //Several things to note here:
        // 1.Here you can specify which function inside the shader should be the entry_point.
        //  These are the functions we marked with @vertex and @fragment
        //
        // 2.The buffers field tells wgpu what type of vertices we want to pass to the vertex shader.
        //  We're specifying the vertices in the vertex shader itself,
        //  so we'll leave this empty. We'll put something there in the next tutorial.
        //
        // 3.The fragment is technically optional, so you have to wrap it in Some().
        //  We need it if we want to store color data to the surface.
        //
        // 4.The targets field tells wgpu what color outputs it should set up. Currently,
        //  we only need one for the surface.
        //  We use the surface's format so that copying to it is easy,
        //  and we specify that the blending should just replace old pixel data with new data.
        //  We also tell wgpu to write to all colors: red, blue, green, and alpha.
        //  We'll talk more about color_state when we talk about textures.
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), // 1.
                buffers: &[],                 // 2.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            // The primitive field describes how to interpret our vertices when converting them into triangles.
            // 1.Using PrimitiveTopology::TriangleList means that every three vertices will correspond to one triangle.
            // 2.The front_face and cull_mode fields tell wgpu how to determine whether a given triangle is facing forward or not.
            //  FrontFace::Ccw means that a triangle is facing forward if the vertices are arranged in a counter-clockwise direction.
            //  Triangles that are not considered facing forward are culled (not included in the render) as specified by CullMode::Back.
            //  We'll cover culling a bit more when we cover Buffers.
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            // The rest of the method is pretty simple:
            // 1.We're not using a depth/stencil buffer currently,
            //  so we leave depth_stencil as None. This will change later.
            // 2. count determines how many samples the pipeline will use.
            //  Multisampling is a complex topic, so we won't get into it here.
            // 3.mask specifies which samples should be active. In this case, we are using all of them.
            // 4.alpha_to_coverage_enabled has to do with anti-aliasing.
            //  We're not covering anti-aliasing here, so we'll leave this as false now.
            //5.multiview indicates how many array layers the render attachments can have.
            //  We won't be rendering to array textures, so we can set this to None.
            //6.cache allows wgpu to cache shader compilation data. Only really useful for Android build targets.
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview_mask: None, // 5.
            cache: None,          // 6.
        });

        Self { render_pipeline }
    }

    pub fn render_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.render_pipeline
    }
}
