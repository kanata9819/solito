struct ScreenUniform {
    screen: vec2<f32>,
    pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> screen: ScreenUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) radius: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) index: u32,
    @location(0) rect_pos: vec2<f32>,
    @location(1) rect_size: vec2<f32>,
    @location(2) rect_color: vec4<f32>,
    @location(3) rect_radius: f32,
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

    let pixel = rect_pos + local * rect_size;

    let ndc = vec2<f32>(
        pixel.x / screen.screen.x * 2.0 - 1.0,
        1.0 - pixel.y / screen.screen.y * 2.0
    );

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.color = rect_color;
    out.local = local * rect_size;
    out.size = rect_size;
    out.radius = min(rect_radius, min(rect_size.x, rect_size.y) * 0.5);

    return out;
}

// Fragment shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if input.radius <= 0.0 {
        return input.color;
    }
    // A pixel-space distance field keeps corners smooth without extra geometry.
    let half_size = input.size * 0.5;
    let q = abs(input.local - half_size) - (half_size - vec2<f32>(input.radius));
    let distance = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - input.radius;
    let coverage = 1.0 - smoothstep(-0.5, 0.5, distance);
    return vec4<f32>(input.color.rgb, input.color.a * coverage);
}
