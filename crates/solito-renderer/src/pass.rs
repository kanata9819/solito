use wgpu::{
    CommandEncoder, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, TextureView,
};

pub(crate) fn begin_render_pass<'a>(
    encoder: &'a mut CommandEncoder,
    view: &TextureView,
) -> RenderPass<'a> {
    encoder.begin_render_pass(&RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}
