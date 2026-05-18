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
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_xz: vec2<f32>,
};

@vertex
fn vs(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip = u.view_proj * vec4<f32>(in.pos, 1.0);
    out.world_xz = in.pos.xz;
    return out;
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let grain = (hash(floor(in.world_xz * 14.0)) - 0.5) * 0.08;
    let base = vec3<f32>(0.86, 0.78, 0.58) + vec3<f32>(grain);
    let n = vec3<f32>(0.0, 1.0, 0.0);
    let sun = -u.moon_dir.xyz;
    let lambert = max(dot(n, sun), 0.0);
    let lit = base * (u.ambient_color.rgb + u.moon_color.rgb * lambert);
    return vec4<f32>(lit, 1.0);
}
