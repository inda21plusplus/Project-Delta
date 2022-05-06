[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

struct VertexInput {
    [[location(0)]] pos: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0), interpolate(linear)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.pos, 0.0, 1.0);
    out.uv = vec2<f32>(
        (in.pos.x + 1.0) / 2.0,
        1.0 - (in.pos.y / 2.0 + 0.5)
    );
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var color = textureSample(t_diffuse, s_diffuse, in.uv);
    color = vec4<f32>(color.xyz * 0.001, color.w);
    return color;
}