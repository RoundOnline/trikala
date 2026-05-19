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

@vertex
fn vs(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u.light_view_proj * vec4<f32>(pos, 1.0);
}
