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
    fade: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var portal_tex: texture_2d<f32>;
@group(1) @binding(1) var portal_samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn vs(@location(0) pos: vec3<f32>) -> VsOut {
    var out: VsOut;
    out.clip = vec4<f32>(pos, 1.0);
    return out;
}

@fragment
fn fs(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(i32(frag_pos.x), i32(frag_pos.y));
    let dest = textureLoad(portal_tex, texel, 0);
    return vec4<f32>(dest.rgb, u.fade.w);
}
