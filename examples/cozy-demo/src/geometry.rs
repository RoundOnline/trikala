//! Mesh primitives — the small box/quad builders every other module
//! uses to assemble geometry. No game logic lives here.

use glam::{Mat4, Vec3};

/// A zero normal flags a vertex as flat-shaded — used by the sky dome
/// and the blob shadows. The shader keys off this (see `scene.wgsl`).
pub const FLAT_NORMAL: [f32; 3] = [0.0, 0.0, 0.0];

/// Vertex order for the two triangles of a quad.
pub const QUAD_IDX: [usize; 6] = [0, 1, 2, 0, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

/// Emit one quad (two triangles) from four corners.
pub fn push_quad(out: &mut Vec<Vertex>, corners: [Vec3; 4], normal: [f32; 3], color: [f32; 3]) {
    for i in QUAD_IDX {
        out.push(Vertex { pos: corners[i].to_array(), normal, color });
    }
}

/// Lerp a colour toward white by `f` (0 = unchanged, 1 = white). Drives
/// the hit-flash on the character and the boss.
pub fn flash_color(c: [f32; 3], f: f32) -> [f32; 3] {
    [c[0] + (1.0 - c[0]) * f, c[1] + (1.0 - c[1]) * f, c[2] + (1.0 - c[2]) * f]
}

/// Append one axis-aligned box (6 faces, 12 triangles).
pub fn push_box(out: &mut Vec<Vertex>, center: Vec3, half: Vec3, color: [f32; 3]) {
    box_inner(out, center, half, color, false);
}

/// Like `push_box`, but drawn flat — it ignores lighting and shows its
/// full colour. For eyes, cracks, anything that should self-illuminate.
pub fn push_box_emissive(out: &mut Vec<Vertex>, center: Vec3, half: Vec3, color: [f32; 3]) {
    box_inner(out, center, half, color, true);
}

fn box_inner(out: &mut Vec<Vertex>, center: Vec3, half: Vec3, color: [f32; 3], emissive: bool) {
    let faces = [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (Vec3::NEG_X, Vec3::Y, Vec3::Z),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (Vec3::NEG_Y, Vec3::Z, Vec3::X),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::X, Vec3::Y),
    ];
    for (n, u, v) in faces {
        let fc = center + n * half.dot(n.abs());
        let uu = u * half.dot(u.abs());
        let vv = v * half.dot(v.abs());
        let normal = if emissive { FLAT_NORMAL } else { n.to_array() };
        push_quad(out, [fc - uu - vv, fc + uu - vv, fc + uu + vv, fc - uu + vv], normal, color);
    }
}

/// Append one vertical quad — a cliff face — rising from `lo` to `hi`
/// between the ground points `a` and `b` (their y is ignored).
pub fn push_wall(out: &mut Vec<Vertex>, a: Vec3, b: Vec3, lo: f32, hi: f32, normal: [f32; 3], color: [f32; 3]) {
    push_quad(
        out,
        [
            Vec3::new(a.x, lo, a.z),
            Vec3::new(b.x, lo, b.z),
            Vec3::new(b.x, hi, b.z),
            Vec3::new(a.x, hi, a.z),
        ],
        normal,
        color,
    );
}

/// Append `part` to `dst`, rotated by `rot` about `pivot`.
pub fn push_rotated(dst: &mut Vec<Vertex>, part: &[Vertex], pivot: Vec3, rot: Mat4) {
    for v in part {
        let p = rot.transform_point3(Vec3::from(v.pos) - pivot) + pivot;
        let n = rot.transform_vector3(Vec3::from(v.normal));
        dst.push(Vertex { pos: p.to_array(), normal: n.to_array(), color: v.color });
    }
}

/// A flat, dark blob shadow on the ground, nudged away from the light.
pub fn push_blob(out: &mut Vec<Vertex>, ground_pos: Vec3, radius: f32) {
    let c = ground_pos
        + Vec3::new(-0.62, 0.0, -0.48) * (radius * 0.55)
        + Vec3::new(0.0, 0.02, 0.0);
    let col = [0.17, 0.20, 0.15];
    let seg = 12;
    for i in 0..seg {
        let a0 = (i as f32) / (seg as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32) / (seg as f32) * std::f32::consts::TAU;
        let r0 = c + Vec3::new(a0.cos() * radius, 0.0, a0.sin() * radius);
        let r1 = c + Vec3::new(a1.cos() * radius, 0.0, a1.sin() * radius);
        for p in [c, r0, r1] {
            out.push(Vertex { pos: p.to_array(), normal: FLAT_NORMAL, color: col });
        }
    }
}
