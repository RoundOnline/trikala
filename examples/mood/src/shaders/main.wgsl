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
@group(1) @binding(0) var shadow_tex: texture_depth_2d;
@group(1) @binding(1) var shadow_samp: sampler_comparison;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) emissive: f32,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) emissive: f32,
    @location(4) light_space: vec4<f32>,
};

@vertex
fn vs(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip = u.view_proj * vec4<f32>(in.pos, 1.0);
    out.world = in.pos;
    out.normal = in.normal;
    out.color = in.color;
    out.emissive = in.emissive;
    out.light_space = u.light_view_proj * vec4<f32>(in.pos, 1.0);
    return out;
}

fn sample_shadow(light_space: vec4<f32>, n_dot_l: f32) -> f32 {
    let bias = max(0.0015 * (1.0 - n_dot_l), 0.0004);
    let proj = light_space.xyz / light_space.w;
    let uv = proj.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || proj.z > 1.0) {
        return 1.0;
    }
    let depth = proj.z - bias;
    var visibility = 0.0;
    let texel = 1.0 / 1024.0;
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            let offset = vec2<f32>(f32(dx), f32(dy)) * texel;
            visibility = visibility + textureSampleCompare(shadow_tex, shadow_samp, uv + offset, depth);
        }
    }
    return visibility / 9.0;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    if (in.emissive > 0.5) {
        let lit = in.color;
        return vec4<f32>(lit / (lit + vec3<f32>(1.0)), 1.0);
    }
    let n = normalize(in.normal);
    let emi = clamp(in.emissive, 0.0, 1.0);

    let to_lamp = u.lamp_pos.xyz - in.world;
    let lamp_d = length(to_lamp);
    let lamp_l = normalize(to_lamp);
    let lamp_atten = 1.0 / (1.0 + 0.10 * lamp_d + 0.06 * lamp_d * lamp_d);
    let lamp_diffuse = max(dot(n, lamp_l), 0.0) * u.lamp_color.xyz * lamp_atten;

    let moon_l = normalize(u.moon_dir.xyz);
    let n_dot_l = max(dot(n, moon_l), 0.0);
    var shadow: f32 = 1.0;
    if (u.ambient_color.w > 0.5) {
        shadow = sample_shadow(in.light_space, n_dot_l);
    }
    let moon_diffuse = n_dot_l * shadow * u.moon_color.xyz;

    // Fireflies — N small warm point lights orbiting the player.
    // .w of each entry is a 0/1 mask so fireflies that haven't yet
    // walked through the door don't light the world they aren't in.
    var fly_total = vec3<f32>(0.0);
    for (var f = 0; f < 4; f = f + 1) {
        let to_fly = u.fly_pos[f].xyz - in.world;
        let fd = length(to_fly);
        let fl = normalize(to_fly);
        let atten = 1.0 / (0.4 + 0.6 * fd + 1.8 * fd * fd);
        fly_total = fly_total + max(dot(n, fl), 0.0) * u.fly_color.xyz * atten * u.fly_pos[f].w;
    }

    let base = in.color * (u.ambient_color.xyz + lamp_diffuse + moon_diffuse + fly_total);
    let glow = in.color * emi * 2.0;
    let lit = base + glow;
    return vec4<f32>(lit / (lit + vec3<f32>(1.0)), 1.0);
}
