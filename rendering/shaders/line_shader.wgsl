struct Camera {
    view_proj: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> camera: Camera;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] color: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = vec4<f32>(model.color, 1.0);
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader
[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
}
