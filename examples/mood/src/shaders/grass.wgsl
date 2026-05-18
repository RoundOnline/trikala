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
    @location(1) tip_factor: f32,
    @location(2) base_xz: vec2<f32>,
    @location(3) phase: f32,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) tip_factor: f32,
};

@vertex
fn vs(in: VsIn) -> VsOut {
    var p = in.pos;
    let to_blade = in.base_xz - u.player_pos.xz;
    let dist = length(to_blade);
    let bend_radius = 1.2;
    if (dist < bend_radius && in.tip_factor > 0.0) {
        let amt = (1.0 - dist / bend_radius) * in.tip_factor;
        let dir = to_blade / max(dist, 0.0001);
        p.x += dir.x * amt * 0.6;
        p.z += dir.y * amt * 0.6;
        p.y -= amt * 0.35;
    }
    let t = u.player_pos.w;
    p.x += sin(t * 1.5 + in.phase) * 0.07 * in.tip_factor;
    p.z += cos(t * 1.2 + in.phase * 1.3) * 0.04 * in.tip_factor;
    var out: VsOut;
    out.clip = u.view_proj * vec4<f32>(p, 1.0);
    out.tip_factor = in.tip_factor;
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let base_col = vec3<f32>(0.16, 0.36, 0.12);
    let tip_col  = vec3<f32>(0.55, 0.78, 0.32);
    let col = mix(base_col, tip_col, in.tip_factor);
    let n = vec3<f32>(0.0, 1.0, 0.0);
    let sun = -u.moon_dir.xyz;
    let lambert = max(dot(n, sun), 0.0);
    let lit = col * (u.ambient_color.rgb + u.moon_color.rgb * lambert);
    return vec4<f32>(lit, 1.0);
}
