struct ScreenUniform {
    screen: vec2<f32>,
    pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> screen: ScreenUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) index: u32,
    @location(0) rect_pos: vec2<f32>,
    @location(1) rect_size: vec2<f32>,
    @location(2) rect_color: vec4<f32>,
    @location(3) rect_slant: f32,
) -> VertexOutput {

    var quad = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),

        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );

    let local = quad[index];

    let pixel = rect_pos + vec2<f32>(
        local.x * rect_size.x + (1.0 - local.y) * rect_slant,
        local.y * rect_size.y
    );

    let ndc = vec2<f32>(
        pixel.x / screen.screen.x * 2.0 - 1.0,
        1.0 - pixel.y / screen.screen.y * 2.0
    );

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.color = rect_color;

    return out;
}

// Fragment shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
