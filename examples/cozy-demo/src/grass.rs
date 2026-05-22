//! Wind-swept grass around the player: short ground grass, medium
//! tufts and tall head-high clumps. Dense thickets alternate with
//! open ground, and blades bend and flatten where the player treads.
//! Plain triangles rebuilt each frame — no shader, pass or new GPU
//! resources. Kept light with few, wide blades and a distance fade.

use crate::geometry::Vertex;
use crate::world::{tile_height, tree_at};
use glam::Vec3;

const REACH: i32 = 14; // grass cells generated in each direction around the player
const SPACING: f32 = 0.5; // world units between grass cells
const SWAY: f32 = 0.15; // wind tip-drift per unit of blade height
const TRAMPLE_R: f32 = 1.1; // how far from the player grass is pressed down

/// One f32 in [0, 1), hashed from a grid cell plus a salt.
fn hash(cx: i32, cz: i32, salt: u32) -> f32 {
    let mut h = cx
        .wrapping_mul(374761393)
        .wrapping_add(cz.wrapping_mul(668265263))
        .wrapping_add(salt.wrapping_mul(2246822519) as i32) as u32;
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0x00ff_ffff) as f32 / 0x0100_0000 as f32
}

/// Smooth low-frequency field in [0, 1] — the raw lushness of the
/// meadow here. A coarse lattice gives patches tens of metres wide.
fn lushness(x: f32, z: f32) -> f32 {
    let s = 1.0 / 13.0;
    let (fx, fz) = (x * s, z * s);
    let (x0, z0) = (fx.floor() as i32, fz.floor() as i32);
    let (tx, tz) = (fx - x0 as f32, fz - z0 as f32);
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sz = tz * tz * (3.0 - 2.0 * tz);
    let c = |ix: i32, iz: i32| hash(ix, iz, 101);
    let a = c(x0, z0) + (c(x0 + 1, z0) - c(x0, z0)) * sx;
    let b = c(x0, z0 + 1) + (c(x0 + 1, z0 + 1) - c(x0, z0 + 1)) * sx;
    a + (b - a) * sz
}

/// Tip of a blade of height `h` rooted at `root`: wind `drift` plus the
/// press-down from the player at `feet`. Blades close to the player
/// splay outward and flatten — the look of grass being trodden on.
fn blade_tip(root: Vec3, h: f32, drift: Vec3, feet: Vec3) -> Vec3 {
    let away = Vec3::new(root.x - feet.x, 0.0, root.z - feet.z);
    let dist = away.length();
    let mut t = (1.0 - dist / TRAMPLE_R).max(0.0);
    t *= t; // soft edge — only blades close in really flatten
    let push = if dist > 1e-3 {
        away / dist * (t * h * 0.95)
    } else {
        Vec3::ZERO
    };
    Vec3::new(
        root.x + drift.x + push.x,
        root.y + h * (1.0 - t * 0.8),
        root.z + drift.z + push.z,
    )
}

/// One grass-blade triangle from `root` up to `tip`.
fn push_blade(
    out: &mut Vec<Vertex>,
    root: Vec3,
    face: f32,
    half_w: f32,
    tip: Vec3,
    root_c: [f32; 3],
    tip_c: [f32; 3],
) {
    let (ox, oz) = (face.cos() * half_w, face.sin() * half_w);
    let n = [0.0, 1.0, 0.0]; // up-facing, so the scene light treats it as grass
    out.push(Vertex { pos: [root.x - ox, root.y, root.z - oz], normal: n, color: root_c });
    out.push(Vertex { pos: [root.x + ox, root.y, root.z + oz], normal: n, color: root_c });
    out.push(Vertex { pos: tip.to_array(), normal: n, color: tip_c });
}

/// Append a patch of grass around `center`, swaying with `time` and
/// bending away where the player treads.
pub fn push_grass(out: &mut Vec<Vertex>, center: Vec3, time: f32) {
    let wind = Vec3::new(0.72, 0.0, 0.69); // world-space wind heading
    let pcx = (center.x / SPACING).floor() as i32;
    let pcz = (center.z / SPACING).floor() as i32;
    for dz in -REACH..=REACH {
        for dx in -REACH..=REACH {
            let (cx, cz) = (pcx + dx, pcz + dz);
            // density fades with distance — lush underfoot, thinning to a
            // soft edge — which also keeps the triangle count down.
            let rr = ((dx * dx + dz * dz) as f32).sqrt() / REACH as f32;
            let near = (1.0 - rr).clamp(0.0, 1.0);
            let bx = (cx as f32 + 0.5) * SPACING + (hash(cx, cz, 1) - 0.5) * SPACING * 0.8;
            let bz = (cz as f32 + 0.5) * SPACING + (hash(cx, cz, 2) - 0.5) * SPACING * 0.8;
            // sharpen the lushness field into clear thick-patch vs open
            // ground, so dense thickets alternate as the player roams.
            let lush = ((lushness(bx, bz) - 0.40) / 0.32).clamp(0.0, 1.0);
            let dense = lush * lush * (3.0 - 2.0 * lush);
            let keep = (0.10 + dense * 0.90) * (0.35 + near * 0.65);
            if hash(cx, cz, 0) > keep {
                continue;
            }
            let (tx, tz) = (bx.floor() as i32, bz.floor() as i32);
            if tree_at(tx, tz) {
                continue; // don't sprout grass inside a tree trunk
            }
            let base = Vec3::new(bx, tile_height(tx, tz), bz);

            let face = hash(cx, cz, 3) * std::f32::consts::TAU;
            let phase = hash(cx, cz, 5) * std::f32::consts::TAU;
            let tint = hash(cx, cz, 6) * 0.12;
            let kind = hash(cx, cz, 7);
            // thick patches grow mostly medium + tall grass; open ground short
            let short_cut = 0.90 - dense * 0.62;
            let med_cut = 0.985 - dense * 0.37;

            if kind < short_cut {
                // short ground grass — a single wide blade
                let h = 0.35 + hash(cx, cz, 4) * 0.32;
                let drift = wind * (SWAY * h * (time * 1.9 + phase).sin());
                let root_c = [0.23 + tint, 0.35 + tint, 0.15 + tint * 0.5];
                let tip_c = [0.50 + tint, 0.69 + tint, 0.30 + tint * 0.5];
                push_blade(out, base, face, 0.07, blade_tip(base, h, drift, center), root_c, tip_c);
            } else if kind < med_cut {
                // medium tuft — two blades, knee-to-thigh high
                let root_c = [0.20 + tint, 0.33 + tint, 0.14 + tint * 0.5];
                let tip_c = [0.47 + tint, 0.66 + tint, 0.29 + tint * 0.5];
                for k in 0..2u32 {
                    let h = 0.78 + hash(cx, cz, 10 + k) * 0.42;
                    let f = face + k as f32 * 1.3;
                    let root = base + Vec3::new(f.cos() * 0.08, 0.0, f.sin() * 0.08);
                    let drift = wind * (SWAY * h * (time * 1.6 + phase + k as f32).sin());
                    push_blade(out, root, f, 0.09, blade_tip(root, h, drift, center), root_c, tip_c);
                }
            } else {
                // tall grass — a clump of head-high golden blades
                let root_c = [0.33 + tint, 0.34 + tint, 0.16 + tint * 0.5];
                let tip_c = [0.74 + tint, 0.69 + tint, 0.34 + tint * 0.5];
                for k in 0..2u32 {
                    let h = 1.5 + hash(cx, cz, 20 + k) * 0.8;
                    let f = face + k as f32 * 2.4;
                    let root = base + Vec3::new(f.cos() * 0.13, 0.0, f.sin() * 0.13);
                    let drift = wind * (SWAY * h * (time * 1.3 + phase + k as f32 * 0.7).sin());
                    push_blade(out, root, f, 0.12, blade_tip(root, h, drift, center), root_c, tip_c);
                }
            }

            // pack thick patches fuller with a few wide understory blades
            let fill = (dense * 2.5) as u32;
            for e in 0..fill {
                let jx = (hash(cx, cz, 40 + e) - 0.5) * SPACING * 0.9;
                let jz = (hash(cx, cz, 45 + e) - 0.5) * SPACING * 0.9;
                let root = base + Vec3::new(jx, 0.0, jz);
                let h = 0.4 + hash(cx, cz, 50 + e) * 0.7;
                let f = hash(cx, cz, 55 + e) * std::f32::consts::TAU;
                let ph = hash(cx, cz, 60 + e) * std::f32::consts::TAU;
                let drift = wind * (SWAY * h * (time * 1.8 + ph).sin());
                let root_c = [0.21 + tint, 0.34 + tint, 0.14 + tint * 0.5];
                let tip_c = [0.49 + tint, 0.67 + tint, 0.30 + tint * 0.5];
                push_blade(out, root, f, 0.08, blade_tip(root, h, drift, center), root_c, tip_c);
            }
        }
    }
}
