[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;
[[group(0), binding(2)]]
var t_normal: texture_2d<f32>;
[[group(0), binding(3)]]
var s_normal: sampler;
[[group(0), binding(4)]]
var t_pos: texture_2d<f32>;
[[group(0), binding(5)]]
var s_pos: sampler;
[[group(2), binding(6)]]
var depth_tex: texture_depth_2d;

struct Camera {
    view_proj: mat4x4<f32>;
    camera_pos: vec3<f32>;
};
[[group(1), binding(0)]]
var<uniform> camera: Camera;


struct VertexInput {
    [[location(0)]] vtx_pos: vec3<f32>;
    [[location(1)]] light_pos: vec3<f32>;
    [[location(2)]] light_color: vec3<f32>;
    [[location(3)]] radius: f32;
    [[location(4)]] k_c: f32;
    [[location(5)]] k_l: f32;
    [[location(6)]] k_q: f32;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0), interpolate(linear)]] uv: vec2<f32>;
    [[location(1), interpolate(flat)]] light_pos: vec3<f32>;
    [[location(2), interpolate(flat)]] light_color: vec3<f32>;
    [[location(4), interpolate(flat)]] k_c: f32;
    [[location(5), interpolate(flat)]] k_l: f32;
    [[location(6), interpolate(flat)]] k_q: f32;
};

[[stage(vertex)]]
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    var pos = camera.view_proj * vec4<f32>(in.vtx_pos * in.radius + in.light_pos, 1.0);
    out.clip_position = pos;
    var uv_pos = pos.xy / pos.w;

    out.uv = vec2<f32>(
        (uv_pos.x + 1.0) / 2.0,
        1.0 - (uv_pos.y / 2.0 + 0.5)
    );

    var light_pos_proj = camera.view_proj * vec4<f32>(in.light_pos, 1.0);
    out.light_pos = in.light_pos;
    out.light_color = in.light_color;
    out.k_c = in.k_c;
    out.k_l = in.k_l;
    out.k_q = in.k_q;
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var pos = textureSample(t_pos, s_pos, in.uv).xyz;
    var normal = textureSample(t_normal, s_normal, in.uv).xyz;
    var albedo = vec4<f32>(textureSample(t_diffuse, s_diffuse, in.uv).xyz, 1.0);
    
    var ambient = albedo * 0.1;

    var dist = length(in.light_pos - pos);
    var attenuation = 1.0 / (in.k_c + in.k_l * dist + in.k_q * dist * dist);
    var lightdir = normalize(in.light_pos - pos);

    var lambertian = max(dot(normal, lightdir), 0.0);
    var diffuse = lambertian * albedo * vec4<f32>(in.light_color, 1.0);

    var color = attenuation * (ambient + diffuse);
    color.w = 1.0;

    return color;
}