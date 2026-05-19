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
@group(2) @binding(0) var<storage, read> bones: array<mat4x4<f32>>;
@group(2) @binding(1) var diffuse_tex: texture_2d<f32>;
@group(2) @binding(2) var diffuse_samp: sampler;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) emissive: f32,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>,
    @location(6) uv: vec2<f32>,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) emissive: f32,
    @location(4) light_space: vec4<f32>,
    @location(5) uv: vec2<f32>,
};

@vertex
fn vs(in: VsIn) -> VsOut {
    let skin = bones[in.joints.x] * in.weights.x
             + bones[in.joints.y] * in.weights.y
             + bones[in.joints.z] * in.weights.z
             + bones[in.joints.w] * in.weights.w;
    let world_pos = (skin * vec4<f32>(in.pos, 1.0)).xyz;
    let world_n = normalize((skin * vec4<f32>(in.normal, 0.0)).xyz);
    var out: VsOut;
    out.clip = u.view_proj * vec4<f32>(world_pos, 1.0);
    out.world = world_pos;
    out.normal = world_n;
    out.color = in.color;
    out.emissive = in.emissive;
    out.light_space = u.light_view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = in.uv;
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
    let n = normalize(in.normal);
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

    var fly_total = vec3<f32>(0.0);
    for (var f = 0; f < 4; f = f + 1) {
        let to_fly = u.fly_pos[f].xyz - in.world;
        let fd = length(to_fly);
        let fl = normalize(to_fly);
        let atten = 1.0 / (0.4 + 0.6 * fd + 1.8 * fd * fd);
        fly_total = fly_total + max(dot(n, fl), 0.0) * u.fly_color.xyz * atten * u.fly_pos[f].w;
    }

    // Diffuse texture × vertex tint, both feeding the lighting.
    let tex = textureSample(diffuse_tex, diffuse_samp, in.uv);
    let albedo = tex.rgb * in.color;
    let lit = albedo * (u.ambient_color.xyz + lamp_diffuse + moon_diffuse + fly_total);
    return vec4<f32>(lit / (lit + vec3<f32>(1.0)), 1.0);
}
