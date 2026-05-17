//! Mood prototype v6 — true portal between two worlds.
//!
//! Two scenes live in separate vertex buffers but share the same
//! local coordinate frame. A Doraemon-style door sits at the same
//! local position in both worlds. The door renders as a quad that
//! samples a texture: that texture is the OTHER world rendered from
//! a virtual camera, so looking through the door you see the other
//! world from the corresponding perspective. Crossing the door's
//! plane teleports the player by swapping which world is active —
//! position and orientation pass through unchanged because the two
//! worlds share coordinates.
//!
//! Controls
//!   WASD / Arrows — move / look
//!   Mouse         — look
//!   Space         — jump
//!   F / LMB       — attack
//!   V             — toggle 1st / 3rd person
//!   Esc           — exit

mod character;

use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};
use character::{Character, SkinVertex};

// ─────────────────────────────────────────────────────────────────
// SECTION 1 — Vertex + Uniform types
// ─────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    emissive: f32,
}

const FIREFLY_COUNT: usize = 4;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    light_view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    lamp_pos: [f32; 4],
    lamp_color: [f32; 4],
    moon_dir: [f32; 4],
    moon_color: [f32; 4],
    // ambient_color.w = shadow strength (1.0 normal, 0.0 disables sampling
    // — used during portal view so we don't sample the wrong world's
    // shadow map).
    ambient_color: [f32; 4],
    fly_pos: [[f32; 4]; FIREFLY_COUNT],
    fly_color: [f32; 4],
}

// ─────────────────────────────────────────────────────────────────
// SECTION 2 — Geometry primitives
// ─────────────────────────────────────────────────────────────────

fn push_box(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    transform: Mat4,
    half: Vec3,
    color: Vec3,
    emissive: f32,
) {
    let base = verts.len() as u32;
    let faces: [([[i32; 3]; 4], [f32; 3]); 6] = [
        ([[1, 1, 1], [1, -1, 1], [1, -1, -1], [1, 1, -1]], [1.0, 0.0, 0.0]),
        ([[-1, 1, -1], [-1, -1, -1], [-1, -1, 1], [-1, 1, 1]], [-1.0, 0.0, 0.0]),
        ([[-1, 1, -1], [-1, 1, 1], [1, 1, 1], [1, 1, -1]], [0.0, 1.0, 0.0]),
        ([[-1, -1, 1], [-1, -1, -1], [1, -1, -1], [1, -1, 1]], [0.0, -1.0, 0.0]),
        ([[-1, 1, 1], [-1, -1, 1], [1, -1, 1], [1, 1, 1]], [0.0, 0.0, 1.0]),
        ([[1, 1, -1], [1, -1, -1], [-1, -1, -1], [-1, 1, -1]], [0.0, 0.0, -1.0]),
    ];
    for (i, (corners, n)) in faces.iter().enumerate() {
        let local_n = Vec3::from_array(*n);
        let world_n = transform.transform_vector3(local_n).normalize();
        for corner in corners {
            let local = Vec3::new(
                corner[0] as f32 * half.x,
                corner[1] as f32 * half.y,
                corner[2] as f32 * half.z,
            );
            let world = transform.transform_point3(local);
            verts.push(Vertex {
                pos: [world.x, world.y, world.z],
                normal: [world_n.x, world_n.y, world_n.z],
                color: [color.x, color.y, color.z],
                emissive,
            });
        }
        let f = base + (i as u32) * 4;
        indices.extend_from_slice(&[f, f + 1, f + 2, f, f + 2, f + 3]);
    }
}

fn push_cube(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    center: Vec3,
    size: Vec3,
    color: Vec3,
    emissive: f32,
) {
    push_box(verts, indices, Mat4::from_translation(center), size * 0.5, color, emissive);
}

fn push_plane(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    center: Vec3,
    size: f32,
    color: Vec3,
) {
    let base = verts.len() as u32;
    let h = size * 0.5;
    let n = [0.0, 1.0, 0.0];
    let corners = [
        Vec3::new(-h, 0.0, -h),
        Vec3::new(-h, 0.0, h),
        Vec3::new(h, 0.0, h),
        Vec3::new(h, 0.0, -h),
    ];
    for c in corners {
        let p = center + c;
        verts.push(Vertex {
            pos: [p.x, p.y, p.z],
            normal: n,
            color: [color.x, color.y, color.z],
            emissive: 0.0,
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn push_tree(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, x: f32, z: f32, scale: f32, canopy: Vec3) {
    let trunk_h = 1.0 * scale;
    let trunk_w = 0.10 * scale;
    let canopy_h = 1.2 * scale;
    let canopy_w = 0.7 * scale;
    push_cube(verts, indices, Vec3::new(x, trunk_h * 0.5, z),
        Vec3::new(trunk_w, trunk_h, trunk_w), Vec3::new(0.18, 0.10, 0.06), 0.0);
    push_cube(verts, indices, Vec3::new(x, trunk_h + canopy_h * 0.5, z),
        Vec3::new(canopy_w, canopy_h, canopy_w), canopy, 0.0);
}

// The pink Doraemon door — a portal frame: two posts, a top lintel,
// and a threshold slab. The "magic surface" inside the frame is a
// separate quad rendered by the portal pipeline (not part of the
// world geometry); see PORTAL_HALF_W / PORTAL_HALF_H.
fn push_doraemon_door(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, center: Vec3) {
    let pink = Vec3::new(0.95, 0.55, 0.75);
    let pink_dark = Vec3::new(0.80, 0.42, 0.62);
    let cx = center.x;
    let cy = center.y;
    let cz = center.z;
    let half_w = PORTAL_HALF_W;
    let half_h = PORTAL_HALF_H;
    let post_t = PORTAL_POST_T;
    let depth_t = PORTAL_DEPTH_T;

    // Left post
    push_cube(verts, indices,
        Vec3::new(cx - half_w - post_t, cy, cz),
        Vec3::new(post_t * 2.0, half_h * 2.0 + post_t * 4.0, depth_t * 2.0),
        pink, 0.0);
    // Right post
    push_cube(verts, indices,
        Vec3::new(cx + half_w + post_t, cy, cz),
        Vec3::new(post_t * 2.0, half_h * 2.0 + post_t * 4.0, depth_t * 2.0),
        pink, 0.0);
    // Top lintel
    push_cube(verts, indices,
        Vec3::new(cx, cy + half_h + post_t, cz),
        Vec3::new(half_w * 2.0 + post_t * 4.0, post_t * 2.0, depth_t * 2.0),
        pink, 0.0);
    // Bottom threshold (darker so it reads as "floor")
    push_cube(verts, indices,
        Vec3::new(cx, cy - half_h - post_t, cz),
        Vec3::new(half_w * 2.0 + post_t * 4.0, post_t * 2.0, depth_t * 2.0),
        pink_dark, 0.0);
}

// ─────────────────────────────────────────────────────────────────
// SECTION 3 — Worlds + portal config
// ─────────────────────────────────────────────────────────────────

// The portal sits at the same local coordinates in both worlds so
// teleport is a pure swap of "which world is active" — no position
// transform needed. The door faces +z (player approaches from +z,
// walks toward -z to step through).
const PORTAL_POS: Vec3 = Vec3::new(2.5, 1.0, -3.0);
const PORTAL_HALF_W: f32 = 0.50;
const PORTAL_HALF_H: f32 = 1.00;
const PORTAL_NORMAL: Vec3 = Vec3::new(0.0, 0.0, 1.0);
// Post dimensions used by both the door mesh and player-collision
// so the two stay in sync when the door's size changes.
const PORTAL_POST_T: f32 = 0.08; // half-thickness of vertical posts
const PORTAL_DEPTH_T: f32 = 0.06; // half-depth (z) of the door frame
const PLAYER_RADIUS: f32 = 0.20;

#[derive(Copy, Clone)]
struct WorldEnv {
    lamp_pos: Vec3,
    lamp_color: [f32; 4],
    moon_dir: Vec3,
    moon_color: [f32; 4],
    ambient: [f32; 3],
    sky: wgpu::Color,
    // Origin of the directional-light camera for shadow mapping.
    light_origin: Vec3,
}

// World A — night forest with the desk, lamp, diorama, and trees.
fn build_world_a() -> (Vec<Vertex>, Vec<u32>, WorldEnv) {
    let mut v = Vec::new();
    let mut i = Vec::new();

    push_plane(&mut v, &mut i, Vec3::ZERO, 60.0, Vec3::new(0.08, 0.07, 0.06));

    // Moon
    push_cube(&mut v, &mut i, Vec3::new(8.0, 18.0, -25.0),
        Vec3::new(3.5, 3.5, 3.5), Vec3::new(2.2, 2.5, 3.0), 1.0);

    // Desk + objects + lamp
    push_cube(&mut v, &mut i, Vec3::new(0.0, 0.75, 0.0),
        Vec3::new(2.5, 0.05, 1.2), Vec3::new(0.22, 0.14, 0.10), 0.0);
    for &(lx, lz) in &[(-1.2, -0.55), (1.2, -0.55), (-1.2, 0.55), (1.2, 0.55)] {
        push_cube(&mut v, &mut i, Vec3::new(lx, 0.375, lz),
            Vec3::new(0.05, 0.75, 0.05), Vec3::new(0.16, 0.10, 0.07), 0.0);
    }
    push_cube(&mut v, &mut i, Vec3::new(-0.75, 0.81, -0.3),
        Vec3::new(0.45, 0.05, 0.30), Vec3::new(0.26, 0.16, 0.10), 0.0);
    push_cube(&mut v, &mut i, Vec3::new(-0.75, 0.86, -0.3),
        Vec3::new(0.42, 0.04, 0.28), Vec3::new(0.55, 0.20, 0.14), 0.0);
    push_cube(&mut v, &mut i, Vec3::new(0.75, 0.90, -0.3),
        Vec3::new(0.18, 0.25, 0.18), Vec3::new(0.50, 0.32, 0.22), 0.0);

    // Diorama
    push_cube(&mut v, &mut i, Vec3::new(0.0, 0.82, 0.20),
        Vec3::new(0.80, 0.06, 0.80), Vec3::new(0.18, 0.13, 0.10), 0.0);
    push_cube(&mut v, &mut i, Vec3::new(0.0, 0.90, 0.20),
        Vec3::new(0.55, 0.08, 0.55), Vec3::new(0.32, 0.42, 0.20), 0.0);
    push_cube(&mut v, &mut i, Vec3::new(0.06, 1.01, 0.16),
        Vec3::new(0.10, 0.14, 0.10), Vec3::new(0.50, 0.32, 0.22), 0.0);
    push_cube(&mut v, &mut i, Vec3::new(0.06, 1.14, 0.16),
        Vec3::new(0.10, 0.10, 0.10), Vec3::new(0.85, 0.65, 0.50), 0.0);
    push_cube(&mut v, &mut i, Vec3::new(-0.16, 1.05, 0.30),
        Vec3::new(0.04, 0.22, 0.04), Vec3::new(0.18, 0.10, 0.06), 0.0);
    push_cube(&mut v, &mut i, Vec3::new(-0.16, 1.24, 0.30),
        Vec3::new(0.24, 0.22, 0.24), Vec3::new(0.22, 0.42, 0.18), 0.0);

    push_cube(&mut v, &mut i, Vec3::new(-1.0, 0.82, -0.3),
        Vec3::new(0.18, 0.04, 0.18), Vec3::new(0.15, 0.12, 0.10), 0.0);
    push_cube(&mut v, &mut i, Vec3::new(-1.0, 1.18, -0.3),
        Vec3::new(0.04, 0.70, 0.04), Vec3::new(0.15, 0.12, 0.10), 0.0);
    let lamp_pos = Vec3::new(-1.0, 1.50, -0.3);
    push_cube(&mut v, &mut i, lamp_pos,
        Vec3::new(0.26, 0.14, 0.26), Vec3::new(4.5, 3.0, 1.5), 1.0);

    // Trees
    let trees: [(f32, f32, f32); 18] = [
        (5.5, -5.0, 1.0), (-5.0, -3.0, 1.2), (7.5, 2.0, 0.9),
        (-6.0, 4.0, 1.1), (-3.0, -7.5, 1.3), (4.5, 6.0, 1.0),
        (9.0, -3.0, 0.8), (-8.0, -6.0, 1.2), (-9.0, 2.5, 1.4),
        (10.0, 5.0, 1.1),                     (-2.0, 9.0, 1.3),
        (12.0, -8.0, 1.2), (-12.0, -1.0, 1.0), (-7.0, 8.0, 0.9),
        (13.0, 0.0, 1.3),                     (-13.0, 6.0, 1.2),
        (-4.0, 12.0, 1.1),                    (8.5, 9.0, 1.0),
    ];
    for &(x, z, s) in &trees {
        push_tree(&mut v, &mut i, x, z, s, Vec3::new(0.18, 0.32, 0.14));
    }

    // Rocks
    for &(x, z, s) in &[(2.5, 1.8, 0.30_f32), (-1.5, 2.4, 0.25),
                        (1.5, -2.0, 0.20), (-2.0, -1.5, 0.28)] {
        push_cube(&mut v, &mut i, Vec3::new(x, s * 0.5, z),
            Vec3::new(s * 1.5, s, s * 1.3), Vec3::new(0.20, 0.18, 0.16), 0.0);
    }

    // The Doraemon door is NOT part of either world's mesh — it's
    // drawn separately by the renderer in only the current world's
    // main pass, so the portal-texture view of the other world
    // doesn't show a second door inside it ("door inside door").

    let env = WorldEnv {
        lamp_pos,
        lamp_color: [5.0, 3.2, 1.6, 1.0],
        moon_dir: Vec3::new(8.0, 18.0, -25.0).normalize(),
        moon_color: [0.45, 0.55, 0.85, 1.0],
        ambient: [0.05, 0.07, 0.12],
        sky: wgpu::Color { r: 0.012, g: 0.016, b: 0.028, a: 1.0 },
        light_origin: Vec3::new(8.0, 18.0, -25.0),
    };
    (v, i, env)
}

// World B — sunlit meadow with the same overall scale and bounds as
// World A, but a completely different mood: bright sun overhead,
// pale-green grass, scattered flowers, a few light birch trees,
// and a small fountain instead of the desk. Same Doraemon door in
// the same spot so the player exits at the matching position.
fn build_world_b() -> (Vec<Vertex>, Vec<u32>, WorldEnv) {
    let mut v = Vec::new();
    let mut i = Vec::new();

    // Bright grass ground.
    push_plane(&mut v, &mut i, Vec3::ZERO, 60.0, Vec3::new(0.32, 0.55, 0.22));

    // Sun (warm-white emissive disc up high — direction roughly
    // matches the directional light below).
    push_cube(&mut v, &mut i, Vec3::new(-12.0, 22.0, 18.0),
        Vec3::new(4.0, 4.0, 4.0), Vec3::new(3.0, 2.7, 2.0), 1.0);

    // Fountain at origin instead of the desk — basin, water, spout.
    push_cube(&mut v, &mut i, Vec3::new(0.0, 0.10, 0.0),
        Vec3::new(2.8, 0.20, 2.8), Vec3::new(0.78, 0.74, 0.68), 0.0);
    push_cube(&mut v, &mut i, Vec3::new(0.0, 0.16, 0.0),
        Vec3::new(2.4, 0.10, 2.4), Vec3::new(0.45, 0.65, 0.85), 0.30);
    // Pillar
    push_cube(&mut v, &mut i, Vec3::new(0.0, 0.55, 0.0),
        Vec3::new(0.40, 0.80, 0.40), Vec3::new(0.85, 0.82, 0.74), 0.0);
    // Cap
    push_cube(&mut v, &mut i, Vec3::new(0.0, 1.00, 0.0),
        Vec3::new(0.70, 0.10, 0.70), Vec3::new(0.85, 0.82, 0.74), 0.0);
    // Water spout (small bright blue cube on top)
    push_cube(&mut v, &mut i, Vec3::new(0.0, 1.18, 0.0),
        Vec3::new(0.20, 0.30, 0.20), Vec3::new(0.55, 0.75, 0.95), 0.40);

    // Birch trees — light bark, pale yellow-green canopy.
    let birches: [(f32, f32, f32); 16] = [
        (5.5, -5.0, 1.0), (-5.0, -3.0, 1.2), (7.5, 2.0, 0.9),
        (-6.0, 4.0, 1.1), (-3.0, -7.5, 1.3), (4.5, 6.0, 1.0),
        (9.0, -3.0, 0.8), (-8.0, -6.0, 1.2), (-9.0, 2.5, 1.4),
        (10.0, 5.0, 1.1),                     (-2.0, 9.0, 1.3),
        (12.0, -8.0, 1.2), (-12.0, -1.0, 1.0), (13.0, 0.0, 1.3),
        (-13.0, 6.0, 1.2),                    (8.5, 9.0, 1.0),
    ];
    for &(x, z, s) in &birches {
        // Override the dark trunk with a paler birch trunk.
        let trunk_h = 1.0 * s;
        let trunk_w = 0.10 * s;
        let canopy_h = 1.2 * s;
        let canopy_w = 0.7 * s;
        push_cube(&mut v, &mut i, Vec3::new(x, trunk_h * 0.5, z),
            Vec3::new(trunk_w, trunk_h, trunk_w), Vec3::new(0.88, 0.85, 0.78), 0.0);
        push_cube(&mut v, &mut i, Vec3::new(x, trunk_h + canopy_h * 0.5, z),
            Vec3::new(canopy_w, canopy_h, canopy_w), Vec3::new(0.55, 0.78, 0.32), 0.0);
    }

    // Scattered wildflowers — tiny coloured emissive cubes for pop.
    let flowers: [(f32, f32, [f32; 3]); 14] = [
        (2.2, 1.8, [0.95, 0.45, 0.55]),  (-1.5, 2.4, [0.90, 0.78, 0.30]),
        (1.5, -2.0, [0.78, 0.45, 0.95]), (-2.0, -1.5, [0.95, 0.85, 0.40]),
        (3.5, 4.0, [0.95, 0.45, 0.55]),  (-3.2, 5.2, [0.90, 0.78, 0.30]),
        (4.8, -3.5, [0.78, 0.45, 0.95]), (-4.5, -2.0, [0.95, 0.50, 0.30]),
        (6.0, 5.5, [0.95, 0.85, 0.40]),  (-6.5, 1.0, [0.78, 0.45, 0.95]),
        (5.5, -8.0, [0.95, 0.45, 0.55]), (-1.8, -9.0, [0.90, 0.78, 0.30]),
        (8.0, -1.5, [0.95, 0.85, 0.40]), (-7.5, 3.5, [0.95, 0.45, 0.55]),
    ];
    for &(x, z, c) in &flowers {
        push_cube(&mut v, &mut i, Vec3::new(x, 0.08, z),
            Vec3::new(0.10, 0.16, 0.10), Vec3::new(c[0], c[1], c[2]), 0.0);
    }

    // A small wooden bench near the door so World B has something to
    // walk toward.
    push_cube(&mut v, &mut i, Vec3::new(2.5, 0.20, -1.0),
        Vec3::new(1.5, 0.08, 0.40), Vec3::new(0.58, 0.42, 0.28), 0.0);
    for &(dx, dz) in &[(-0.65, -0.15), (0.65, -0.15), (-0.65, 0.15), (0.65, 0.15)] {
        push_cube(&mut v, &mut i, Vec3::new(2.5 + dx, 0.10, -1.0 + dz),
            Vec3::new(0.06, 0.20, 0.06), Vec3::new(0.42, 0.30, 0.20), 0.0);
    }

    // World B's "lamp" lives at the fountain spout — a soft cool
    // point light so the basin has a hint of its own glow even at
    // noon.
    let lamp_pos = Vec3::new(0.0, 1.25, 0.0);

    let env = WorldEnv {
        lamp_pos,
        lamp_color: [1.2, 1.8, 2.5, 1.0],
        moon_dir: Vec3::new(-12.0, 22.0, 18.0).normalize(),
        moon_color: [1.40, 1.30, 1.05, 1.0],
        ambient: [0.50, 0.55, 0.58],
        sky: wgpu::Color { r: 0.62, g: 0.78, b: 0.92, a: 1.0 },
        light_origin: Vec3::new(-12.0, 22.0, 18.0),
    };
    (v, i, env)
}

// The Doraemon door geometry as its own mesh — rendered ONLY in
// the current world's main pass + shadow pass, never inside the
// portal-texture view of the other world. (If it were part of
// each world's mesh, looking through the portal would reveal the
// other side's door frame inside the portal itself — a "door
// inside door" duplication that breaks the window illusion.)
fn build_door_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    push_doraemon_door(&mut v, &mut i, PORTAL_POS);
    (v, i)
}

// Each firefly carries its own position and velocity so it drifts
// independently. A soft spring pulls them toward the player (with
// a minimum stand-off so they don't all collapse onto the head),
// plus per-firefly sin-noise creates organic wandering. The result
// reads as four little creatures with minds of their own that just
// happen to like staying near you.
#[derive(Copy, Clone)]
struct Firefly {
    pos: Vec3,
    prev_pos: Vec3,
    vel: Vec3,
    world: u8, // 0 = A, 1 = B — flips when the firefly walks through the door
    phase: [f32; 3],
}

fn step_fireflies(
    flies: &mut [Firefly; FIREFLY_COUNT],
    player_pos: Vec3,
    player_world: u8,
    dt: f32,
    t: f32,
) {
    let player_target = player_pos + Vec3::new(0.0, 0.9, 0.0);
    let door_waypoint = PORTAL_POS + Vec3::new(0.0, -0.3, 0.0);
    for fly in flies.iter_mut() {
        fly.prev_pos = fly.pos;
        // A firefly that's already in the same world as the player
        // is "with" them; one that's in the other world needs to go
        // through the door first. The world tag — not the spatial
        // position — decides which side of the portal counts as the
        // way home, so the firefly seeks the door whenever it's
        // separated from the player by a teleport.
        let need_to_cross = fly.world != player_world;
        let target = if need_to_cross { door_waypoint } else { player_target };
        let to_target = target - fly.pos;
        let dist = to_target.length();
        let dir = if dist > 0.001 { to_target / dist } else { Vec3::ZERO };
        let pull = if need_to_cross {
            dir * 7.0
        } else if dist > 1.6 {
            dir * 5.0
        } else if dist < 0.45 {
            -dir * 3.0
        } else {
            dir * 0.6
        };
        let p = fly.phase;
        let wander_scale = if need_to_cross { 0.25 } else { 1.0 };
        let wander = Vec3::new(
            (t * 1.7 + p[0]).sin() * 2.2 + (t * 0.9 + p[1]).cos() * 1.0,
            (t * 2.3 + p[1]).sin() * 1.4 + (t * 1.1 + p[2]).cos() * 0.5,
            (t * 1.3 + p[2]).sin() * 2.2 + (t * 0.7 + p[0]).cos() * 1.0,
        ) * wander_scale;
        fly.vel += (pull + wander) * dt;
        fly.vel *= (1.0 - 1.5 * dt).max(0.0);
        let speed = fly.vel.length();
        let max_speed = if need_to_cross { 6.0 } else { 4.0 };
        if speed > max_speed {
            fly.vel *= max_speed / speed;
        }
        fly.pos += fly.vel * dt;
        if fly.pos.y < 0.15 {
            fly.pos.y = 0.15;
            fly.vel.y = fly.vel.y.abs() * 0.4;
        }

        // Did the firefly just walk through the door rectangle?
        // The world tag only changes when the firefly is catching
        // UP to the player (different world). Once it matches the
        // player's world, further door crossings don't flip it —
        // otherwise wandering past the door, or chasing a player
        // who walked around to the door's other side, would cause
        // a ping-pong between worlds.
        if fly.world != player_world {
            let prev_d = (fly.prev_pos - PORTAL_POS).dot(PORTAL_NORMAL);
            let curr_d = (fly.pos - PORTAL_POS).dot(PORTAL_NORMAL);
            if prev_d.signum() != curr_d.signum() {
                let local = fly.pos - PORTAL_POS;
                let in_x = local.x.abs() < PORTAL_HALF_W + 0.20;
                let in_y = local.y > -PORTAL_HALF_H - 0.20 && local.y < PORTAL_HALF_H + 0.50;
                if in_x && in_y {
                    fly.world = player_world;
                }
            }
        }
    }
}

fn fireflies_for_world(flies: &[Firefly; FIREFLY_COUNT], world: u8) -> Vec<Vec3> {
    flies.iter().filter(|f| f.world == world).map(|f| f.pos).collect()
}

fn build_fireflies(positions: &[Vec3]) -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    let glow = Vec3::new(3.5, 5.0, 2.5);
    for &pos in positions {
        push_cube(&mut v, &mut i, pos, Vec3::new(0.07, 0.07, 0.07), glow, 1.0);
    }
    (v, i)
}

// The "magic surface" of the portal — a single quad sitting inside
// the door frame, oriented facing +z. The portal pipeline draws
// this quad and samples portal_color_tex with screen-space pixel
// coordinates, so what appears inside the door is the other world.
fn build_portal_quad() -> (Vec<Vertex>, Vec<u32>) {
    let cx = PORTAL_POS.x;
    let cy = PORTAL_POS.y;
    let cz = PORTAL_POS.z;
    let hw = PORTAL_HALF_W;
    let hh = PORTAL_HALF_H;
    // The surface sits slightly forward of cz so it's flush with the
    // frame's front face. Normals don't matter (portal shader ignores
    // them) so we leave them zero.
    let z = cz + 0.001;
    let v = vec![
        Vertex { pos: [cx - hw, cy - hh, z], normal: [0.0, 0.0, 1.0], color: [1.0; 3], emissive: 0.0 },
        Vertex { pos: [cx + hw, cy - hh, z], normal: [0.0, 0.0, 1.0], color: [1.0; 3], emissive: 0.0 },
        Vertex { pos: [cx + hw, cy + hh, z], normal: [0.0, 0.0, 1.0], color: [1.0; 3], emissive: 0.0 },
        Vertex { pos: [cx - hw, cy + hh, z], normal: [0.0, 0.0, 1.0], color: [1.0; 3], emissive: 0.0 },
    ];
    let i = vec![0, 1, 2, 0, 2, 3];
    (v, i)
}

// ─────────────────────────────────────────────────────────────────
// SECTION 4 — Character (per-frame rebuild)
// ─────────────────────────────────────────────────────────────────

fn build_character(pos: Vec3, yaw: f32, walk_t: f32, attack_t: f32, moving: bool) -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    const FOOT_OFFSET: f32 = 0.36;
    let root = Mat4::from_translation(pos - Vec3::new(0.0, FOOT_OFFSET, 0.0))
        * Mat4::from_rotation_y(yaw);
    let walk_freq = 9.0;
    let walk_amp = if moving { 0.55 } else { 0.0 };
    let leg_angle = (walk_t * walk_freq).sin() * walk_amp;
    let arm_swing = (walk_t * walk_freq).sin() * 0.40 * if moving { 1.0 } else { 0.0 };
    let attack_dur = 0.30;
    let attack_angle = if attack_t > 0.0 {
        let p = (1.0 - attack_t / attack_dur).clamp(0.0, 1.0);
        -(p * std::f32::consts::PI).sin() * 1.6
    } else { 0.0 };

    let shirt = Vec3::new(0.30, 0.40, 0.55);
    let pants = Vec3::new(0.22, 0.18, 0.14);
    let skin = Vec3::new(0.85, 0.65, 0.50);
    let hair = Vec3::new(0.18, 0.12, 0.08);
    let blade = Vec3::new(0.75, 0.78, 0.85);
    let hilt = Vec3::new(0.40, 0.28, 0.16);

    let body = root * Mat4::from_translation(Vec3::new(0.0, 1.10, 0.0));
    push_box(&mut v, &mut i, body, Vec3::new(0.16, 0.25, 0.10), shirt, 0.0);
    let belt = root * Mat4::from_translation(Vec3::new(0.0, 0.86, 0.0));
    push_box(&mut v, &mut i, belt, Vec3::new(0.17, 0.04, 0.11), Vec3::new(0.18, 0.12, 0.08), 0.0);

    let head = root * Mat4::from_translation(Vec3::new(0.0, 1.55, 0.0));
    push_box(&mut v, &mut i, head, Vec3::new(0.12, 0.12, 0.12), skin, 0.0);
    let cap = root * Mat4::from_translation(Vec3::new(0.0, 1.69, 0.0));
    push_box(&mut v, &mut i, cap, Vec3::new(0.13, 0.04, 0.13), hair, 0.0);
    let eye_l = root * Mat4::from_translation(Vec3::new(-0.05, 1.56, -0.115));
    push_box(&mut v, &mut i, eye_l, Vec3::new(0.015, 0.015, 0.005), Vec3::new(0.04, 0.04, 0.04), 0.0);
    let eye_r = root * Mat4::from_translation(Vec3::new(0.05, 1.56, -0.115));
    push_box(&mut v, &mut i, eye_r, Vec3::new(0.015, 0.015, 0.005), Vec3::new(0.04, 0.04, 0.04), 0.0);

    let arm_half = 0.18;
    let left_shoulder = root
        * Mat4::from_translation(Vec3::new(-0.22, 1.32, 0.0))
        * Mat4::from_rotation_x(-arm_swing);
    let left_arm = left_shoulder * Mat4::from_translation(Vec3::new(0.0, -arm_half, 0.0));
    push_box(&mut v, &mut i, left_arm, Vec3::new(0.045, arm_half, 0.045), shirt, 0.0);
    let left_hand = left_shoulder * Mat4::from_translation(Vec3::new(0.0, -2.0 * arm_half - 0.02, 0.0));
    push_box(&mut v, &mut i, left_hand, Vec3::new(0.05, 0.04, 0.05), skin, 0.0);

    let right_shoulder = root
        * Mat4::from_translation(Vec3::new(0.22, 1.32, 0.0))
        * Mat4::from_rotation_x(arm_swing + attack_angle);
    let right_arm = right_shoulder * Mat4::from_translation(Vec3::new(0.0, -arm_half, 0.0));
    push_box(&mut v, &mut i, right_arm, Vec3::new(0.045, arm_half, 0.045), shirt, 0.0);
    let right_hand = right_shoulder * Mat4::from_translation(Vec3::new(0.0, -2.0 * arm_half - 0.02, 0.0));
    push_box(&mut v, &mut i, right_hand, Vec3::new(0.05, 0.04, 0.05), skin, 0.0);

    let sword_root = right_shoulder
        * Mat4::from_translation(Vec3::new(0.0, -2.0 * arm_half - 0.02, 0.0))
        * Mat4::from_rotation_x(-1.5708);
    let hilt_xform = sword_root * Mat4::from_translation(Vec3::new(0.0, 0.05, 0.0));
    push_box(&mut v, &mut i, hilt_xform, Vec3::new(0.03, 0.05, 0.03), hilt, 0.0);
    let guard_xform = sword_root * Mat4::from_translation(Vec3::new(0.0, 0.10, 0.0));
    push_box(&mut v, &mut i, guard_xform, Vec3::new(0.08, 0.015, 0.025), hilt, 0.0);
    let blade_xform = sword_root * Mat4::from_translation(Vec3::new(0.0, 0.35, 0.0));
    push_box(&mut v, &mut i, blade_xform, Vec3::new(0.022, 0.22, 0.018), blade, 0.0);

    let leg_half = 0.20;
    let left_hip = root
        * Mat4::from_translation(Vec3::new(-0.09, 0.82, 0.0))
        * Mat4::from_rotation_x(leg_angle);
    let left_leg = left_hip * Mat4::from_translation(Vec3::new(0.0, -leg_half, 0.0));
    push_box(&mut v, &mut i, left_leg, Vec3::new(0.055, leg_half, 0.055), pants, 0.0);
    let left_boot = left_hip * Mat4::from_translation(Vec3::new(0.0, -2.0 * leg_half - 0.02, 0.0));
    push_box(&mut v, &mut i, left_boot, Vec3::new(0.065, 0.04, 0.075), Vec3::new(0.10, 0.07, 0.05), 0.0);

    let right_hip = root
        * Mat4::from_translation(Vec3::new(0.09, 0.82, 0.0))
        * Mat4::from_rotation_x(-leg_angle);
    let right_leg = right_hip * Mat4::from_translation(Vec3::new(0.0, -leg_half, 0.0));
    push_box(&mut v, &mut i, right_leg, Vec3::new(0.055, leg_half, 0.055), pants, 0.0);
    let right_boot = right_hip * Mat4::from_translation(Vec3::new(0.0, -2.0 * leg_half - 0.02, 0.0));
    push_box(&mut v, &mut i, right_boot, Vec3::new(0.065, 0.04, 0.075), Vec3::new(0.10, 0.07, 0.05), 0.0);

    (v, i)
}

// ─────────────────────────────────────────────────────────────────
// SECTION 5 — Input + Player
// ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Input {
    fwd: bool, back: bool, strafe_l: bool, strafe_r: bool,
    look_up: bool, look_down: bool, look_left: bool, look_right: bool,
    jump_pressed: bool,
    attack_pressed: bool,
    mouse_dx: f32, mouse_dy: f32,
}

struct Player {
    pos: Vec3,
    prev_pos: Vec3,
    yaw: f32,    // camera yaw
    pitch: f32,
    body_yaw: f32, // character body — smoothly rotates toward movement
    vel_y: f32,
    on_ground: bool,
    walk_t: f32,
    attack_t: f32,
}

// Animation state machine — selects which named clip plays and
// cross-fades between consecutive states over a short transition.
// Clips beyond "Walking" are optional: if the loaded asset doesn't
// supply them (CesiumMan only has walking), the corresponding
// state falls back to bind pose during the fade.
#[derive(Copy, Clone, PartialEq, Eq)]
enum AnimState { Idle, Walk, Jump, Attack }

fn resolve_state_clip<'a>(state: AnimState, character: &'a Character) -> Option<&'a str> {
    match state {
        AnimState::Idle => character.find_clip(&["Idle", "Standing", "Stand"]),
        AnimState::Walk => character.walk_clip_name(),
        AnimState::Jump => character.find_clip(&["Jump", "Jumping"]),
        AnimState::Attack => character.find_clip(&["Punch", "Attack", "Slash", "SwordSlash"]),
    }
}

struct AnimController {
    state: AnimState,
    state_time: f32,
    previous: Option<AnimState>,
    previous_time: f32,
    transition: f32, // 0..1 — 1 = transition complete
    transition_duration: f32,
}

impl AnimController {
    fn new() -> Self {
        Self {
            state: AnimState::Idle,
            state_time: 0.0,
            previous: None,
            previous_time: 0.0,
            transition: 1.0,
            transition_duration: 0.18,
        }
    }

    fn set(&mut self, new_state: AnimState) {
        if new_state == self.state {
            return;
        }
        self.previous = Some(self.state);
        self.previous_time = self.state_time;
        self.state = new_state;
        self.state_time = 0.0;
        self.transition = 0.0;
    }

    fn tick(&mut self, dt: f32, state_rate: f32) {
        self.state_time += state_rate * dt;
        if self.transition < 1.0 {
            self.transition = (self.transition + dt / self.transition_duration).min(1.0);
            if self.transition >= 1.0 {
                self.previous = None;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// SECTION 6 — Shaders
// ─────────────────────────────────────────────────────────────────

const MAIN_SHADER: &str = r#"
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
"#;

const SKIN_SHADER: &str = r#"
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

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) emissive: f32,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>,
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

    let lit = in.color * (u.ambient_color.xyz + lamp_diffuse + moon_diffuse + fly_total);
    return vec4<f32>(lit / (lit + vec3<f32>(1.0)), 1.0);
}
"#;

const SHADOW_SHADER: &str = r#"
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

@vertex
fn vs(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u.light_view_proj * vec4<f32>(pos, 1.0);
}
"#;

// Portal shader — draws the magic surface inside the door frame.
// Vertex stage transforms by the real camera's view_proj so the
// quad sits where the door is in the current world. Fragment stage
// reads portal_tex at the SCREEN PIXEL the fragment occupies, so
// what shows up in the door is whatever was rendered to portal_tex
// from the corresponding virtual camera — i.e., the other world.
const SKIN_SHADOW_SHADER: &str = r#"
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
@group(1) @binding(0) var<storage, read> bones: array<mat4x4<f32>>;

@vertex
fn vs(
    @location(0) pos: vec3<f32>,
    @location(1) _normal: vec3<f32>,
    @location(2) _color: vec3<f32>,
    @location(3) _emissive: f32,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let skin = bones[joints.x] * weights.x
             + bones[joints.y] * weights.y
             + bones[joints.z] * weights.z
             + bones[joints.w] * weights.w;
    let world_pos = (skin * vec4<f32>(pos, 1.0)).xyz;
    return u.light_view_proj * vec4<f32>(world_pos, 1.0);
}
"#;

const PORTAL_SHADER: &str = r#"
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
@group(1) @binding(0) var portal_tex: texture_2d<f32>;
@group(1) @binding(1) var portal_samp: sampler;

@vertex
fn vs(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u.view_proj * vec4<f32>(pos, 1.0);
}

@fragment
fn fs(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(i32(frag_pos.x), i32(frag_pos.y));
    return textureLoad(portal_tex, texel, 0);
}
"#;

// ─────────────────────────────────────────────────────────────────
// SECTION 7 — Game state + render
// ─────────────────────────────────────────────────────────────────

const PLAYER_VBUF_CAPACITY: u64 = 4096 * std::mem::size_of::<Vertex>() as u64;
const PLAYER_IBUF_CAPACITY: u64 = 4096 * std::mem::size_of::<u32>() as u64;
const FIREFLY_VBUF_CAPACITY: u64 = 256 * std::mem::size_of::<Vertex>() as u64;
const FIREFLY_IBUF_CAPACITY: u64 = 256 * std::mem::size_of::<u32>() as u64;
const SHADOW_MAP_SIZE: u32 = 1024;

struct WorldGpu {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    index_count: u32,
    env: WorldEnv,
}

struct Game {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    depth_view: wgpu::TextureView,
    shadow_view: wgpu::TextureView,

    // Offscreen target where the OTHER world is rendered each frame.
    portal_color_view: wgpu::TextureView,
    portal_depth_view: wgpu::TextureView,
    portal_tex_bg: wgpu::BindGroup,
    portal_sampler: wgpu::Sampler,
    bgl1_portal: wgpu::BindGroupLayout,

    main_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    portal_pipeline: wgpu::RenderPipeline,
    skin_pipeline: wgpu::RenderPipeline,
    skin_shadow_pipeline: wgpu::RenderPipeline,
    character: Character,
    anim_ctrl: AnimController,
    jump_blend: f32,
    main_bg0: wgpu::BindGroup,
    main_bg1_shadow: wgpu::BindGroup,
    shadow_bg0: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,

    world_a: WorldGpu,
    world_b: WorldGpu,
    portal_vbuf: wgpu::Buffer,
    portal_ibuf: wgpu::Buffer,
    portal_index_count: u32,
    door_vbuf: wgpu::Buffer,
    door_ibuf: wgpu::Buffer,
    door_index_count: u32,

    player_vbuf: wgpu::Buffer,
    player_ibuf: wgpu::Buffer,
    // Two firefly mesh buffers — one per world. Each frame the
    // fireflies are sorted by their `.world` tag and the matching
    // group is written here. The main pass picks the buffer that
    // matches cur_world; the portal pass picks the other.
    firefly_vbufs: [wgpu::Buffer; 2],
    firefly_ibufs: [wgpu::Buffer; 2],
    firefly_counts: [u32; 2],

    current_world: u8, // 0 = A, 1 = B
    player: Player,
    input: Input,
    third_person: bool,
    fireflies: [Firefly; FIREFLY_COUNT],
    time: f32,
    last_frame: std::time::Instant,
}

impl Game {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // ── Build both worlds ──
        let (va, ia, env_a) = build_world_a();
        let (vb, ib, env_b) = build_world_b();
        let vbuf_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world-a-v"),
            contents: bytemuck::cast_slice(&va),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world-a-i"),
            contents: bytemuck::cast_slice(&ia),
            usage: wgpu::BufferUsages::INDEX,
        });
        let vbuf_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world-b-v"),
            contents: bytemuck::cast_slice(&vb),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world-b-i"),
            contents: bytemuck::cast_slice(&ib),
            usage: wgpu::BufferUsages::INDEX,
        });

        let (pv, pi) = build_portal_quad();
        let portal_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-v"),
            contents: bytemuck::cast_slice(&pv),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let portal_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-i"),
            contents: bytemuck::cast_slice(&pi),
            usage: wgpu::BufferUsages::INDEX,
        });

        let (dv_, di) = build_door_mesh();
        let door_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("door-v"),
            contents: bytemuck::cast_slice(&dv_),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let door_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("door-i"),
            contents: bytemuck::cast_slice(&di),
            usage: wgpu::BufferUsages::INDEX,
        });
        let door_index_count = di.len() as u32;

        let player_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("player-v"),
            size: PLAYER_VBUF_CAPACITY,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let player_ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("player-i"),
            size: PLAYER_IBUF_CAPACITY,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mk_fly_vbuf = |label: &str| device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: FIREFLY_VBUF_CAPACITY,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mk_fly_ibuf = |label: &str| device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: FIREFLY_IBUF_CAPACITY,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let firefly_vbufs = [mk_fly_vbuf("firefly-v-a"), mk_fly_vbuf("firefly-v-b")];
        let firefly_ibufs = [mk_fly_ibuf("firefly-i-a"), mk_fly_ibuf("firefly-i-b")];

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── Textures ──
        let depth_view = make_depth(&device, config.width, config.height, "depth");
        let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow-tex"),
            size: wgpu::Extent3d { width: SHADOW_MAP_SIZE, height: SHADOW_MAP_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow-samp"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let (portal_color_view, portal_depth_view) =
            make_portal_targets(&device, config.width, config.height, format);
        let portal_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("portal-samp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // ── Bind group layouts ──
        let bgl0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl0"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bgl1_shadow = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl1-shadow"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let bgl1_portal = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl1-portal"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let main_bg0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("main-bg0"), layout: &bgl0,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() }],
        });
        let main_bg1_shadow = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("main-bg1-shadow"), layout: &bgl1_shadow,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&shadow_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&shadow_sampler) },
            ],
        });
        let shadow_bg0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow-bg0"), layout: &bgl0,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() }],
        });
        let portal_tex_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("portal-tex-bg"), layout: &bgl1_portal,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&portal_color_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&portal_sampler) },
            ],
        });

        // ── Shaders + pipelines ──
        let main_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("main-shader"),
            source: wgpu::ShaderSource::Wgsl(MAIN_SHADER.into()),
        });
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()),
        });
        let portal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("portal-shader"),
            source: wgpu::ShaderSource::Wgsl(PORTAL_SHADER.into()),
        });

        let main_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main-pl"),
            bind_group_layouts: &[&bgl0, &bgl1_shadow],
            push_constant_ranges: &[],
        });
        let shadow_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow-pl"),
            bind_group_layouts: &[&bgl0],
            push_constant_ranges: &[],
        });
        let portal_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("portal-pl"),
            bind_group_layouts: &[&bgl0, &bgl1_portal],
            push_constant_ranges: &[],
        });

        let vattrs = [
            wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
            wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
            wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x3 },
            wgpu::VertexAttribute { offset: 36, shader_location: 3, format: wgpu::VertexFormat::Float32 },
        ];

        let main_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("main-pipeline"),
            layout: Some(&main_pl),
            vertex: wgpu::VertexState {
                module: &main_shader, entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &vattrs,
                }],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &main_shader, entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None, cache: None,
        });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow-pipeline"),
            layout: Some(&shadow_pl),
            vertex: wgpu::VertexState {
                module: &shadow_shader, entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &vattrs,
                }],
            },
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: Default::default(),
            fragment: None,
            multiview: None, cache: None,
        });

        let portal_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("portal-pipeline"),
            layout: Some(&portal_pl),
            vertex: wgpu::VertexState {
                module: &portal_shader, entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &vattrs,
                }],
            },
            primitive: wgpu::PrimitiveState {
                // No culling on the portal quad — visible from both sides.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &portal_shader, entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None, cache: None,
        });

        // ── Skinned-character pipeline ──
        let bone_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bone-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let skin_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("skin-shader"),
            source: wgpu::ShaderSource::Wgsl(SKIN_SHADER.into()),
        });
        let skin_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("skin-pl"),
            bind_group_layouts: &[&bgl0, &bgl1_shadow, &bone_bgl],
            push_constant_ranges: &[],
        });
        let skin_vattrs = [
            wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
            wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
            wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x3 },
            wgpu::VertexAttribute { offset: 36, shader_location: 3, format: wgpu::VertexFormat::Float32 },
            wgpu::VertexAttribute { offset: 40, shader_location: 4, format: wgpu::VertexFormat::Uint32x4 },
            wgpu::VertexAttribute { offset: 56, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
        ];
        let skin_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("skin-pipeline"),
            layout: Some(&skin_pl),
            vertex: wgpu::VertexState {
                module: &skin_shader, entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SkinVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &skin_vattrs,
                }],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &skin_shader, entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None, cache: None,
        });
        let character = Character::load(
            &device,
            concat!(env!("CARGO_MANIFEST_DIR"), "/assets/character.glb"),
            &bone_bgl,
        );

        // Shadow pipeline for the skinned character — same depth
        // target as the regular shadow pass but reads SkinVertex
        // and applies skinning so the cast shadow matches the
        // animated pose.
        let skin_shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("skin-shadow-shader"),
            source: wgpu::ShaderSource::Wgsl(SKIN_SHADOW_SHADER.into()),
        });
        let skin_shadow_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("skin-shadow-pl"),
            bind_group_layouts: &[&bgl0, &bone_bgl],
            push_constant_ranges: &[],
        });
        let skin_shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("skin-shadow-pipeline"),
            layout: Some(&skin_shadow_pl),
            vertex: wgpu::VertexState {
                module: &skin_shadow_shader, entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SkinVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &skin_vattrs,
                }],
            },
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: Default::default(),
            fragment: None,
            multiview: None, cache: None,
        });

        let _ = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
        window.set_cursor_visible(false);

        let spawn = Vec3::new(4.5, 0.0, 1.5);
        Self {
            window, surface, device, queue, config,
            depth_view, shadow_view,
            portal_color_view, portal_depth_view, portal_tex_bg,
            portal_sampler,
            bgl1_portal,
            main_pipeline, shadow_pipeline, portal_pipeline,
            skin_pipeline, skin_shadow_pipeline, character,
            anim_ctrl: AnimController::new(),
            jump_blend: 0.0,
            main_bg0, main_bg1_shadow, shadow_bg0,
            uniform_buf,
            world_a: WorldGpu { vbuf: vbuf_a, ibuf: ibuf_a, index_count: ia.len() as u32, env: env_a },
            world_b: WorldGpu { vbuf: vbuf_b, ibuf: ibuf_b, index_count: ib.len() as u32, env: env_b },
            portal_vbuf, portal_ibuf, portal_index_count: pi.len() as u32,
            door_vbuf, door_ibuf, door_index_count,
            player_vbuf, player_ibuf,
            firefly_vbufs, firefly_ibufs, firefly_counts: [0, 0],
            current_world: 0,
            player: Player {
                pos: spawn, prev_pos: spawn,
                // Face the door at (2.5, 1.0, -3.0) from spawn (4.5, 0, 1.5):
                // forward = (-sin yaw, 0, -cos yaw) ≈ (-0.408, 0, -0.913)
                yaw: 0.42,
                pitch: 0.0,
                body_yaw: 0.42,
                vel_y: 0.0,
                on_ground: true,
                walk_t: 0.0,
                attack_t: 0.0,
            },
            input: Input::default(),
            third_person: true,
            fireflies: {
                let make = |off: Vec3, phase: [f32; 3]| {
                    let p = spawn + off;
                    Firefly { pos: p, prev_pos: p, vel: Vec3::ZERO, world: 0, phase }
                };
                [
                    make(Vec3::new(0.5, 1.0, 0.0), [0.0, 1.7, 3.4]),
                    make(Vec3::new(-0.5, 1.2, 0.3), [2.1, 0.5, 4.0]),
                    make(Vec3::new(0.2, 0.7, -0.5), [1.3, 3.2, 0.8]),
                    make(Vec3::new(0.3, 1.4, 0.4), [3.7, 2.4, 1.5]),
                ]
            },
            time: 0.0,
            last_frame: std::time::Instant::now(),
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
        self.depth_view = make_depth(&self.device, self.config.width, self.config.height, "depth");
        let (pcv, pdv) = make_portal_targets(&self.device, self.config.width, self.config.height, self.config.format);
        self.portal_color_view = pcv;
        self.portal_depth_view = pdv;
        // Recreate portal bind group since the texture view changed.
        self.portal_tex_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("portal-tex-bg"),
            layout: &self.bgl1_portal,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.portal_color_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.portal_sampler) },
            ],
        });
    }

    fn key(&mut self, code: KeyCode, pressed: bool) {
        match code {
            KeyCode::KeyW => self.input.fwd = pressed,
            KeyCode::KeyS => self.input.back = pressed,
            KeyCode::KeyA => self.input.strafe_l = pressed,
            KeyCode::KeyD => self.input.strafe_r = pressed,
            KeyCode::ArrowUp => self.input.look_up = pressed,
            KeyCode::ArrowDown => self.input.look_down = pressed,
            KeyCode::ArrowLeft => self.input.look_left = pressed,
            KeyCode::ArrowRight => self.input.look_right = pressed,
            KeyCode::Space => { if pressed { self.input.jump_pressed = true; } }
            KeyCode::KeyF => { if pressed { self.input.attack_pressed = true; } }
            KeyCode::KeyV => { if pressed { self.third_person = !self.third_person; } }
            _ => {}
        }
    }

    fn update(&mut self, dt: f32) {
        // Look — arrows = rate-based virtual mouse, real mouse = delta.
        let look_speed = 1.8;
        if self.input.look_left { self.player.yaw += look_speed * dt; }
        if self.input.look_right { self.player.yaw -= look_speed * dt; }
        if self.input.look_up { self.player.pitch += look_speed * dt; }
        if self.input.look_down { self.player.pitch -= look_speed * dt; }
        let sens = 0.0025;
        self.player.yaw -= self.input.mouse_dx * sens;
        self.player.pitch -= self.input.mouse_dy * sens;
        self.input.mouse_dx = 0.0;
        self.input.mouse_dy = 0.0;
        self.player.pitch = self.player.pitch.clamp(-1.2, 1.2);

        let (sy, cy) = self.player.yaw.sin_cos();
        let forward = Vec3::new(-sy, 0.0, -cy);
        let right = Vec3::new(cy, 0.0, -sy);
        let mut m = Vec3::ZERO;
        if self.input.fwd { m += forward; }
        if self.input.back { m -= forward; }
        if self.input.strafe_r { m += right; }
        if self.input.strafe_l { m -= right; }
        let moving = m.length_squared() > 0.0;

        self.player.prev_pos = self.player.pos;
        if moving {
            let m_norm = m.normalize();
            self.player.pos += m_norm * 3.0 * dt;
            self.player.walk_t += dt;
            // Target body yaw from movement direction (world space).
            // forward(yaw) = (-sin yaw, 0, -cos yaw), so the yaw that
            // makes forward equal m_norm is atan2(-m.x, -m.z).
            let target_body_yaw = (-m_norm.x).atan2(-m_norm.z);
            let tau = std::f32::consts::TAU;
            let diff = (target_body_yaw - self.player.body_yaw + std::f32::consts::PI)
                .rem_euclid(tau) - std::f32::consts::PI;
            let alpha = (10.0 * dt).min(1.0);
            self.player.body_yaw += diff * alpha;
        }

        if self.input.jump_pressed && self.player.on_ground {
            self.player.vel_y = 4.5;
            self.player.on_ground = false;
        }
        self.input.jump_pressed = false;
        self.player.vel_y -= 12.0 * dt;
        self.player.pos.y += self.player.vel_y * dt;
        if self.player.pos.y <= 0.0 {
            self.player.pos.y = 0.0;
            self.player.vel_y = 0.0;
            self.player.on_ground = true;
        }

        if self.input.attack_pressed && self.player.attack_t <= 0.0 {
            self.player.attack_t = 0.30;
        }
        self.input.attack_pressed = false;
        if self.player.attack_t > 0.0 {
            self.player.attack_t = (self.player.attack_t - dt).max(0.0);
        }

        // ── Teleport + collision check ──
        // The door is a two-way passage with the "open" side flipped
        // per world: in A the +z side is open (the player approaches
        // from there); in B the -z side is open (where they emerge
        // after teleport, and where they walk back from). The other
        // side of each world is locked — attempting to cross it
        // bumps the player back instead of letting them through.
        let prev_d = (self.player.prev_pos - PORTAL_POS).dot(PORTAL_NORMAL);
        let curr_d = (self.player.pos - PORTAL_POS).dot(PORTAL_NORMAL);
        let local = self.player.pos - PORTAL_POS;
        let inside_x = local.x.abs() < PORTAL_HALF_W + 0.25;
        let inside_y = local.y > -PORTAL_HALF_H - 0.20 && local.y < PORTAL_HALF_H + 1.20;
        let crossed = prev_d.signum() != curr_d.signum();
        if crossed && inside_x && inside_y {
            let teleport_allowed = if self.current_world == 0 {
                prev_d > 0.0 && curr_d < 0.0 // A: +z → -z
            } else {
                prev_d < 0.0 && curr_d > 0.0 // B: -z → +z
            };
            if teleport_allowed {
                self.current_world ^= 1;
            }
        }

        // ── Post collision ──
        // The door frame's two vertical posts are solid. If the
        // player's xz overlaps either post's AABB (expanded by the
        // player's body radius), revert the offending move so they
        // stop against the frame instead of phasing through it. The
        // doorway opening between the posts remains free to walk.
        let z_min = PORTAL_POS.z - PORTAL_DEPTH_T;
        let z_max = PORTAL_POS.z + PORTAL_DEPTH_T;
        let left_min_x = PORTAL_POS.x - PORTAL_HALF_W - 2.0 * PORTAL_POST_T;
        let left_max_x = PORTAL_POS.x - PORTAL_HALF_W;
        let right_min_x = PORTAL_POS.x + PORTAL_HALF_W;
        let right_max_x = PORTAL_POS.x + PORTAL_HALF_W + 2.0 * PORTAL_POST_T;
        let hits_post = |p: Vec3, min_x: f32, max_x: f32| {
            p.x > min_x - PLAYER_RADIUS && p.x < max_x + PLAYER_RADIUS
                && p.z > z_min - PLAYER_RADIUS && p.z < z_max + PLAYER_RADIUS
        };
        if hits_post(self.player.pos, left_min_x, left_max_x)
            || hits_post(self.player.pos, right_min_x, right_max_x)
        {
            self.player.pos.x = self.player.prev_pos.x;
            self.player.pos.z = self.player.prev_pos.z;
        }
    }

    fn render(&mut self) {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.update(dt);

        let aspect = self.config.width as f32 / self.config.height as f32;
        let (sy, cy) = self.player.yaw.sin_cos();
        let sp = self.player.pitch.sin();
        let cp = self.player.pitch.cos();
        let forward = Vec3::new(-sy * cp, sp, -cy * cp);
        let eye_anchor = self.player.pos + Vec3::new(0.0, 1.05, 0.0);
        let cam = camera_for(self.player.pos, forward, self.third_person);
        let target = if self.third_person {
            eye_anchor
        } else {
            self.player.pos + Vec3::new(0.0, 1.20, 0.0) + forward
        };
        let view = Mat4::look_at_rh(cam, target, Vec3::Y);
        let proj = Mat4::perspective_rh(60f32.to_radians(), aspect, 0.05, 200.0);
        let view_proj = proj * view;

        // The portal's magic surface activates on whichever side is
        // "open" for the current world: +z in A (preview of B), -z
        // in B (preview of A, the way back). The door itself stays
        // visible from every angle in both worlds; only the magic
        // surface that reveals the other world is gated here.
        let cam_d = (cam - PORTAL_POS).dot(PORTAL_NORMAL);
        let portal_active = if self.current_world == 0 {
            cam_d > 0.0
        } else {
            cam_d < 0.0
        };
        let portal_src = if self.current_world == 0 {
            &self.world_b
        } else {
            &self.world_a
        };

        // Virtual camera for the OTHER world. Since both worlds share
        // local coords AND both portals are at the same position with
        // the same orientation, the transform A→B (or B→A) is the
        // identity, so the virtual camera equals the real one.
        let virt_view = view;
        let virt_view_proj = proj * virt_view;

        let cur = if self.current_world == 0 { &self.world_a } else { &self.world_b };

        // Player movement check (for walking animation flag).
        let m_input = self.input.fwd as i32 + self.input.back as i32
            + self.input.strafe_l as i32 + self.input.strafe_r as i32;
        let moving = m_input > 0 && self.player.on_ground;

        // Rebuild + upload character mesh.
        let (cv, ci) = build_character(
            self.player.pos, self.player.yaw,
            self.player.walk_t, self.player.attack_t, moving,
        );
        let _player_index_count = ci.len() as u32;
        self.queue.write_buffer(&self.player_vbuf, 0, bytemuck::cast_slice(&cv));
        self.queue.write_buffer(&self.player_ibuf, 0, bytemuck::cast_slice(&ci));

        // Animation state machine — pick a target state from the
        // player's situation, hand it to the controller (which
        // handles the cross-fade), then assemble weighted clip
        // samples and let the skin shader pose the body.
        let target_state = if self.player.attack_t > 0.0 {
            AnimState::Attack
        } else if !self.player.on_ground {
            AnimState::Jump
        } else if moving {
            AnimState::Walk
        } else {
            AnimState::Idle
        };
        self.anim_ctrl.set(target_state);
        // Walk advances proportional to ground distance; everything
        // else plays at wall-clock rate.
        const NOMINAL_WALK_SPEED: f32 = 1.4;
        const PLAYER_SPEED: f32 = 3.0;
        let state_rate = match self.anim_ctrl.state {
            AnimState::Walk => PLAYER_SPEED / NOMINAL_WALK_SPEED,
            _ => 1.0,
        };
        self.anim_ctrl.tick(dt, state_rate);

        let mut samples: Vec<(&str, f32, f32)> = Vec::new();
        if let Some(prev) = self.anim_ctrl.previous {
            if let Some(name) = resolve_state_clip(prev, &self.character) {
                samples.push((name, self.anim_ctrl.previous_time, 1.0 - self.anim_ctrl.transition));
            }
        }
        if let Some(name) = resolve_state_clip(self.anim_ctrl.state, &self.character) {
            samples.push((name, self.anim_ctrl.state_time, self.anim_ctrl.transition));
        }
        // Procedural jump pose — when no Jump clip is available
        // we tilt the character forward and squash it slightly on
        // Y to suggest a tucked leap. jump_blend rises while in
        // the air and falls back smoothly on landing.
        let target_jump = if !self.player.on_ground { 1.0_f32 } else { 0.0 };
        let jb_alpha = (10.0 * dt).min(1.0);
        self.jump_blend += (target_jump - self.jump_blend) * jb_alpha;
        let tilt = -0.45 * self.jump_blend;
        let squash = 1.0 - 0.10 * self.jump_blend;
        let extra_pre = Mat4::from_rotation_x(tilt)
            * Mat4::from_scale(Vec3::new(1.0, squash, 1.0));
        self.character.update(
            &self.queue,
            self.player.pos,
            self.player.body_yaw,
            &samples,
            extra_pre,
        );

        // Fireflies — step their state forward, then rebuild a
        // separate mesh per world (so a firefly only appears in the
        // world it's currently tagged with).
        self.time += dt;
        step_fireflies(
            &mut self.fireflies,
            self.player.pos,
            self.current_world,
            dt,
            self.time,
        );
        for w in 0..2u8 {
            let positions = fireflies_for_world(&self.fireflies, w);
            let (fv, fi) = build_fireflies(&positions);
            self.firefly_counts[w as usize] = fi.len() as u32;
            if !fv.is_empty() {
                self.queue.write_buffer(
                    &self.firefly_vbufs[w as usize], 0, bytemuck::cast_slice(&fv));
                self.queue.write_buffer(
                    &self.firefly_ibufs[w as usize], 0, bytemuck::cast_slice(&fi));
            }
        }
        let cur_world = self.current_world;
        let portal_src_world: u8 = 1 - cur_world;

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };
        let frame_view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = self.device.create_command_encoder(&Default::default());

        // ── PASS A & B: only when the portal is active ──
        // From the back of the door (or from world B) there's no
        // portal effect at all, so we skip both the offscreen world
        // render and the shadow pass that feeds it.
        if portal_active {
            let portal_lvp = compute_light_view_proj(&portal_src.env, self.player.pos);
            let u_shadow_portal = build_uniforms(portal_lvp, &portal_src.env, &cam, &self.player.pos, 1.0, &self.fireflies, portal_src_world);
            self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u_shadow_portal));
            {
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("shadow-portal"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.shadow_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
                pass.set_pipeline(&self.shadow_pipeline);
                pass.set_bind_group(0, &self.shadow_bg0, &[]);
                pass.set_vertex_buffer(0, portal_src.vbuf.slice(..));
                pass.set_index_buffer(portal_src.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..portal_src.index_count, 0, 0..1);
                // Skinned character — depth-only skin shader.
                pass.set_pipeline(&self.skin_shadow_pipeline);
                pass.set_bind_group(0, &self.shadow_bg0, &[]);
                pass.set_bind_group(1, &self.character.bone_bind_group, &[]);
                pass.set_vertex_buffer(0, self.character.vbuf.slice(..));
                pass.set_index_buffer(self.character.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.character.index_count, 0, 0..1);
            }

            let u_portal = build_uniforms(virt_view_proj, &portal_src.env, &cam, &self.player.pos, 1.0, &self.fireflies, portal_src_world);
            self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u_portal));
            {
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("portal-view"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.portal_color_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(portal_src.env.sky),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.portal_depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
                pass.set_pipeline(&self.main_pipeline);
                pass.set_bind_group(0, &self.main_bg0, &[]);
                pass.set_bind_group(1, &self.main_bg1_shadow, &[]);
                pass.set_vertex_buffer(0, portal_src.vbuf.slice(..));
                pass.set_index_buffer(portal_src.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..portal_src.index_count, 0, 0..1);
                if self.third_person {
                    pass.set_pipeline(&self.skin_pipeline);
                    pass.set_bind_group(0, &self.main_bg0, &[]);
                    pass.set_bind_group(1, &self.main_bg1_shadow, &[]);
                    pass.set_bind_group(2, &self.character.bone_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.character.vbuf.slice(..));
                    pass.set_index_buffer(self.character.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..self.character.index_count, 0, 0..1);
                    pass.set_pipeline(&self.main_pipeline);
                    pass.set_bind_group(1, &self.main_bg1_shadow, &[]);
                }
                let fc = self.firefly_counts[portal_src_world as usize];
                if fc > 0 {
                    pass.set_vertex_buffer(0, self.firefly_vbufs[portal_src_world as usize].slice(..));
                    pass.set_index_buffer(self.firefly_ibufs[portal_src_world as usize].slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..fc, 0, 0..1);
                }
            }
        }
        let _ = portal_src; let _ = virt_view_proj;

        // ── PASS C: shadow map for CURRENT world ──
        let light_view_proj = compute_light_view_proj(&cur.env, self.player.pos);
        let u_shadow = build_uniforms(light_view_proj, &cur.env, &cam, &self.player.pos, 1.0, &self.fireflies, cur_world);
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u_shadow));
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow-current"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.shadow_bg0, &[]);
            pass.set_vertex_buffer(0, cur.vbuf.slice(..));
            pass.set_index_buffer(cur.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..cur.index_count, 0, 0..1);
            // Door is always present in either world — visible and
            // casting shadow even from the locked side; only the
            // portal's magic surface is gated by portal_active.
            pass.set_vertex_buffer(0, self.door_vbuf.slice(..));
            pass.set_index_buffer(self.door_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.door_index_count, 0, 0..1);
            // Skinned character — depth-only skin shader.
            pass.set_pipeline(&self.skin_shadow_pipeline);
            pass.set_bind_group(0, &self.shadow_bg0, &[]);
            pass.set_bind_group(1, &self.character.bone_bind_group, &[]);
            pass.set_vertex_buffer(0, self.character.vbuf.slice(..));
            pass.set_index_buffer(self.character.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.character.index_count, 0, 0..1);
        }

        // ── PASS 3: main render of CURRENT world + portal quad ──
        let u_main = build_uniforms(view_proj, &cur.env, &cam, &self.player.pos, 1.0, &self.fireflies, cur_world);
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u_main));
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(cur.env.sky),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            // Current world.
            pass.set_pipeline(&self.main_pipeline);
            pass.set_bind_group(0, &self.main_bg0, &[]);
            pass.set_bind_group(1, &self.main_bg1_shadow, &[]);
            pass.set_vertex_buffer(0, cur.vbuf.slice(..));
            pass.set_index_buffer(cur.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..cur.index_count, 0, 0..1);
            // The door is always drawn — visible in both worlds and
            // from both sides. Only the portal's magic surface is
            // gated below.
            pass.set_vertex_buffer(0, self.door_vbuf.slice(..));
            pass.set_index_buffer(self.door_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.door_index_count, 0, 0..1);
            // Skinned character — switches pipeline + binds bone matrices.
            if self.third_person {
                pass.set_pipeline(&self.skin_pipeline);
                pass.set_bind_group(0, &self.main_bg0, &[]);
                pass.set_bind_group(1, &self.main_bg1_shadow, &[]);
                pass.set_bind_group(2, &self.character.bone_bind_group, &[]);
                pass.set_vertex_buffer(0, self.character.vbuf.slice(..));
                pass.set_index_buffer(self.character.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.character.index_count, 0, 0..1);
                // Restore main pipeline for the fireflies and portal quad below.
                pass.set_pipeline(&self.main_pipeline);
                pass.set_bind_group(1, &self.main_bg1_shadow, &[]);
            }
            // Fireflies — only those currently tagged with cur_world
            // appear in the main pass. The others are still in the
            // other world and will only be seen through the portal
            // (or once they walk through the door themselves).
            let fc = self.firefly_counts[cur_world as usize];
            if fc > 0 {
                pass.set_vertex_buffer(0, self.firefly_vbufs[cur_world as usize].slice(..));
                pass.set_index_buffer(self.firefly_ibufs[cur_world as usize].slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..fc, 0, 0..1);
            }
            // Portal quad — only when the portal is active.
            if portal_active {
                pass.set_pipeline(&self.portal_pipeline);
                pass.set_bind_group(0, &self.main_bg0, &[]);
                pass.set_bind_group(1, &self.portal_tex_bg, &[]);
                pass.set_vertex_buffer(0, self.portal_vbuf.slice(..));
                pass.set_index_buffer(self.portal_ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.portal_index_count, 0, 0..1);
            }
        }

        self.queue.submit(Some(enc.finish()));
        frame.present();
        self.window.request_redraw();
    }
}

fn build_uniforms(
    view_proj: Mat4,
    env: &WorldEnv,
    cam: &Vec3,
    player_pos: &Vec3,
    shadow_strength: f32,
    flies: &[Firefly; FIREFLY_COUNT],
    target_world: u8,
) -> Uniforms {
    let light_view_proj = compute_light_view_proj(env, *player_pos);
    let mut fly_pos = [[0.0_f32; 4]; FIREFLY_COUNT];
    for (i, f) in flies.iter().enumerate() {
        // .w is an intensity mask: 1.0 if this firefly belongs to
        // the world being rendered, 0.0 if it lives in the other
        // world (so the shader doesn't add its glow).
        let w = if f.world == target_world { 1.0 } else { 0.0 };
        fly_pos[i] = [f.pos.x, f.pos.y, f.pos.z, w];
    }
    Uniforms {
        view_proj: view_proj.to_cols_array_2d(),
        light_view_proj: light_view_proj.to_cols_array_2d(),
        camera_pos: [cam.x, cam.y, cam.z, 1.0],
        lamp_pos: [env.lamp_pos.x, env.lamp_pos.y, env.lamp_pos.z, 1.0],
        lamp_color: env.lamp_color,
        moon_dir: [env.moon_dir.x, env.moon_dir.y, env.moon_dir.z, 0.0],
        moon_color: env.moon_color,
        ambient_color: [env.ambient[0], env.ambient[1], env.ambient[2], shadow_strength],
        fly_pos,
        fly_color: [3.5, 5.0, 2.5, 1.0],
    }
}

// The camera position used both for actual rendering and the
// teleport check. Pulled out so update() and render() stay in
// sync on what "where the camera is" means.
fn camera_for(player_pos: Vec3, forward: Vec3, third_person: bool) -> Vec3 {
    if third_person {
        player_pos + Vec3::new(0.0, 1.05, 0.0) - forward * 3.2 + Vec3::new(0.0, 0.35, 0.0)
    } else {
        player_pos + Vec3::new(0.0, 1.20, 0.0)
    }
}

fn compute_light_view_proj(env: &WorldEnv, player_pos: Vec3) -> Mat4 {
    let target = Vec3::new(player_pos.x, 0.5, player_pos.z);
    let to_light = (env.light_origin - target).normalize();
    let light_view = Mat4::look_at_rh(target + to_light * 30.0, target, Vec3::Y);
    let half = 16.0;
    let light_proj = Mat4::orthographic_rh(-half, half, -half, half, 1.0, 60.0);
    light_proj * light_view
}

fn make_depth(device: &wgpu::Device, w: u32, h: u32, label: &str) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

fn make_portal_targets(device: &wgpu::Device, w: u32, h: u32, format: wgpu::TextureFormat) -> (wgpu::TextureView, wgpu::TextureView) {
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("portal-color"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let cv = color.create_view(&wgpu::TextureViewDescriptor::default());
    let dv = make_depth(device, w, h, "portal-depth");
    (cv, dv)
}

// ─────────────────────────────────────────────────────────────────
// SECTION 8 — winit ApplicationHandler
// ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct App {
    game: Option<Game>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("trikala mood — portal · WASD/Arrows · Space · F · V · Esc")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.game = Some(pollster::block_on(Game::new(window)));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(g) = self.game.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(s) => g.resize(s.width, s.height),
            WindowEvent::RedrawRequested => g.render(),
            WindowEvent::KeyboardInput {
                event: KeyEvent { state, physical_key: PhysicalKey::Code(code), .. },
                ..
            } => {
                if code == KeyCode::Escape && state == ElementState::Pressed {
                    event_loop.exit();
                } else {
                    g.key(code, state == ElementState::Pressed);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left && state == ElementState::Pressed {
                    g.input.attack_pressed = true;
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let Some(g) = self.game.as_mut() else { return };
        if let DeviceEvent::MouseMotion { delta } = event {
            g.input.mouse_dx += delta.0 as f32;
            g.input.mouse_dy += delta.1 as f32;
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
