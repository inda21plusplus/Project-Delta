// Vertex shader

struct Camera {
    view_proj: mat4x4<f32>;
    pos: vec3<f32>;
};

struct Light {
    world_pos: vec3<f32>;
    radius: f32;
    color: vec3<f32>;
    k_c: f32;
    k_l: f32;
    k_q: f32;
};

[[group(1), binding(0)]]
var<uniform> camera: Camera;
[[group(2), binding(0)]]
var<uniform> light_buf: Light;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
};
struct InstanceInput {
    [[location(5)]] model_matrix_0: vec4<f32>;
    [[location(6)]] model_matrix_1: vec4<f32>;
    [[location(7)]] model_matrix_2: vec4<f32>;
    [[location(8)]] model_matrix_3: vec4<f32>;
    [[location(9)]] normal_matrix_0: vec3<f32>;
    [[location(10)]] normal_matrix_1: vec3<f32>;
    [[location(11)]] normal_matrix_2: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1), interpolate(linear)]] world_normal: vec3<f32>;
    [[location(2), interpolate(linear)]] world_position: vec3<f32>;
};

fn shade_sample(
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    light_pos: vec3<f32>,
    diff_col: vec3<f32>,
    amb_col: vec3<f32>,
    spec_col: vec3<f32>,
    k_a: f32,
    k_d: f32,
    k_s: f32,
    shininess: f32,
) -> vec3<f32> {
    let light_ray = light_pos - world_pos;
    let dist = length(light_ray);
    let attenuation = 1.0 / (light_buf.k_c + light_buf.k_l * dist + light_buf.k_q * dist * dist);

    let light_dir = normalize(light_ray);
    let lambertian = max(dot(normal, light_dir), 0.0);
    let reflect_dir = reflect(-light_dir, normal);
    let viewer_dir = normalize(world_pos - camera.pos);
    let spec_angle = max(dot(reflect_dir, viewer_dir), 0.0);
    let specular = pow(spec_angle, shininess);
    let color =
        vec3<f32>(k_a * amb_col +
                  k_d * lambertian * diff_col +
                  k_s * specular * spec_col);
    return color;
}

[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_normal = (model_matrix * vec4<f32>(model.normal, 0.0)).xyz;
    out.clip_position = camera.view_proj * world_position;
    out.world_position = world_position.xyz;
    //out.normal = (camera.view_proj * model_matrix * vec4<f32>(model.normal, 0.0)).xyz;
    return out;
}

// Fragment shader

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;
[[group(0), binding(2)]]
var t_normal: texture_2d<f32>;
[[group(0), binding(3)]]
var s_normal: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let k_a = 0.1;
    let k_d = 0.8;
    let k_s = 0.1;

    let normal = normalize(in.world_normal);
    let world_pos = in.world_position;
    let light_pos = light_buf.world_pos;
    
    let diff_col = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let white = vec3<f32>(1.0);

    let shininess = 80.0;

    let color = shade_sample(
        world_pos,
        normal,
        light_pos,

        diff_col.xyz,
        diff_col.xyz, // amb_col
        white, // spec_col

        k_a,
        k_d,
        k_s,

        shininess,
    );

    return vec4<f32>(diff_col.xyz, 1.0);
}
