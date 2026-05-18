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
    player_pos: vec4<f32>,
    ripples: array<vec4<f32>, 8>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) age01: f32,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) age01: f32,
};

@vertex
fn vs(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip = u.view_proj * vec4<f32>(in.pos, 1.0);
    out.uv = in.uv;
    out.age01 = in.age01;
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // Oval footprint mask centred in UV space.
    let c = in.uv - vec2<f32>(0.5);
    let r2 = c.x * c.x * 2.0 + c.y * c.y * 0.8;
    if (r2 > 0.25) { discard; }
    let strength = (1.0 - in.age01) * 0.55 * (1.0 - r2 * 3.5);
    return vec4<f32>(0.10, 0.08, 0.06, max(strength, 0.0));
}
