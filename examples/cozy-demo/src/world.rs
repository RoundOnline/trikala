//! The world: an endless, procedurally generated stepped landscape.
//! Terrain height comes from value noise, so it is defined for every
//! cell — the player can walk forever. The mesh is built in chunks
//! around the player and rebuilt as they cross a chunk boundary.

use crate::geometry::{push_box, push_quad, push_wall, Vertex, FLAT_NORMAL, QUAD_IDX};
use glam::Vec3;

const TREE_TOP: f32 = 1.67;
pub const TREE_SOLID_R: f32 = 0.55;
pub const CHUNK: i32 = 16; // tiles per chunk edge
const STEP: f32 = 1.2; // terrain quantises to this height step
pub const WORLD: i32 = 384; // tiles per lap — the landscape repeats every WORLD

// ---- procedural terrain ---------------------------------------------

/// Hash an integer lattice point to a value in [0, 1).
fn hash(gx: i32, gz: i32) -> f32 {
    let mut h = (gx.wrapping_mul(374761393)).wrapping_add(gz.wrapping_mul(668265263)) as u32;
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0x00ff_ffff) as f32 / 0x0100_0000 as f32
}

/// Value noise that tiles seamlessly: `cells` lattice cells span the
/// whole WORLD and the lattice index wraps — so terrain repeats exactly
/// every WORLD tiles, making the landscape a closed loop.
fn vnoise(gx: i32, gz: i32, cells: i32) -> f32 {
    let span = WORLD as f32 / cells as f32;
    let (fx, fz) = (gx as f32 / span, gz as f32 / span);
    let (x0, z0) = (fx.floor() as i32, fz.floor() as i32);
    let (tx, tz) = (fx - x0 as f32, fz - z0 as f32);
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sz = tz * tz * (3.0 - 2.0 * tz);
    let h = |ix: i32, iz: i32| hash(ix.rem_euclid(cells), iz.rem_euclid(cells));
    let a = h(x0, z0) + (h(x0 + 1, z0) - h(x0, z0)) * sx;
    let b = h(x0, z0 + 1) + (h(x0 + 1, z0 + 1) - h(x0, z0 + 1)) * sx;
    a + (b - a) * sz
}

/// Stepped terrain height of the tile at grid cell (gx, gz) — defined
/// for every cell, so the landscape is endless.
pub fn tile_height(gx: i32, gz: i32) -> f32 {
    let h = vnoise(gx, gz, 8) * 6.0 + vnoise(gx, gz, 24) * 1.6;
    (h / STEP).floor() * STEP
}

/// Is there a tree on the tile (gx, gz)? Deterministic and sparse.
pub fn tree_at(gx: i32, gz: i32) -> bool {
    let (wx, wz) = (gx.rem_euclid(WORLD), gz.rem_euclid(WORLD));
    hash(wx.wrapping_mul(7) + 1, wz.wrapping_mul(13) + 5) > 0.955
}

fn ground_at(x: f32, z: f32) -> f32 {
    tile_height(x.floor() as i32, z.floor() as i32)
}

/// Highest solid surface — terrain or a tree top — at (x, z).
pub fn solid_height(x: f32, z: f32) -> f32 {
    let mut h = ground_at(x, z);
    let (gx, gz) = (x.floor() as i32, z.floor() as i32);
    for dz in -1..=1 {
        for dx in -1..=1 {
            let (tx, tz) = (gx + dx, gz + dz);
            if tree_at(tx, tz) {
                let (cx, cz) = (tx as f32 + 0.5, tz as f32 + 0.5);
                if (x - cx) * (x - cx) + (z - cz) * (z - cz) < TREE_SOLID_R * TREE_SOLID_R {
                    h = h.max(tile_height(tx, tz) + TREE_TOP);
                }
            }
        }
    }
    h
}

// ---- meshing --------------------------------------------------------

fn push_tree(out: &mut Vec<Vertex>, pos: Vec3) {
    push_box(out, pos + Vec3::new(0.0, 0.45, 0.0), Vec3::new(0.12, 0.45, 0.12), [0.40, 0.29, 0.21]);
    push_box(out, pos + Vec3::new(0.0, TREE_TOP - 0.52, 0.0), Vec3::splat(0.52), [0.29, 0.49, 0.30]);
}

fn sky_color(y: f32) -> [f32; 3] {
    let t = ((y + 25.0) / 175.0).clamp(0.0, 1.0);
    Vec3::new(0.97, 0.63, 0.43).lerp(Vec3::new(0.15, 0.16, 0.33), t).to_array()
}

/// A big sky dome (four walls + a lid), centred on (cx, cz) so the
/// player is never able to walk out of it.
pub fn push_sky(out: &mut Vec<Vertex>, cx: f32, cz: f32) {
    let r = 140.0;
    let (lo, hi) = (-30.0, 150.0);
    let xz = [(-r, -r), (r, -r), (r, r), (-r, r)];
    for i in 0..4 {
        let (ax, az) = xz[i];
        let (bx, bz) = xz[(i + 1) % 4];
        let q = [
            Vec3::new(cx + ax, lo, cz + az),
            Vec3::new(cx + bx, lo, cz + bz),
            Vec3::new(cx + bx, hi, cz + bz),
            Vec3::new(cx + ax, hi, cz + az),
        ];
        for k in QUAD_IDX {
            out.push(Vertex { pos: q[k].to_array(), normal: FLAT_NORMAL, color: sky_color(q[k].y) });
        }
    }
    let lid = [
        Vec3::new(cx - r, hi, cz - r),
        Vec3::new(cx + r, hi, cz - r),
        Vec3::new(cx + r, hi, cz + r),
        Vec3::new(cx - r, hi, cz + r),
    ];
    push_quad(out, lid, FLAT_NORMAL, sky_color(hi));
}

/// Mesh one chunk — the CHUNK×CHUNK tiles of chunk (cx, cz): tile
/// tops, cliff faces, and trees.
pub fn build_chunk(cx: i32, cz: i32) -> Vec<Vertex> {
    let mut v = Vec::new();
    let cliff = [0.42, 0.33, 0.25];
    let (lo_x, lo_z) = (cx * CHUNK, cz * CHUNK);
    for gz in lo_z..lo_z + CHUNK {
        for gx in lo_x..lo_x + CHUNK {
            let h = tile_height(gx, gz);
            let (x, z) = (gx as f32, gz as f32);
            let base = if (gx + gz).rem_euclid(2) == 0 {
                [0.46, 0.63, 0.37]
            } else {
                [0.41, 0.57, 0.33]
            };
            let lift = (h * 0.012).clamp(0.0, 0.13); // higher ground reads lighter
            let color = [base[0] + lift, base[1] + lift, base[2] + lift * 0.6];
            push_quad(
                &mut v,
                [
                    Vec3::new(x, h, z),
                    Vec3::new(x + 1.0, h, z),
                    Vec3::new(x + 1.0, h, z + 1.0),
                    Vec3::new(x, h, z + 1.0),
                ],
                [0.0, 1.0, 0.0],
                color,
            );
            // a cliff face toward each lower neighbour
            for (dx, dz, ax, az, bx, bz, nrm) in [
                (1, 0, 1.0, 0.0, 1.0, 1.0, [1.0, 0.0, 0.0]),
                (-1, 0, 0.0, 0.0, 0.0, 1.0, [-1.0, 0.0, 0.0]),
                (0, 1, 0.0, 1.0, 1.0, 1.0, [0.0, 0.0, 1.0]),
                (0, -1, 0.0, 0.0, 1.0, 0.0, [0.0, 0.0, -1.0]),
            ] {
                let nh = tile_height(gx + dx, gz + dz);
                if nh < h {
                    let a = Vec3::new(x + ax, 0.0, z + az);
                    let b = Vec3::new(x + bx, 0.0, z + bz);
                    push_wall(&mut v, a, b, nh, h, nrm, cliff);
                }
            }
            if tree_at(gx, gz) {
                push_tree(&mut v, Vec3::new(x + 0.5, h, z + 0.5));
            }
        }
    }
    v
}
