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
    @location(0) world_y: f32,
    @location(1) normal: vec3<f32>,
};

fn ripple_height(world_xz: vec2<f32>, t_now: f32) -> f32 {
    var h = 0.0;
    for (var i = 0u; i < 8u; i = i + 1u) {
        let r = u.ripples[i];
        if (r.w < 0.5) { continue; }
        let age = t_now - r.z;
        if (age < 0.0 || age > 2.0) { continue; }
        let d = distance(world_xz, r.xy);
        let speed = 1.8;
        let front = age * speed;
        // a wavelet that travels outward from the spawn point
        let band = d - front;
        let ring = exp(-band * band * 6.0);
        let decay = (1.0 - age / 2.0);
        h += sin(d * 9.0 - age * 8.0) * 0.05 * ring * decay;
    }
    return h;
}

@vertex
fn vs(in: VsIn) -> VsOut {
    let t = u.player_pos.w;
    // Background swell — two crossing sin waves.
    let sw = sin(in.pos.x * 1.6 + t * 1.1) * 0.03
           + cos(in.pos.z * 2.1 + t * 0.9) * 0.025;
    let rp = ripple_height(in.pos.xz, t);
    let y = in.pos.y + sw + rp;

    // Approximate normal from finite differences of the same field.
    let eps = 0.15;
    let h_x1 = sin((in.pos.x + eps) * 1.6 + t * 1.1) * 0.03
             + cos(in.pos.z * 2.1 + t * 0.9) * 0.025
             + ripple_height(vec2<f32>(in.pos.x + eps, in.pos.z), t);
    let h_z1 = sin(in.pos.x * 1.6 + t * 1.1) * 0.03
             + cos((in.pos.z + eps) * 2.1 + t * 0.9) * 0.025
             + ripple_height(vec2<f32>(in.pos.x, in.pos.z + eps), t);
    let n = normalize(vec3<f32>(-(h_x1 - sw - rp) / eps, 1.0, -(h_z1 - sw - rp) / eps));

    var out: VsOut;
    out.clip = u.view_proj * vec4<f32>(in.pos.x, y, in.pos.z, 1.0);
    out.world_y = y;
    out.normal = n;
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let sun = -u.moon_dir.xyz;
    let lambert = max(dot(in.normal, sun), 0.0);
    let base = vec3<f32>(0.10, 0.32, 0.46);
    let lit = base * (u.ambient_color.rgb + u.moon_color.rgb * lambert);
    // Fresnel-ish whitening at glancing angles.
    let view_dir = normalize(u.camera_pos.xyz - vec3<f32>(0.0, in.world_y, 0.0));
    let f = pow(1.0 - max(dot(in.normal, view_dir), 0.0), 3.0);
    let col = lit + vec3<f32>(0.25, 0.35, 0.45) * f * 0.6;
    return vec4<f32>(col, 0.92);
}
