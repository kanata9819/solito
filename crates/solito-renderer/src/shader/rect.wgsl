struct CaretUniform {
    pos: vec2<f32>,
    size: vec2<f32>,
    screen: vec2<f32>,
    pad: vec2<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> caret: CaretUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {

    var quad = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),

        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );

    let local = quad[index];

    let pixel = caret.pos + local * caret.size;

    let ndc = vec2<f32>(
        pixel.x / caret.screen.x * 2.0 - 1.0,
        1.0 - pixel.y / caret.screen.y * 2.0
    );

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);

    return out;
}

// Fragment shader
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return caret.color;
}
