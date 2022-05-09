// Vertex shader

struct Camera {
    view_proj: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> camera: Camera;

var<private> light_pos: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

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
    [[location(12)]] tex_id: u32;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1), interpolate(linear)]] world_normal: vec3<f32>;
    [[location(2)]] world_position: vec3<f32>;
    [[location(3)]] tex_id: u32;
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
    let light_dir = normalize(light_pos - world_pos);
    let lambertian = max(dot(normal, light_dir), 0.0);
    let reflect_dir = reflect(-light_dir, normal);
    let viewer_dir = normalize(-world_pos);
    let spec_angle = max(dot(reflect_dir, viewer_dir), 0.0);
    let specular = pow(spec_angle, shininess);
    let color = vec3<f32>(k_a * amb_col +
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
    out.tex_id = instance.tex_id;
    return out;
}

// Fragment shader

[[group(0), binding(0)]]
var t_diffuse: binding_array<texture_2d<f32>>;
[[group(0), binding(1)]]
var s_diffuse: binding_array<sampler>;
[[group(0), binding(2)]]
var t_normal: binding_array<texture_2d<f32>>;
[[group(0), binding(3)]]
var s_normal: binding_array<sampler>;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let k_a = 0.1;
    let k_d = 0.8;
    let k_s = 0.1;

    let normal = normalize(in.world_normal);
    let world_pos = in.world_position;
    let light_pos = vec3<f32>(1.0, 1.0, 1.0);
    
    let diff_col = textureSample(t_diffuse[in.tex_id], s_diffuse[in.tex_id], in.tex_coords);
    let white = vec3<f32>(1.0, 1.0, 1.0);

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

    return vec4<f32>(color, 1.0);
}
