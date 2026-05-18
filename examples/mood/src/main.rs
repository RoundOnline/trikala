//! Trikala mood prototype — what one runnable Rust+wgpu scene
//! can demonstrate of the engine's capabilities.
//!
//! # Systems in this file
//!
//! - **Two worlds + portal** — `world_a` (night forest) and
//!   `world_b` (sunlit meadow) live in separate vertex buffers but
//!   share local coordinates. A Doraemon-style door is the same
//!   physical position in both. Looking at it from world A's open
//!   side, a quad samples an offscreen texture into which world B
//!   was just rendered from a virtual camera matching the real one
//!   — that gives a real "see through to the other world" portal.
//!   Walking the player through the door's rectangle in the
//!   allowed direction swaps `current_world`. Each world has its
//!   own "open" side; the other is a closed frame the player can
//!   approach but not phase through.
//! - **Fireflies** — four little glowing creatures with their own
//!   pos+vel state, a soft spring back toward the player, and per-
//!   firefly sin-wander so they never quite synchronise. Each
//!   carries a world tag and walks through the door behind the
//!   player when their tag and the player's diverge — so they
//!   don't render in the wrong world. They contribute warm point-
//!   light to the scene; the shader masks off-world fireflies via
//!   `fly_pos.w`.
//! - **Skinned character** — see `character.rs`. The character is
//!   a real rigged glTF (Erika Archer from Mixamo, bundled with a
//!   full locomotion pack: idle / walk / run / jump / strafe /
//!   turn) driven by an animation state machine with cross-fades.
//!   Body yaw smoothly tracks movement direction for forward
//!   locomotion; for a pure strafe (only A or D) the body stays
//!   facing the camera and a dedicated sidestep clip plays. When
//!   the camera yaw drifts away from the body's facing while
//!   standing still, a turn-in-place clip plays as body_yaw
//!   catches up.
//! - **Shadows** — one shadow map per pass, written by the
//!   directional light. Primitive scene geometry uses the basic
//!   shadow pipeline; the skinned mesh has its own skin-shadow
//!   pipeline that applies bone matrices before depth write.
//!
//! Controls
//!   WASD / Arrows — move (camera) / look
//!   Mouse         — look
//!   (default)     — run
//!   Shift         — hold to walk
//!   Space         — jump
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

// Grass blade vertex — pos is the bind-pose world position, base_xz
// is the blade's footprint on the ground (so every vertex of one
// blade shares this) for the player-distance bend, and phase is a
// per-blade randomiser for the wind sway.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GrassVertex {
    pos: [f32; 3],
    tip_factor: f32,
    base_xz: [f32; 2],
    phase: f32,
    _pad: f32,
}

// Water and sand vertices — just position (vertex shaders compute
// the displacement / shading procedurally).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PosVertex {
    pos: [f32; 3],
}

// Footprint decal vertex — world position + UV inside the quad +
// normalized age (0 fresh, 1 expired) for the fade.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct DecalVertex {
    pos: [f32; 3],
    uv: [f32; 2],
    age01: f32,
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
    // player.xyz + wall-clock seconds (in .w). Used by environment
    // shaders that react to the player (grass bend, water ripples)
    // or animate over time. Existing shaders don't read this field;
    // it sits at the tail of the buffer so they don't need updating.
    player_pos: [f32; 4],
    // Up to 8 active water ripples — xz origin in .xy, spawn time in
    // .z, .w = 1 when slot is active else 0. Read by the water
    // shader to displace the surface.
    ripples: [[f32; 4]; 8],
    // Door-transition tint, drawn as a fullscreen quad at the end of
    // the main pass. .rgb is the colour (black for the standard
    // Doraemon-door fade); .w is the alpha (0 = no fade visible,
    // 1 = fully opaque). The fade pipeline reads only this field.
    fade: [f32; 4],
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
// ─────────────────────────────────────────────────────────────────
// Locomotion + animation tuning
// ─────────────────────────────────────────────────────────────────
// World movement speeds (m/s) — what the player actually travels.
// These are deliberately set equal to the corresponding clip's
// authored speed below, so every locomotion clip plays at its
// natural rate (state_rate = MOVE / NOMINAL = 1.0). That keeps the
// feet planted exactly and the limbs at the cadence the animator
// intended — no fake-fast walk, no skating, no rushed gait. The
// trade-off is gameplay speed: the character moves at realistic
// human pace (~1.4 m/s walk, ~4 m/s run). If a faster game feel is
// wanted later, raise a MOVE_* value above its NOMINAL — but the
// matching clip will visibly speed up to compensate.
const MOVE_WALK: f32 = 1.8;
const MOVE_RUN: f32  = 5.0;
// "Nominal" clip speeds (m/s) — the locomotion rate each Mixamo
// clip is authored at. Estimated from the Locomotion Pack. If a
// clip looks too slow at MOVE = NOMINAL, lower that NOMINAL (and
// the rate will rise above 1.0× until it matches the visible
// cadence).
const NOMINAL_WALK_SPEED: f32 = 1.4;
const NOMINAL_RUN_SPEED: f32  = 4.0;

// Doorways sized so the character (~2.07 m after the 1.15× model
// scale) clears with ~0.5 m headroom. Two doors stand on the north
// wall: door A flips bit 0 of `current_world`, door B flips bit 1,
// so the four-room set is reachable from any starting room with at
// most two crossings.
//
//   current_world bits:
//     bit 0 (door A) — water (0) ↔ grass (1), sand (2) ↔ free (3)
//     bit 1 (door B) — water (0) ↔ sand (2), grass (1) ↔ free (3)
const PORTAL_A_POS: Vec3 = Vec3::new(3.5, 1.30, -5.0);
const PORTAL_B_POS: Vec3 = Vec3::new(-3.5, 1.30, -5.0);
// Half-width is wider than the character (~1.04 m half-width after
// the 1.15× model scale) so the posts don't clip the player's arms
// when the camera looks at them through the doorway right after a
// teleport.
const PORTAL_HALF_W: f32 = 1.20;
const PORTAL_HALF_H: f32 = 1.30;

// Interactive ground zones — each is the central feature of its
// themed room. Bounds drive both the mesh builders and the
// per-frame `in_patch()` foot-on-feature checks (which only matter
// when the player is in the matching room).
//
// Placement around the doors (at z = -5):
//   spawn (z = +6)  ──south──  doors  ──north──  z = -12
//
//   Water: south side only (spawn area)
//   Grass: north side only (the "other side" of the doors, what
//          the player walks into after stepping through)
//   Sand : both sides (covers the whole play area)
const WATER_MIN: Vec3 = Vec3::new(-9.0, 0.0, -3.5);
const WATER_MAX: Vec3 = Vec3::new( 9.0, 0.0,  5.0);
const WATER_LEVEL: f32 = 0.04;
const WATER_GRID: u32 = 56;

const GRASS_MIN: Vec3 = Vec3::new(-9.0, 0.0, -12.0);
const GRASS_MAX: Vec3 = Vec3::new( 9.0, 0.0,  -5.5);
const GRASS_DENSITY: usize = 4500;

const SAND_MIN: Vec3 = Vec3::new(-9.0, 0.0, -12.0);
const SAND_MAX: Vec3 = Vec3::new( 9.0, 0.0,   8.0);
const SAND_Y: f32 = 0.005;

const MAX_RIPPLES: usize = 8;
const MAX_FOOTPRINTS: usize = 60;
const PORTAL_NORMAL: Vec3 = Vec3::new(0.0, 0.0, 1.0);

// Door transition — driven by the player's distance to the nearest
// door rather than a timer. Within BLEND_RADIUS of a door's xz
// position, the destination room's portal texture is alpha-blended
// over the rendered scene; alpha is linear, 1.0 right at the door
// and 0.0 at the radius edge. The actual world flip is instant at
// the door plane, but visually that moment sits at alpha = 1.0 so
// it reads as a seamless dissolve in both directions.
const BLEND_RADIUS: f32 = 0.6;

// Room theme — feature shown + ambient palette. Indices into
// `worlds[]` are the bit-pair encoding above.
#[derive(Copy, Clone, PartialEq, Eq)]
enum RoomTheme { Water = 0, Grass = 1, Sand = 2, Free = 3 }

fn room_theme(world: u8) -> RoomTheme {
    match world & 0b11 {
        0 => RoomTheme::Water,
        1 => RoomTheme::Grass,
        2 => RoomTheme::Sand,
        _ => RoomTheme::Free,
    }
}
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

/// Trees ringing the playable area — same xz positions in every
/// room so the layout reads consistently through the portals.
/// Trees within ~7 m of either door (A at +3, B at -3, both at z=-5)
/// have been pruned so the doorway is visually clear from both
/// sides and through the portal texture.
const LANDSCAPE_TREES: [(f32, f32, f32); 14] = [
    // East edge
    (14.0, -3.0, 1.0), (15.0, 2.0, 1.1), (14.0, 7.0, 0.9),
    (14.0, -9.0, 1.2),
    // South edge
    (-6.0, 11.0, 1.0), (2.0, 12.0, 1.2), (8.0, 12.0, 0.9),
    // West edge
    (-14.0, -2.0, 1.2), (-14.0, 4.0, 1.0), (-13.0, -7.0, 1.1), (-14.0, 9.0, 1.3),
    // Far-out accents kept for depth
    (-3.0, 4.0, 0.9),
    (8.0, -8.5, 0.8), (12.0, -8.5, 0.9),
];

// Each themed room is a minimal scene — a flat floor in the theme's
// tint, a single bright sun/moon cube, and the global tree ring for
// depth. The room's defining feature (water / grass / sand) is drawn
// separately at render time so this builder doesn't even need to
// know which feature is involved.
fn build_room(theme: RoomTheme) -> (Vec<Vertex>, Vec<u32>, WorldEnv) {
    let mut v = Vec::new();
    let mut i = Vec::new();

    let (floor_col, sky, ambient, moon_dir, moon_col, sun_pos, sun_col, tree_canopy) = match theme {
        RoomTheme::Water => (
            Vec3::new(0.06, 0.10, 0.14),
            wgpu::Color { r: 0.10, g: 0.18, b: 0.30, a: 1.0 },
            [0.18, 0.22, 0.30],
            Vec3::new(6.0, 20.0, -22.0).normalize(),
            [0.55, 0.70, 1.00, 1.0],
            Vec3::new(6.0, 20.0, -22.0),
            Vec3::new(2.0, 2.6, 3.2),
            Vec3::new(0.18, 0.32, 0.40),
        ),
        RoomTheme::Grass => (
            Vec3::new(0.30, 0.52, 0.20),
            wgpu::Color { r: 0.62, g: 0.78, b: 0.92, a: 1.0 },
            [0.50, 0.55, 0.58],
            Vec3::new(-12.0, 22.0, 18.0).normalize(),
            [1.40, 1.30, 1.05, 1.0],
            Vec3::new(-12.0, 22.0, 18.0),
            Vec3::new(3.0, 2.7, 2.0),
            Vec3::new(0.55, 0.78, 0.32),
        ),
        RoomTheme::Sand => (
            Vec3::new(0.74, 0.62, 0.42),
            wgpu::Color { r: 0.92, g: 0.68, b: 0.45, a: 1.0 },
            [0.55, 0.45, 0.32],
            Vec3::new(-18.0, 12.0, -8.0).normalize(),
            [1.55, 1.10, 0.70, 1.0],
            Vec3::new(-18.0, 12.0, -8.0),
            Vec3::new(3.5, 2.4, 1.4),
            Vec3::new(0.55, 0.40, 0.22),
        ),
        RoomTheme::Free => (
            Vec3::new(0.10, 0.10, 0.13),
            wgpu::Color { r: 0.04, g: 0.04, b: 0.08, a: 1.0 },
            [0.16, 0.16, 0.22],
            Vec3::new(0.0, 25.0, 0.0).normalize(),
            [0.70, 0.65, 0.85, 1.0],
            Vec3::new(0.0, 25.0, -5.0),
            Vec3::new(1.0, 0.9, 1.4),
            Vec3::new(0.20, 0.18, 0.24),
        ),
    };

    // Floor and sun.
    push_plane(&mut v, &mut i, Vec3::ZERO, 60.0, floor_col);
    push_cube(&mut v, &mut i, sun_pos, Vec3::new(3.5, 3.5, 3.5), sun_col, 1.0);

    // Tree ring — kept in every room as a horizon edge. The "free"
    // room uses a darker canopy so it doesn't read as a meadow.
    for &(x, z, s) in &LANDSCAPE_TREES {
        push_tree(&mut v, &mut i, x, z, s, tree_canopy);
    }

    // No fountain, no desk, no bench, no flowers — each room is
    // intentionally clean so the central feature carries the read.

    let env = WorldEnv {
        lamp_pos: Vec3::new(0.0, 2.0, 0.0),
        lamp_color: [0.0, 0.0, 0.0, 0.0],
        moon_dir,
        moon_color: moon_col,
        ambient,
        sky,
        light_origin: sun_pos,
    };
    (v, i, env)
}

/// The Doraemon-door mesh for both portals, concatenated. Rendered
/// separately from the room mesh so the portal texture doesn't show
/// duplicate door frames ("door inside door").
fn build_door_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    push_doraemon_door(&mut v, &mut i, PORTAL_A_POS);
    push_doraemon_door(&mut v, &mut i, PORTAL_B_POS);
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
    vel: Vec3,
    phase: [f32; 3],
}

fn step_fireflies(flies: &mut [Firefly; FIREFLY_COUNT], player_pos: Vec3, dt: f32, t: f32) {
    let player_target = player_pos + Vec3::new(0.0, 0.9, 0.0);
    for fly in flies.iter_mut() {
        let to_target = player_target - fly.pos;
        let dist = to_target.length();
        let dir = if dist > 0.001 { to_target / dist } else { Vec3::ZERO };
        let pull = if dist > 1.6 {
            dir * 5.0
        } else if dist < 0.45 {
            -dir * 3.0
        } else {
            dir * 0.6
        };
        let p = fly.phase;
        let wander = Vec3::new(
            (t * 1.7 + p[0]).sin() * 2.2 + (t * 0.9 + p[1]).cos() * 1.0,
            (t * 2.3 + p[1]).sin() * 1.4 + (t * 1.1 + p[2]).cos() * 0.5,
            (t * 1.3 + p[2]).sin() * 2.2 + (t * 0.7 + p[0]).cos() * 1.0,
        );
        fly.vel += (pull + wander) * dt;
        fly.vel *= (1.0 - 1.5 * dt).max(0.0);
        let speed = fly.vel.length();
        if speed > 4.0 {
            fly.vel *= 4.0 / speed;
        }
        fly.pos += fly.vel * dt;
        if fly.pos.y < 0.15 {
            fly.pos.y = 0.15;
            fly.vel.y = fly.vel.y.abs() * 0.4;
        }
    }
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

// ─────────────────────────────────────────────────────────────────
// Grass / water / sand / decal mesh builders
// ─────────────────────────────────────────────────────────────────

/// Scatter `density` grass blades across the rectangle `[min..max]`
/// (xz). Each blade is a triangle (two base verts + one tip) with
/// a deterministic pseudo-random position, height and wind phase.
fn build_grass(min: Vec3, max: Vec3, density: usize) -> (Vec<GrassVertex>, Vec<u32>) {
    let mut verts = Vec::with_capacity(density * 3);
    let mut indices = Vec::with_capacity(density * 3);
    for n in 0..density {
        let n32 = n as u32;
        let r1 = ((n32.wrapping_mul(2654435761)) & 0xffff) as f32 / 65535.0;
        let r2 = ((n32.wrapping_mul(1597334677)) & 0xffff) as f32 / 65535.0;
        let r3 = ((n32.wrapping_mul(374761393))  & 0xffff) as f32 / 65535.0;
        let r4 = ((n32.wrapping_mul(2246822519)) & 0xffff) as f32 / 65535.0;
        let bx = min.x + r1 * (max.x - min.x);
        let bz = min.z + r2 * (max.z - min.z);
        let h = 0.35 + r3 * 0.30;        // blade height variance
        let w = 0.025 + r4 * 0.020;      // blade half-width
        let phase = r3 * std::f32::consts::TAU;
        let base = verts.len() as u32;
        verts.push(GrassVertex { pos: [bx - w, 0.0, bz], tip_factor: 0.0, base_xz: [bx, bz], phase, _pad: 0.0 });
        verts.push(GrassVertex { pos: [bx + w, 0.0, bz], tip_factor: 0.0, base_xz: [bx, bz], phase, _pad: 0.0 });
        verts.push(GrassVertex { pos: [bx,     h,   bz], tip_factor: 1.0, base_xz: [bx, bz], phase, _pad: 0.0 });
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
    }
    (verts, indices)
}

/// Subdivided horizontal plane filling `[min..max]` (xz) at y=`y`,
/// with `grid` × `grid` vertices. The vertex shader displaces this
/// in Y for waves and ripples.
fn build_water(min: Vec3, max: Vec3, y: f32, grid: u32) -> (Vec<PosVertex>, Vec<u32>) {
    let mut verts = Vec::with_capacity((grid * grid) as usize);
    let mut indices = Vec::with_capacity(((grid - 1) * (grid - 1) * 6) as usize);
    for gz in 0..grid {
        let tz = gz as f32 / (grid - 1) as f32;
        let z = min.z + tz * (max.z - min.z);
        for gx in 0..grid {
            let tx = gx as f32 / (grid - 1) as f32;
            let x = min.x + tx * (max.x - min.x);
            verts.push(PosVertex { pos: [x, y, z] });
        }
    }
    for gz in 0..grid - 1 {
        for gx in 0..grid - 1 {
            let i0 = gz * grid + gx;
            let i1 = gz * grid + gx + 1;
            let i2 = (gz + 1) * grid + gx;
            let i3 = (gz + 1) * grid + gx + 1;
            indices.extend_from_slice(&[i0, i1, i2, i2, i1, i3]);
        }
    }
    (verts, indices)
}

/// Flat sandy patch at `y` covering `[min..max]` (xz).
fn build_sand(min: Vec3, max: Vec3, y: f32) -> (Vec<PosVertex>, Vec<u32>) {
    let verts = vec![
        PosVertex { pos: [min.x, y, min.z] },
        PosVertex { pos: [max.x, y, min.z] },
        PosVertex { pos: [min.x, y, max.z] },
        PosVertex { pos: [max.x, y, max.z] },
    ];
    let indices = vec![0, 1, 2, 2, 1, 3];
    (verts, indices)
}

/// True when (x,z) is inside the open xz rectangle `[min..max]`.
fn in_patch(x: f32, z: f32, min: Vec3, max: Vec3) -> bool {
    x >= min.x && x <= max.x && z >= min.z && z <= max.z
}

/// Build the footprint-decal mesh from the active footprint queue.
/// Each entry is one short quad oriented along the body yaw it was
/// stamped at; `age01` drives the fragment shader's fade.
fn build_decals(
    footprints: &std::collections::VecDeque<(Vec3, f32, f32, f32)>,
    now: f32,
) -> (Vec<DecalVertex>, Vec<u32>) {
    const LIFETIME: f32 = 12.0;
    let mut verts = Vec::with_capacity(footprints.len() * 4);
    let mut indices = Vec::with_capacity(footprints.len() * 6);
    for &(pos, yaw, t0, side) in footprints.iter() {
        let age = now - t0;
        if !(0.0..=LIFETIME).contains(&age) {
            continue;
        }
        let age01 = age / LIFETIME;
        let (sy, cy) = yaw.sin_cos();
        // forward (-Z when body_yaw = 0) and right vectors.
        let fx = -sy;
        let fz = -cy;
        let rx = cy;
        let rz = -sy;
        // Shift left/right of body centre for each foot, then build
        // a small quad in the ground plane.
        let lateral = 0.10 * side;
        let cx = pos.x + rx * lateral;
        let cz = pos.z + rz * lateral;
        let cy_ = pos.y + 0.012; // a hair above the sand to avoid z-fight
        let half_l = 0.13;
        let half_w = 0.07;
        let base = verts.len() as u32;
        for &(u, vu) in &[(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let dx = fx * vu * half_l + rx * u * half_w;
            let dz = fz * vu * half_l + rz * u * half_w;
            verts.push(DecalVertex {
                pos: [cx + dx, cy_, cz + dz],
                uv: [(u + 1.0) * 0.5, (vu + 1.0) * 0.5],
                age01,
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    (verts, indices)
}

// The "magic surface" of one door — a quad inside the frame, facing
// +z. Renderer draws each door's quad with its own portal texture
// bound so each window into the destination room samples correctly.
fn build_portal_quad(center: Vec3) -> (Vec<Vertex>, Vec<u32>) {
    let cx = center.x;
    let cy = center.y;
    let cz = center.z;
    let hw = PORTAL_HALF_W;
    let hh = PORTAL_HALF_H;
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
// SECTION 4 — Input + Player
// ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Input {
    fwd: bool, back: bool, strafe_l: bool, strafe_r: bool,
    look_up: bool, look_down: bool, look_left: bool, look_right: bool,
    walk_held: bool, // hold Shift to walk (default is run)
    jump_pressed: bool,
    attack_pressed: bool,
    mouse_dx: f32, mouse_dy: f32,
}

struct Player {
    pos: Vec3,
    prev_pos: Vec3,
    yaw: f32,      // camera yaw (mouse / arrow keys)
    pitch: f32,
    body_yaw: f32, // character body yaw, smoothly tracks movement direction
    vel_y: f32,
    on_ground: bool,
    attack_t: f32, // seconds remaining in the current attack swing
}

// Animation state machine — selects which named clip plays and
// cross-fades between consecutive states over a short transition.
// Clip names are the ones baked into the bundled character.glb by
// `tools/build_character.md` (Mixamo Erika Archer + locomotion pack).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum AnimState {
    Idle,
    Walk,
    Run,
    Jump,
}

fn resolve_state_clip(state: AnimState, character: &Character) -> Option<&str> {
    match state {
        AnimState::Idle      => character.find_clip(&["Idle", "Standing", "Stand"]),
        AnimState::Walk      => character.walk_clip_name(),
        AnimState::Run       => character.find_clip(&["Run", "Running", "Jog", "Jogging"]),
        AnimState::Jump      => character.find_clip(&["Jump", "Jumping"]),
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
        println!("[anim] {:?} -> {:?}", self.state, new_state);
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
// SECTION 5 — Shaders
// ─────────────────────────────────────────────────────────────────

const MAIN_SHADER: &str = include_str!("shaders/main.wgsl");

const SKIN_SHADER: &str = include_str!("shaders/skin.wgsl");

const SHADOW_SHADER: &str = include_str!("shaders/shadow.wgsl");

// Portal shader — draws the magic surface inside the door frame.
// Vertex stage transforms by the real camera's view_proj so the
// quad sits where the door is in the current world. Fragment stage
// reads portal_tex at the SCREEN PIXEL the fragment occupies, so
// what shows up in the door is whatever was rendered to portal_tex
// from the corresponding virtual camera — i.e., the other world.
const SKIN_SHADOW_SHADER: &str = include_str!("shaders/skin_shadow.wgsl");

const PORTAL_SHADER: &str = include_str!("shaders/portal.wgsl");

// Grass blade — triangle from two base verts + one tip. Vertex
// shader bends the tip away from the player when within a small
// radius, plus a low-amplitude wind sway. No shadowing (the blades
// are tiny and far below the directional light's relevant range).
const GRASS_SHADER: &str = include_str!("shaders/grass.wgsl");

// Water — subdivided plane. Vertex shader adds two layers of
// background sin waves, then up to 8 expanding ripple wavelets each
// centred on a recent footstep position (decay over a couple of
// seconds). Fragment shader is a flat blue tinted by the sky and
// moon directions.
const WATER_SHADER: &str = include_str!("shaders/water.wgsl");

// Sand — a flat patch with a baked grainy color, no fancy
// displacement. The footprints layer on top of this through the
// DECAL shader below.
const SAND_SHADER: &str = include_str!("shaders/sand.wgsl");

// Door transition — three NDC vertices cover the screen; the
// fragment samples the destination room's portal texture (already
// rendered this frame) at the matching screen pixel and outputs it
// with alpha = `u.fade.w`. That gives a dream-like dissolve of the
// destination room onto the current scene, with alpha driven by how
// close the player is to a door.
const FADE_SHADER: &str = include_str!("shaders/fade.wgsl");

// Footprint decals — small dark oriented quads dropped behind the
// character while on the sand patch. Each quad fades out with age.
const DECAL_SHADER: &str = include_str!("shaders/decal.wgsl");

// ─────────────────────────────────────────────────────────────────
// SECTION 6 — Game state + render
// ─────────────────────────────────────────────────────────────────

const FIREFLY_VBUF_CAPACITY: u64 = 256 * std::mem::size_of::<Vertex>() as u64;
const FIREFLY_IBUF_CAPACITY: u64 = 256 * std::mem::size_of::<u32>() as u64;
const SHADOW_MAP_SIZE: u32 = 1024;

struct WorldGpu {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    index_count: u32,
    env: WorldEnv,
}

/// Render-to-texture portals + the door frame + magic-surface mesh.
/// Both doors share one mesh; indices [0..N) are door A, [N..2N) are
/// door B, with N stored as `indices_per_door`.
struct PortalSystem {
    /// Per-door offscreen colour target where the destination room is
    /// rendered each frame. [0] = door A (bit-0 axis), [1] = door B.
    color_views: [wgpu::TextureView; 2],
    depth_views: [wgpu::TextureView; 2],
    tex_bgs: [wgpu::BindGroup; 2],
    sampler: wgpu::Sampler,
    bgl1: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    indices_per_door: u32,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    door_vbuf: wgpu::Buffer,
    door_ibuf: wgpu::Buffer,
    door_index_count: u32,
}

struct GrassSystem {
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    index_count: u32,
}

struct WaterSystem {
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    index_count: u32,
    /// Active ripples — (world-space xz, spawn time). Capped at 8.
    ripples: std::collections::VecDeque<(Vec3, f32)>,
    /// Packed into the main Uniforms buffer for the water shader.
    ripples_uniform: [[f32; 4]; 8],
}

struct SandSystem {
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    index_count: u32,
}

struct DecalSystem {
    pipeline: wgpu::RenderPipeline,
    /// Footprint mesh rebuilt every frame from `footprints`.
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    index_count: u32,
    vbuf_capacity: usize,
    /// (world position, body_yaw, spawn_time, left_or_right).
    footprints: std::collections::VecDeque<(Vec3, f32, f32, f32)>,
    /// Time of the last footprint spawn, so we space them evenly.
    last_footprint_t: f32,
}

struct FadeSystem {
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
}

/// Fireflies follow the player into whichever room they're in, so we
/// only ever need one mesh buffer for them. They're drawn in the main
/// pass only; the portal pass doesn't include them because the other
/// room shouldn't borrow the player's escort.
struct FireflyMesh {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    count: u32,
}

struct Game {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    depth_view: wgpu::TextureView,
    shadow_view: wgpu::TextureView,

    main_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    skin_pipeline: wgpu::RenderPipeline,
    skin_shadow_pipeline: wgpu::RenderPipeline,
    character: Character,
    anim_ctrl: AnimController,
    /// Pre-computed jump pacing — derived from the Jump clip's
    /// duration so the landing pose lines up with hitting the
    /// ground. See the constants in `new()` for the math.
    jump_impulse: f32,
    jump_clip_start: f32,
    main_bg0: wgpu::BindGroup,
    main_bg1_shadow: wgpu::BindGroup,
    shadow_bg0: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,

    /// 4 themed rooms, indexed by `current_world`.
    worlds: [WorldGpu; 4],
    portal: PortalSystem,
    firefly_mesh: FireflyMesh,

    /// Current room — interpreted as two independent bits:
    /// bit 0 = door-A axis, bit 1 = door-B axis. The enum mapping is
    /// 0=Water, 1=Grass, 2=Sand, 3=Free; see `room_theme()`.
    current_world: u8,
    player: Player,
    input: Input,
    third_person: bool,
    fireflies: [Firefly; FIREFLY_COUNT],
    time: f32,
    last_frame: std::time::Instant,

    grass: GrassSystem,
    water: WaterSystem,
    sand: SandSystem,
    decal: DecalSystem,
    fade: FadeSystem,
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

        // ── Build the four themed rooms ──
        let themes = [RoomTheme::Water, RoomTheme::Grass, RoomTheme::Sand, RoomTheme::Free];
        let worlds: [WorldGpu; 4] = themes.map(|theme| {
            let (verts, indices, env) = build_room(theme);
            let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world-v"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world-i"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            WorldGpu { vbuf, ibuf, index_count: indices.len() as u32, env }
        });

        // Portal quad mesh — both doors concatenated into one buffer.
        // First 6 indices = door A's quad, next 6 = door B's.
        let (pv_a, pi_a) = build_portal_quad(PORTAL_A_POS);
        let (pv_b, pi_b) = build_portal_quad(PORTAL_B_POS);
        let mut pv = pv_a.clone();
        let base = pv.len() as u32;
        pv.extend(pv_b.clone());
        let mut pi = pi_a.clone();
        pi.extend(pi_b.iter().map(|i| i + base));
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
        let portal_indices_per_door: u32 = pi_a.len() as u32;

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

        let firefly_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("firefly-v"),
            size: FIREFLY_VBUF_CAPACITY,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let firefly_ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("firefly-i"),
            size: FIREFLY_IBUF_CAPACITY,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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

        let (pcv_a, pdv_a) = make_portal_targets(&device, config.width, config.height, format);
        let (pcv_b, pdv_b) = make_portal_targets(&device, config.width, config.height, format);
        let portal_color_views = [pcv_a, pcv_b];
        let portal_depth_views = [pdv_a, pdv_b];
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
        let mk_portal_bg = |view: &wgpu::TextureView| device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("portal-tex-bg"), layout: &bgl1_portal,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&portal_sampler) },
            ],
        });
        let portal_tex_bgs = [
            mk_portal_bg(&portal_color_views[0]),
            mk_portal_bg(&portal_color_views[1]),
        ];

        // ── Pipelines ──
        let main_pipeline        = build_main_pipeline       (&device, format, &bgl0, &bgl1_shadow);
        let shadow_pipeline      = build_shadow_pipeline     (&device, &bgl0);
        let portal_pipeline      = build_portal_pipeline     (&device, format, &bgl0, &bgl1_portal);
        let bone_bgl             = build_bone_bgl            (&device);
        let skin_pipeline        = build_skin_pipeline       (&device, format, &bgl0, &bgl1_shadow, &bone_bgl);
        let skin_shadow_pipeline = build_skin_shadow_pipeline(&device, &bgl0, &bone_bgl);
        let character = Character::load(
            &device,
            &queue,
            concat!(env!("CARGO_MANIFEST_DIR"), "/assets/character.glb"),
            &bone_bgl,
        );
        let grass_pipeline       = build_grass_pipeline      (&device, format, &bgl0);
        let water_pipeline       = build_water_pipeline      (&device, format, &bgl0);
        let sand_pipeline        = build_sand_pipeline       (&device, format, &bgl0);
        let decal_pipeline       = build_decal_pipeline      (&device, format, &bgl0);
        let fade_pipeline        = build_fade_pipeline       (&device, format, &bgl0, &bgl1_portal);

        // ── Per-feature meshes (vertex/index buffers) ──
        let (grass_verts, grass_indices) = build_grass(GRASS_MIN, GRASS_MAX, GRASS_DENSITY);
        let grass_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grass-vb"),
            contents: bytemuck::cast_slice(&grass_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let grass_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grass-ib"),
            contents: bytemuck::cast_slice(&grass_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let grass_index_count = grass_indices.len() as u32;

        let (water_verts, water_indices) = build_water(WATER_MIN, WATER_MAX, WATER_LEVEL, WATER_GRID);
        let water_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water-vb"),
            contents: bytemuck::cast_slice(&water_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let water_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water-ib"),
            contents: bytemuck::cast_slice(&water_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let water_index_count = water_indices.len() as u32;

        let (sand_verts, sand_indices) = build_sand(SAND_MIN, SAND_MAX, SAND_Y);
        let sand_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sand-vb"),
            contents: bytemuck::cast_slice(&sand_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sand_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sand-ib"),
            contents: bytemuck::cast_slice(&sand_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let sand_index_count = sand_indices.len() as u32;

        let decal_vbuf_capacity = MAX_FOOTPRINTS * 4;
        let decal_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("decal-vb"),
            size: (decal_vbuf_capacity * std::mem::size_of::<DecalVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let decal_ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("decal-ib"),
            size: (MAX_FOOTPRINTS * 6 * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // NDC triangle (oversized so it covers the screen) for the fade overlay.
        let fade_quad = [
            PosVertex { pos: [-1.0, -1.0, 0.0] },
            PosVertex { pos: [ 3.0, -1.0, 0.0] },
            PosVertex { pos: [-1.0,  3.0, 0.0] },
        ];
        let fade_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fade-vb"),
            contents: bytemuck::cast_slice(&fade_quad),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let _ = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
        window.set_cursor_visible(false);

        // Spawn in the south of the room, facing -z toward the doors.
        let spawn = Vec3::new(0.0, 0.0, 6.0);
        // Jump pacing — derived once from the Jump clip's duration
        // so the landing pose finishes the moment the player hits
        // the ground. ANTICIPATION_SKIP trims the opening crouch we
        // can't show (the launch is gameplay-instantaneous).
        // airborne_time = (1 − skip) × clip_duration
        // vel_y_initial = airborne_time × g / 2
        // apex = vel_y² / (2g)
        const GRAVITY: f32 = 12.0;
        const ANTICIPATION_SKIP: f32 = 0.25;
        let jump_clip_dur = character
            .clip_duration("Jump")
            .unwrap_or(2.0);
        let airborne_time = jump_clip_dur * (1.0 - ANTICIPATION_SKIP);
        let jump_impulse = airborne_time * GRAVITY / 2.0;
        let jump_clip_start = jump_clip_dur * ANTICIPATION_SKIP;
        println!(
            "[jump] clip={:.2}s, anticipation_skip={:.0}%, airborne={:.2}s, v0={:.2} m/s, apex={:.2} m",
            jump_clip_dur,
            ANTICIPATION_SKIP * 100.0,
            airborne_time,
            jump_impulse,
            jump_impulse * jump_impulse / (2.0 * GRAVITY),
        );
        Self {
            window, surface, device, queue, config,
            depth_view, shadow_view,
            main_pipeline, shadow_pipeline,
            skin_pipeline, skin_shadow_pipeline, character,
            anim_ctrl: AnimController::new(),
            jump_impulse, jump_clip_start,
            main_bg0, main_bg1_shadow, shadow_bg0,
            uniform_buf,
            worlds,
            portal: PortalSystem {
                color_views: portal_color_views,
                depth_views: portal_depth_views,
                tex_bgs: portal_tex_bgs,
                sampler: portal_sampler,
                bgl1: bgl1_portal,
                pipeline: portal_pipeline,
                indices_per_door: portal_indices_per_door,
                vbuf: portal_vbuf,
                ibuf: portal_ibuf,
                door_vbuf, door_ibuf, door_index_count,
            },
            firefly_mesh: FireflyMesh {
                vbuf: firefly_vbuf,
                ibuf: firefly_ibuf,
                count: 0,
            },
            current_world: 0, // start in Water room
            player: Player {
                pos: spawn, prev_pos: spawn,
                // Face -z toward the doors on the north wall.
                yaw: 0.0,
                pitch: 0.0,
                body_yaw: 0.0,
                vel_y: 0.0,
                on_ground: true,
                attack_t: 0.0,
            },
            input: Input::default(),
            third_person: true,
            fireflies: {
                let make = |off: Vec3, phase: [f32; 3]| {
                    let p = spawn + off;
                    Firefly { pos: p, vel: Vec3::ZERO, phase }
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
            grass: GrassSystem {
                pipeline: grass_pipeline,
                vbuf: grass_vbuf,
                ibuf: grass_ibuf,
                index_count: grass_index_count,
            },
            water: WaterSystem {
                pipeline: water_pipeline,
                vbuf: water_vbuf,
                ibuf: water_ibuf,
                index_count: water_index_count,
                ripples: std::collections::VecDeque::with_capacity(MAX_RIPPLES),
                ripples_uniform: [[0.0; 4]; MAX_RIPPLES],
            },
            sand: SandSystem {
                pipeline: sand_pipeline,
                vbuf: sand_vbuf,
                ibuf: sand_ibuf,
                index_count: sand_index_count,
            },
            decal: DecalSystem {
                pipeline: decal_pipeline,
                vbuf: decal_vbuf,
                ibuf: decal_ibuf,
                index_count: 0,
                vbuf_capacity: decal_vbuf_capacity,
                footprints: std::collections::VecDeque::with_capacity(MAX_FOOTPRINTS),
                last_footprint_t: -10.0,
            },
            fade: FadeSystem {
                pipeline: fade_pipeline,
                vbuf: fade_vbuf,
            },
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
        self.depth_view = make_depth(&self.device, self.config.width, self.config.height, "depth");
        for i in 0..2 {
            let (pcv, pdv) = make_portal_targets(&self.device, self.config.width, self.config.height, self.config.format);
            self.portal.color_views[i] = pcv;
            self.portal.depth_views[i] = pdv;
            self.portal.tex_bgs[i] = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("portal-tex-bg"),
                layout: &self.portal.bgl1,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.portal.color_views[i]) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.portal.sampler) },
                ],
            });
        }
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
            KeyCode::ShiftLeft | KeyCode::ShiftRight => self.input.walk_held = pressed,
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
        // One locomotion model in every direction: default is a run,
        // Shift downshifts to a walk. The body rotates to face the
        // movement vector — sidestepping just means turning 90° and
        // running into that direction, no dedicated strafe clip.
        let move_speed = if self.input.walk_held { MOVE_WALK } else { MOVE_RUN };
        if moving {
            let m_norm = m.normalize();
            self.player.pos += m_norm * move_speed * dt;
            let target_body_yaw = (-m_norm.x).atan2(-m_norm.z);
            let tau = std::f32::consts::TAU;
            let diff = (target_body_yaw - self.player.body_yaw + std::f32::consts::PI)
                .rem_euclid(tau) - std::f32::consts::PI;
            let alpha = (10.0 * dt).min(1.0);
            self.player.body_yaw += diff * alpha;
        }

        if self.input.jump_pressed && self.player.on_ground {
            // vel_y is pre-computed in new() so airborne time matches
            // the Jump clip's playable portion at 1× rate — landing
            // pose finishes the moment the player hits the ground.
            self.player.vel_y = self.jump_impulse;
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

        // ── Teleport (both doors) ──
        // Door A flips bit 0, door B flips bit 1. The flip is
        // instant at the door plane; the visual continuity comes
        // from the distance-based portal-blend overlay in render(),
        // which is at peak alpha while the flip happens so the
        // moment of swap is invisible.
        for (door_pos, bit) in [(PORTAL_A_POS, 1u8), (PORTAL_B_POS, 2u8)] {
            let prev_d = (self.player.prev_pos - door_pos).dot(PORTAL_NORMAL);
            let curr_d = (self.player.pos - door_pos).dot(PORTAL_NORMAL);
            let local = self.player.pos - door_pos;
            let inside_x = local.x.abs() < PORTAL_HALF_W + 0.25;
            let inside_y = local.y > -PORTAL_HALF_H - 0.20 && local.y < PORTAL_HALF_H + 1.20;
            if prev_d.signum() != curr_d.signum() && inside_x && inside_y {
                self.current_world ^= bit;
            }
        }

        // ── Door-post collisions (both doors) ──
        // Each frame's two vertical posts block the player. The
        // doorway between the posts is open in both directions.
        for door_pos in [PORTAL_A_POS, PORTAL_B_POS] {
            let z_min = door_pos.z - PORTAL_DEPTH_T;
            let z_max = door_pos.z + PORTAL_DEPTH_T;
            let left_min_x = door_pos.x - PORTAL_HALF_W - 2.0 * PORTAL_POST_T;
            let left_max_x = door_pos.x - PORTAL_HALF_W;
            let right_min_x = door_pos.x + PORTAL_HALF_W;
            let right_max_x = door_pos.x + PORTAL_HALF_W + 2.0 * PORTAL_POST_T;
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

        // Door A flips bit 0; door B flips bit 1. From the current
        // room each door is a window into the room you'd arrive in if
        // you stepped through it.
        let cur_world = self.current_world;
        let dest_a = cur_world ^ 1; // door A destination
        let dest_b = cur_world ^ 2; // door B destination
        let cur = &self.worlds[cur_world as usize];

        // Each door has one "open" side per room — the side the
        // magic surface renders on. The side flips with every
        // crossing, so after teleporting the player emerges on the
        // door's new open side (the one they're now in front of from
        // the new room's perspective). This is what stops the
        // "character hidden behind the door's portal quad" bug right
        // after teleporting.
        let door_a_active_south = (cur_world & 1) == 0;
        let door_b_active_south = (cur_world & 2) == 0;
        let cam_a_south_of_door = cam.z > PORTAL_A_POS.z;
        let cam_b_south_of_door = cam.z > PORTAL_B_POS.z;
        let portal_a_visible = door_a_active_south == cam_a_south_of_door;
        let portal_b_visible = door_b_active_south == cam_b_south_of_door;

        // Player movement check (for walking animation flag).
        let m_input = self.input.fwd as i32 + self.input.back as i32
            + self.input.strafe_l as i32 + self.input.strafe_r as i32;
        let moving = m_input > 0 && self.player.on_ground;

        // Animation state machine — pick a target state from the
        // player's situation, hand it to the controller (which
        // handles the cross-fade), then assemble weighted clip
        // samples and let the skin shader pose the body.
        let target_state = if !self.player.on_ground {
            AnimState::Jump
        } else if moving && self.input.walk_held {
            AnimState::Walk
        } else if moving {
            AnimState::Run
        } else {
            AnimState::Idle
        };
        let prev_state = self.anim_ctrl.state;
        self.anim_ctrl.set(target_state);
        if target_state == AnimState::Jump && prev_state != AnimState::Jump {
            self.anim_ctrl.state_time = self.jump_clip_start;
        }

        // Body-yaw catch-up while idle — when the player rotates the
        // camera away from where the character is facing, rotate the
        // body smoothly toward the camera yaw over ~1.4 s. No turn
        // clip is played; the Idle pose just rotates as a unit, which
        // reads cleaner than mixing Mixamo's authored turn (which has
        // its own root rotation and double-rotated visibly).
        let tau = std::f32::consts::TAU;
        let cam_body_diff = (self.player.yaw - self.player.body_yaw + std::f32::consts::PI)
            .rem_euclid(tau) - std::f32::consts::PI;
        if !moving && cam_body_diff.abs() > 0.001 {
            let alpha = (5.0 * dt).min(1.0);
            self.player.body_yaw += cam_body_diff * alpha;
        }

        // Animation play rate — for locomotion clips we scale by
        // (world-speed / clip's authored speed) so the feet plant.
        // The MOVE_* / NOMINAL_*_SPEED constants at the top of the
        // file are the only knobs needed to retune the whole feel.
        let state_rate = match self.anim_ctrl.state {
            AnimState::Walk => MOVE_WALK / NOMINAL_WALK_SPEED,
            AnimState::Run  => MOVE_RUN / NOMINAL_RUN_SPEED,
            // Jump plays at natural rate — vel_y is sized in new()
            // so the airborne window matches the clip remainder.
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
        self.character.update(
            &self.queue,
            self.player.pos,
            self.player.body_yaw,
            &samples,
        );

        // Advance the wall clock first — every system below reads it.
        self.time += dt;

        // Ripples — spawn one when the player is moving on the pond
        // surface, cap the active set at MAX_RIPPLES, and pack the
        // result into the uniform array for the water shader.
        let on_water = self.player.on_ground
            && in_patch(self.player.pos.x, self.player.pos.z, WATER_MIN, WATER_MAX);
        let walking_pace = self.player.pos.distance(self.player.prev_pos) > 0.0;
        if on_water && walking_pace {
            // Stagger by step distance so a slow walk gets fewer than a run.
            let last = self.water.ripples.back().map(|r| r.1).unwrap_or(-10.0);
            if self.time - last > 0.35 {
                if self.water.ripples.len() == MAX_RIPPLES {
                    self.water.ripples.pop_front();
                }
                self.water.ripples.push_back((self.player.pos, self.time));
            }
        }
        // Drop expired ripples (lifetime ~2 s — must match the shader).
        while let Some(&(_, t)) = self.water.ripples.front() {
            if self.time - t > 2.0 {
                self.water.ripples.pop_front();
            } else {
                break;
            }
        }
        self.water.ripples_uniform = [[0.0; 4]; MAX_RIPPLES];
        for (i, &(p, t0)) in self.water.ripples.iter().enumerate() {
            self.water.ripples_uniform[i] = [p.x, p.z, t0, 1.0];
        }

        // Footprints — stamp one quad per step (alternating left/right
        // side) when the player walks across the sand. The decal mesh
        // is rebuilt every frame because the `age01` attribute fades
        // each existing print over time.
        let on_sand = self.player.on_ground
            && in_patch(self.player.pos.x, self.player.pos.z, SAND_MIN, SAND_MAX);
        if on_sand && walking_pace && self.time - self.decal.last_footprint_t > 0.34 {
            if self.decal.footprints.len() == MAX_FOOTPRINTS {
                self.decal.footprints.pop_front();
            }
            let side = if self.decal.footprints.len().is_multiple_of(2) { 1.0 } else { -1.0 };
            self.decal.footprints.push_back((self.player.pos, self.player.body_yaw, self.time, side));
            self.decal.last_footprint_t = self.time;
        }
        // Drop fully-faded prints.
        while let Some(&(_, _, t0, _)) = self.decal.footprints.front() {
            if self.time - t0 > 12.0 {
                self.decal.footprints.pop_front();
            } else {
                break;
            }
        }
        let (dv, di) = build_decals(&self.decal.footprints, self.time);
        if !dv.is_empty() && dv.len() <= self.decal.vbuf_capacity {
            self.queue.write_buffer(&self.decal.vbuf, 0, bytemuck::cast_slice(&dv));
            self.queue.write_buffer(&self.decal.ibuf, 0, bytemuck::cast_slice(&di));
        }
        self.decal.index_count = di.len() as u32;

        // Fireflies follow the player through every door, so we only
        // need one mesh buffer.
        step_fireflies(&mut self.fireflies, self.player.pos, dt, self.time);
        let positions: Vec<Vec3> = self.fireflies.iter().map(|f| f.pos).collect();
        let (fv, fi) = build_fireflies(&positions);
        self.firefly_mesh.count = fi.len() as u32;
        if !fv.is_empty() {
            self.queue.write_buffer(&self.firefly_mesh.vbuf, 0, bytemuck::cast_slice(&fv));
            self.queue.write_buffer(&self.firefly_mesh.ibuf, 0, bytemuck::cast_slice(&fi));
        }

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };
        let frame_view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = self.device.create_command_encoder(&Default::default());

        // Render the destination rooms into the per-door portal textures.
        // Always render both even when the player isn't facing the door —
        // keeps the texture content fresh; the cost is two small offscreen
        // passes.
        for door_idx in 0..2usize {
            let dest = if door_idx == 0 { dest_a } else { dest_b } as usize;
            self.record_portal_pass(&mut enc, door_idx, dest, cam, proj * view);
        }

        // Shadow map for the current room. Includes character so the
        // skinned body casts a shadow into the world.
        self.record_shadow_pass_current(&mut enc, cur, cam);

        // Door dream-blend — alpha rises as the player approaches the
        // nearest door (peaks right at the door plane), then falls again
        // over BLEND_RADIUS. The actual world flip happens at peak alpha
        // so the cut is hidden inside the dissolve.
        let to_a = self.player.pos - PORTAL_A_POS;
        let to_b = self.player.pos - PORTAL_B_POS;
        let d_a = (to_a.x * to_a.x + to_a.z * to_a.z).sqrt();
        let d_b = (to_b.x * to_b.x + to_b.z * to_b.z).sqrt();
        let (nearest_door_d, nearest_door_idx) =
            if d_a <= d_b { (d_a, 0usize) } else { (d_b, 1usize) };
        let fade_alpha = (1.0 - nearest_door_d / BLEND_RADIUS).clamp(0.0, 1.0);

        self.record_main_pass(
            &mut enc,
            &frame_view,
            cur,
            cur_world,
            cam,
            view_proj,
            portal_a_visible,
            portal_b_visible,
            fade_alpha,
            nearest_door_idx,
        );

        self.queue.submit(Some(enc.finish()));
        frame.present();
        self.window.request_redraw();
    }

    /// Renders `dest` (destination room) into `self.portal.color_views[door_idx]`,
    /// preceded by a shadow pass for that room. `cam` and `view_proj` are
    /// the player's actual camera — the destination room is rendered as
    /// if the player were looking from the same viewpoint, which the
    /// portal magic-surface shader then samples per-pixel.
    fn record_portal_pass(
        &self,
        enc: &mut wgpu::CommandEncoder,
        door_idx: usize,
        dest: usize,
        cam: Vec3,
        view_proj: Mat4,
    ) {
        let dest_world = &self.worlds[dest];
        let dest_theme = room_theme(dest as u8);

        let portal_lvp = compute_light_view_proj(&dest_world.env, self.player.pos);
        let u_shadow_portal =
            self.build_uniforms(portal_lvp, &dest_world.env, cam, false, [0.0; 4]);
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
            pass.set_vertex_buffer(0, dest_world.vbuf.slice(..));
            pass.set_index_buffer(dest_world.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..dest_world.index_count, 0, 0..1);
        }

        // show_flies = true so the destination room's lighting includes
        // the player's firefly glow — same as how that room will look
        // once the player actually enters it, which makes the portal
        // preview match the post-teleport reality (otherwise the colour
        // shifts at the moment of crossing).
        let u_portal = self.build_uniforms(view_proj, &dest_world.env, cam, true, [0.0; 4]);
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u_portal));
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("portal-view"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.portal.color_views[door_idx],
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(dest_world.env.sky),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.portal.depth_views[door_idx],
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
        pass.set_vertex_buffer(0, dest_world.vbuf.slice(..));
        pass.set_index_buffer(dest_world.ibuf.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..dest_world.index_count, 0, 0..1);
        self.draw_room_feature(&mut pass, dest_theme, /*include_decals=*/ false);
        // No character / no fireflies in the portal view — they're with
        // the player in the current room.
    }

    /// Shadow map for the room the player is in. Drawn into the same
    /// `shadow_view` as the portal pre-pass — main pass reads whichever
    /// was rendered last, which is this one.
    fn record_shadow_pass_current(
        &self,
        enc: &mut wgpu::CommandEncoder,
        cur: &WorldGpu,
        cam: Vec3,
    ) {
        let light_view_proj = compute_light_view_proj(&cur.env, self.player.pos);
        let u_shadow = self.build_uniforms(light_view_proj, &cur.env, cam, true, [0.0; 4]);
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u_shadow));
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
        // Door is always present in either world — visible and casting
        // shadow even from the locked side; only the portal's magic
        // surface is gated by portal visibility.
        pass.set_vertex_buffer(0, self.portal.door_vbuf.slice(..));
        pass.set_index_buffer(self.portal.door_ibuf.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.portal.door_index_count, 0, 0..1);
        // Skinned character — depth-only skin shader.
        pass.set_pipeline(&self.skin_shadow_pipeline);
        pass.set_bind_group(0, &self.shadow_bg0, &[]);
        pass.set_bind_group(1, &self.character.bone_bind_group, &[]);
        pass.set_vertex_buffer(0, self.character.vbuf.slice(..));
        pass.set_index_buffer(self.character.ibuf.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.character.index_count, 0, 0..1);
    }

    /// Final pass — clears the swapchain to the sky colour, draws the
    /// current room, its signature feature, the portal magic surfaces
    /// (location), the dream-blend overlay, then the door frames,
    /// character and fireflies on top (so they don't fade with the
    /// room dissolve).
    #[allow(clippy::too_many_arguments)]
    fn record_main_pass(
        &self,
        enc: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        cur: &WorldGpu,
        cur_world: u8,
        cam: Vec3,
        view_proj: Mat4,
        portal_a_visible: bool,
        portal_b_visible: bool,
        fade_alpha: f32,
        nearest_door_idx: usize,
    ) {
        let u_main = self.build_uniforms(
            view_proj, &cur.env, cam, true, [0.0, 0.0, 0.0, fade_alpha]);
        let cur_theme = room_theme(cur_world);
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u_main));
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame_view,
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
        // Current room's signature feature (sand gets footprint decals;
        // water gets ripple displacement; grass blades bend toward the
        // player — all driven by the uniform array).
        self.draw_room_feature(&mut pass, cur_theme, /*include_decals=*/ true);
        // Portal magic surfaces — drawn before the dream-blend because
        // they're part of the LOCATION. The blend overlay tints them
        // along with everything else, then character + fireflies layer
        // on top crisp.
        let n = self.portal.indices_per_door;
        if portal_a_visible || portal_b_visible {
            pass.set_pipeline(&self.portal.pipeline);
            pass.set_bind_group(0, &self.main_bg0, &[]);
            pass.set_vertex_buffer(0, self.portal.vbuf.slice(..));
            pass.set_index_buffer(self.portal.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            if portal_a_visible {
                pass.set_bind_group(1, &self.portal.tex_bgs[0], &[]);
                pass.draw_indexed(0..n, 0, 0..1);
            }
            if portal_b_visible {
                pass.set_bind_group(1, &self.portal.tex_bgs[1], &[]);
                pass.draw_indexed(n..(2 * n), 0, 0..1);
            }
        }
        // Door dream-blend — the LOCATION dissolves between rooms; the
        // player, door frames, and fireflies below are drawn AFTER this
        // so they stay crisp through the transition.
        if fade_alpha > 0.0 {
            pass.set_pipeline(&self.fade.pipeline);
            pass.set_bind_group(0, &self.main_bg0, &[]);
            pass.set_bind_group(1, &self.portal.tex_bgs[nearest_door_idx], &[]);
            pass.set_vertex_buffer(0, self.fade.vbuf.slice(..));
            pass.draw(0..3, 0..1);
        }
        // Door frames — drawn AFTER the dream-blend so they never fade.
        // They're a permanent fixture in every room, not part of the
        // dissolving environment.
        pass.set_pipeline(&self.main_pipeline);
        pass.set_bind_group(0, &self.main_bg0, &[]);
        pass.set_bind_group(1, &self.main_bg1_shadow, &[]);
        pass.set_vertex_buffer(0, self.portal.door_vbuf.slice(..));
        pass.set_index_buffer(self.portal.door_ibuf.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.portal.door_index_count, 0, 0..1);
        // Skinned character — drawn after the dream-blend so it never
        // fades. The transition is about the room, not the player.
        if self.third_person {
            pass.set_pipeline(&self.skin_pipeline);
            pass.set_bind_group(0, &self.main_bg0, &[]);
            pass.set_bind_group(1, &self.main_bg1_shadow, &[]);
            pass.set_vertex_buffer(0, self.character.vbuf.slice(..));
            pass.set_index_buffer(self.character.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            for sm in &self.character.submeshes {
                pass.set_bind_group(2, &sm.bind_group, &[]);
                pass.draw_indexed(sm.index_start..sm.index_start + sm.index_count, 0, 0..1);
            }
        }
        // Fireflies — also drawn after the dream-blend.
        if self.firefly_mesh.count > 0 {
            pass.set_pipeline(&self.main_pipeline);
            pass.set_bind_group(0, &self.main_bg0, &[]);
            pass.set_bind_group(1, &self.main_bg1_shadow, &[]);
            pass.set_vertex_buffer(0, self.firefly_mesh.vbuf.slice(..));
            pass.set_index_buffer(self.firefly_mesh.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.firefly_mesh.count, 0, 0..1);
        }
    }

    /// Picks the right ground-feature pipeline for a room and draws it.
    /// When `include_decals` is true and the theme is Sand, the
    /// footprint decals layer on top of the sand patch.
    fn draw_room_feature<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        theme: RoomTheme,
        include_decals: bool,
    ) {
        match theme {
            RoomTheme::Sand => {
                pass.set_pipeline(&self.sand.pipeline);
                pass.set_bind_group(0, &self.main_bg0, &[]);
                pass.set_vertex_buffer(0, self.sand.vbuf.slice(..));
                pass.set_index_buffer(self.sand.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.sand.index_count, 0, 0..1);
                if include_decals && self.decal.index_count > 0 {
                    pass.set_pipeline(&self.decal.pipeline);
                    pass.set_bind_group(0, &self.main_bg0, &[]);
                    pass.set_vertex_buffer(0, self.decal.vbuf.slice(..));
                    pass.set_index_buffer(self.decal.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..self.decal.index_count, 0, 0..1);
                }
            }
            RoomTheme::Grass => {
                pass.set_pipeline(&self.grass.pipeline);
                pass.set_bind_group(0, &self.main_bg0, &[]);
                pass.set_vertex_buffer(0, self.grass.vbuf.slice(..));
                pass.set_index_buffer(self.grass.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.grass.index_count, 0, 0..1);
            }
            RoomTheme::Water => {
                pass.set_pipeline(&self.water.pipeline);
                pass.set_bind_group(0, &self.main_bg0, &[]);
                pass.set_vertex_buffer(0, self.water.vbuf.slice(..));
                pass.set_index_buffer(self.water.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.water.index_count, 0, 0..1);
            }
            RoomTheme::Free => {}
        }
    }
}

impl Game {
    /// Builds the per-pass `Uniforms` from `self` + the few values that
    /// actually vary between passes. `show_flies` is the only firefly
    /// flag — `false` masks them out for the portal pre-pass (the room
    /// being previewed shouldn't borrow the player's escort).
    fn build_uniforms(
        &self,
        view_proj: Mat4,
        env: &WorldEnv,
        cam: Vec3,
        show_flies: bool,
        fade: [f32; 4],
    ) -> Uniforms {
        let light_view_proj = compute_light_view_proj(env, self.player.pos);
        let mut fly_pos = [[0.0_f32; 4]; FIREFLY_COUNT];
        let w = if show_flies { 1.0 } else { 0.0 };
        for (i, f) in self.fireflies.iter().enumerate() {
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
            // .w = shadow strength — always 1 (shadowing on) for now.
            ambient_color: [env.ambient[0], env.ambient[1], env.ambient[2], 1.0],
            fly_pos,
            fly_color: [3.5, 5.0, 2.5, 1.0],
            player_pos: [self.player.pos.x, self.player.pos.y, self.player.pos.z, self.time],
            ripples: self.water.ripples_uniform,
            fade,
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Pipeline builders
//
// Each `build_*_pipeline` is self-contained: it compiles its shader,
// creates the pipeline layout from the bind-group layouts it needs,
// and returns the final `RenderPipeline`. Game::new just calls each
// one with the relevant BGLs — keeps the construction declarative.
// ─────────────────────────────────────────────────────────────────

const VERTEX_ATTRS: [wgpu::VertexAttribute; 4] = [
    wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 36, shader_location: 3, format: wgpu::VertexFormat::Float32 },
];

const SKIN_VERTEX_ATTRS: [wgpu::VertexAttribute; 7] = [
    wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 36, shader_location: 3, format: wgpu::VertexFormat::Float32 },
    wgpu::VertexAttribute { offset: 40, shader_location: 4, format: wgpu::VertexFormat::Uint32x4 },
    wgpu::VertexAttribute { offset: 56, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
    wgpu::VertexAttribute { offset: 72, shader_location: 6, format: wgpu::VertexFormat::Float32x2 },
];

fn make_shader(device: &wgpu::Device, label: &str, src: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(src.into()),
    })
}

fn make_layout(
    device: &wgpu::Device,
    label: &str,
    bgls: &[&wgpu::BindGroupLayout],
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: bgls,
        push_constant_ranges: &[],
    })
}

fn depth_opaque() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: Default::default(),
        bias: Default::default(),
    }
}

fn build_main_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
    bgl1_shadow: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "main-shader", MAIN_SHADER);
    let layout = make_layout(device, "main-pl", &[bgl0, bgl1_shadow]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("main-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &VERTEX_ATTRS,
            }],
        },
        primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
        depth_stencil: Some(depth_opaque()),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
}

fn build_shadow_pipeline(
    device: &wgpu::Device,
    bgl0: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "shadow-shader", SHADOW_SHADER);
    let layout = make_layout(device, "shadow-pl", &[bgl0]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shadow-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &VERTEX_ATTRS,
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
    })
}

fn build_portal_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
    bgl1_portal: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "portal-shader", PORTAL_SHADER);
    let layout = make_layout(device, "portal-pl", &[bgl0, bgl1_portal]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("portal-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &VERTEX_ATTRS,
            }],
        },
        // No culling on the portal quad — visible from both sides.
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(depth_opaque()),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
}

fn build_skin_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
    bgl1_shadow: &wgpu::BindGroupLayout,
    bone_bgl: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "skin-shader", SKIN_SHADER);
    let layout = make_layout(device, "skin-pl", &[bgl0, bgl1_shadow, bone_bgl]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("skin-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SkinVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &SKIN_VERTEX_ATTRS,
            }],
        },
        primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
        depth_stencil: Some(depth_opaque()),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
}

fn build_skin_shadow_pipeline(
    device: &wgpu::Device,
    bgl0: &wgpu::BindGroupLayout,
    bone_bgl: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "skin-shadow-shader", SKIN_SHADOW_SHADER);
    let layout = make_layout(device, "skin-shadow-pl", &[bgl0, bone_bgl]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("skin-shadow-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SkinVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &SKIN_VERTEX_ATTRS,
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
    })
}

/// Bind-group layout for the skinned character's per-mesh resources
/// (bone storage + diffuse texture + sampler).
fn build_bone_bgl(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("char-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn build_grass_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "grass-shader", GRASS_SHADER);
    let layout = make_layout(device, "grass-pl", &[bgl0]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("grass-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GrassVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32 },
                    wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 24, shader_location: 3, format: wgpu::VertexFormat::Float32 },
                ],
            }],
        },
        // No culling — blades are paper-thin and viewable from both sides.
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(depth_opaque()),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
}

fn build_water_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "water-shader", WATER_SHADER);
    let layout = make_layout(device, "water-pl", &[bgl0]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("water-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<PosVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(depth_opaque()),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
}

fn build_sand_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "sand-shader", SAND_SHADER);
    let layout = make_layout(device, "sand-pl", &[bgl0]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sand-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<PosVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(depth_opaque()),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
}

fn build_decal_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "decal-shader", DECAL_SHADER);
    let layout = make_layout(device, "decal-pl", &[bgl0]);
    // Decals lie just above the sand — read depth so they don't draw
    // through other geometry, but don't write back (so successive
    // decals can blend without z-fighting).
    let depth = wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: Default::default(),
        bias: wgpu::DepthBiasState { constant: -2, slope_scale: -1.0, clamp: 0.0 },
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("decal-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<DecalVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 20, shader_location: 2, format: wgpu::VertexFormat::Float32 },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(depth),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
}

fn build_fade_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
    bgl1_portal: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "fade-shader", FADE_SHADER);
    let layout = make_layout(device, "fade-pl", &[bgl0, bgl1_portal]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fade-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<PosVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        // Skip depth — the overlay sits in NDC and should always paint over the scene.
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
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
// SECTION 7 — winit ApplicationHandler
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
