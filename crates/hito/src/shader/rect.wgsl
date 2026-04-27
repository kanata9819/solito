struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in: u32,
) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-0.01, 0.00), // left bottom
        vec2<f32>( 0.00, 0.00), // right bottom
        vec2<f32>( 0.00, 0.05), // right top

        vec2<f32>(-0.01, 0.00), // left bottom
        vec2<f32>( 0.00, 0.05), // right top
        vec2<f32>(-0.01, 0.05), // left top
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[in], 0.0, 1.0);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.7, 1.0, 1.0);
}