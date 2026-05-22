// cozy-demo — 3D scene shader.
// Geometry with a real normal gets hemisphere lighting + a warm key
// light. Geometry with a zero normal (sky dome, blob shadows) is flat.

struct Camera { view_proj: mat4x4<f32>, };
@group(0) @binding(0) var<uniform> camera: Camera;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
) -> VsOut {
    var out: VsOut;
    out.clip = camera.view_proj * vec4<f32>(pos, 1.0);
    out.normal = normal;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var lit: vec3<f32>;
    if (dot(in.normal, in.normal) < 0.001) {
        // flat / emissive — the sky dome and the blob shadows
        lit = in.color;
    } else {
        let n = normalize(in.normal);
        let light_dir = normalize(vec3<f32>(0.45, 0.9, 0.35));
        let key = max(dot(n, light_dir), 0.0);
        // hemisphere ambient: brighter and cooler from the sky above
        let up = clamp(n.y * 0.5 + 0.5, 0.0, 1.0);
        let ambient = mix(vec3<f32>(0.30, 0.27, 0.33),
                          vec3<f32>(0.52, 0.55, 0.62), up);
        let warm = vec3<f32>(1.05, 0.92, 0.70) * key * 0.85;
        lit = in.color * (ambient + warm);
    }
    let col = pow(max(lit, vec3<f32>(0.0, 0.0, 0.0)), vec3<f32>(2.2, 2.2, 2.2));
    return vec4<f32>(col, 1.0);
}
