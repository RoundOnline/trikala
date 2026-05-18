struct Uniforms {
    view_proj: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    lamp_pos: vec4<f32>,
    lamp_color: vec4<f32>,
    moon_dir: vec4<f32>,
    moon_color: vec4<f32>,
    ambient_color: vec4<f32>,
    fly_pos: array<vec4<f32>, 4>,
    fly_color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var<storage, read> bones: array<mat4x4<f32>>;

@vertex
fn vs(
    @location(0) pos: vec3<f32>,
    @location(1) _normal: vec3<f32>,
    @location(2) _color: vec3<f32>,
    @location(3) _emissive: f32,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>,
    @location(6) _uv: vec2<f32>,
) -> @builtin(position) vec4<f32> {
    let skin = bones[joints.x] * weights.x
             + bones[joints.y] * weights.y
             + bones[joints.z] * weights.z
             + bones[joints.w] * weights.w;
    let world_pos = (skin * vec4<f32>(pos, 1.0)).xyz;
    return u.light_view_proj * vec4<f32>(world_pos, 1.0);
}
