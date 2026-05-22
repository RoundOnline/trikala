// cozy-demo — HUD overlay shader. Vertices arrive already in clip
// space (x, y in -1..1), so the camera is not involved. Flat-coloured,
// with per-vertex opacity packed into normal.x by hud.rs.

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) alpha: f32,
};

@vertex
fn vs_main(
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
) -> VsOut {
    var out: VsOut;
    out.clip = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
    out.color = color;
    out.alpha = normal.x; // hud.rs packs opacity into normal.x
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, in.alpha);
}
