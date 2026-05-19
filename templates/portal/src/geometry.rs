use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

// ── Vertex types ──

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub emissive: f32,
}

/// Grass blade vertex — `pos` is the bind-pose world position, `base_xz`
/// is the blade's footprint on the ground (so every vertex of one
/// blade shares this) for the player-distance bend, and `phase` is a
/// per-blade randomiser for the wind sway.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GrassVertex {
    pub pos: [f32; 3],
    pub tip_factor: f32,
    pub base_xz: [f32; 2],
    pub phase: f32,
    pub _pad: f32,
}

/// Water / sand / fade vertex — just position; the vertex shader does
/// the displacement and shading procedurally.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PosVertex {
    pub pos: [f32; 3],
}

/// Footprint decal vertex — world position + UV inside the quad +
/// normalized age (0 fresh, 1 expired) for the fade.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct DecalVertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub age01: f32,
}

// ── Geometry tuning ──

pub const PORTAL_A_POS: Vec3 = Vec3::new(3.5, 1.30, -5.0);
pub const PORTAL_B_POS: Vec3 = Vec3::new(-3.5, 1.30, -5.0);
/// Half-width is wider than the character (~1.04 m half-width after
/// the 1.15× model scale) so the posts don't clip the player's arms
/// when the camera looks at them through the doorway right after a
/// teleport.
pub const PORTAL_HALF_W: f32 = 1.20;
pub const PORTAL_HALF_H: f32 = 1.30;
/// Half-thickness of the vertical door posts.
pub const PORTAL_POST_T: f32 = 0.08;
/// Half-depth (z) of the door frame.
pub const PORTAL_DEPTH_T: f32 = 0.06;

pub const WATER_MIN: Vec3 = Vec3::new(-9.0, 0.0, -3.5);
pub const WATER_MAX: Vec3 = Vec3::new(9.0, 0.0, 5.0);
pub const WATER_LEVEL: f32 = 0.04;
pub const WATER_GRID: u32 = 56;

pub const GRASS_MIN: Vec3 = Vec3::new(-9.0, 0.0, -12.0);
pub const GRASS_MAX: Vec3 = Vec3::new(9.0, 0.0, -5.5);
pub const GRASS_DENSITY: usize = 4500;

pub const SAND_MIN: Vec3 = Vec3::new(-9.0, 0.0, -12.0);
pub const SAND_MAX: Vec3 = Vec3::new(9.0, 0.0, 8.0);
pub const SAND_Y: f32 = 0.005;

pub const FIREFLY_COUNT: usize = 4;

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

// ── World + theme ──

/// Per-room lighting, sky, and shadow-camera origin. Looked up each
/// frame from the current world's index into `worlds[]`.
#[derive(Copy, Clone)]
pub struct WorldEnv {
    pub lamp_pos: Vec3,
    pub lamp_color: [f32; 4],
    pub moon_dir: Vec3,
    pub moon_color: [f32; 4],
    pub ambient: [f32; 3],
    pub sky: wgpu::Color,
    /// Origin of the directional-light camera for shadow mapping.
    pub light_origin: Vec3,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum RoomTheme { Water = 0, Grass = 1, Sand = 2, Free = 3 }

pub fn room_theme(world: u8) -> RoomTheme {
    match world & 0b11 {
        0 => RoomTheme::Water,
        1 => RoomTheme::Grass,
        2 => RoomTheme::Sand,
        _ => RoomTheme::Free,
    }
}

// ── Geometry primitives ──

pub fn push_box(
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
                pos: world.to_array(),
                normal: world_n.to_array(),
                color: color.to_array(),
                emissive,
            });
        }
        let face_base = base + (i as u32) * 4;
        indices.extend_from_slice(&[
            face_base, face_base + 1, face_base + 2,
            face_base, face_base + 2, face_base + 3,
        ]);
    }
}

pub fn push_cube(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    center: Vec3,
    size: Vec3,
    color: Vec3,
    emissive: f32,
) {
    let transform = Mat4::from_translation(center);
    push_box(verts, indices, transform, size * 0.5, color, emissive);
}

pub fn push_plane(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, center: Vec3, half: f32, color: Vec3) {
    let base = verts.len() as u32;
    let corners = [
        Vec3::new(center.x - half, center.y, center.z - half),
        Vec3::new(center.x + half, center.y, center.z - half),
        Vec3::new(center.x + half, center.y, center.z + half),
        Vec3::new(center.x - half, center.y, center.z + half),
    ];
    for c in corners {
        verts.push(Vertex {
            pos: c.to_array(),
            normal: [0.0, 1.0, 0.0],
            color: color.to_array(),
            emissive: 0.0,
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

pub fn push_tree(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, x: f32, z: f32, scale: f32, canopy: Vec3) {
    let trunk = Vec3::new(0.20, 0.55, 0.18);
    push_cube(verts, indices,
        Vec3::new(x, 0.6 * scale, z),
        Vec3::new(0.30 * scale, 1.2 * scale, 0.30 * scale),
        trunk, 0.0);
    push_cube(verts, indices,
        Vec3::new(x, 1.6 * scale, z),
        Vec3::new(1.40 * scale, 1.20 * scale, 1.40 * scale),
        canopy, 0.0);
}

pub fn push_doraemon_door(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, center: Vec3) {
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

// ── Whole-room builder ──

/// Each themed room is a minimal scene — a flat floor in the theme's
/// tint, a single bright sun/moon cube, and the global tree ring for
/// depth. The room's defining feature (water / grass / sand) is drawn
/// separately at render time so this builder doesn't even need to
/// know which feature is involved.
pub fn build_room(theme: RoomTheme) -> (Vec<Vertex>, Vec<u32>, WorldEnv) {
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

    push_plane(&mut v, &mut i, Vec3::ZERO, 60.0, floor_col);
    push_cube(&mut v, &mut i, sun_pos, Vec3::new(3.5, 3.5, 3.5), sun_col, 1.0);

    for &(x, z, s) in &LANDSCAPE_TREES {
        push_tree(&mut v, &mut i, x, z, s, tree_canopy);
    }

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
pub fn build_door_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    push_doraemon_door(&mut v, &mut i, PORTAL_A_POS);
    push_doraemon_door(&mut v, &mut i, PORTAL_B_POS);
    (v, i)
}

// ── Fireflies ──

/// Each firefly carries its own position and velocity so it drifts
/// independently. A soft spring pulls them toward the player (with
/// a minimum stand-off so they don't all collapse onto the head),
/// plus per-firefly sin-noise creates organic wandering. The result
/// reads as four little creatures with minds of their own that just
/// happen to like staying near you.
#[derive(Copy, Clone)]
pub struct Firefly {
    pub pos: Vec3,
    pub vel: Vec3,
    pub phase: [f32; 3],
}

pub fn step_fireflies(flies: &mut [Firefly; FIREFLY_COUNT], player_pos: Vec3, dt: f32, t: f32) {
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

pub fn build_fireflies(positions: &[Vec3]) -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    let glow = Vec3::new(3.5, 5.0, 2.5);
    for &pos in positions {
        push_cube(&mut v, &mut i, pos, Vec3::new(0.07, 0.07, 0.07), glow, 1.0);
    }
    (v, i)
}

// ── Feature mesh builders ──

/// Scatter `density` grass blades across the rectangle `[min..max]`
/// (xz). Each blade is a triangle (two base verts + one tip) with
/// a deterministic pseudo-random position, height and wind phase.
pub fn build_grass(min: Vec3, max: Vec3, density: usize) -> (Vec<GrassVertex>, Vec<u32>) {
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
        let h = 0.35 + r3 * 0.30;
        let w = 0.025 + r4 * 0.020;
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
pub fn build_water(min: Vec3, max: Vec3, y: f32, grid: u32) -> (Vec<PosVertex>, Vec<u32>) {
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
pub fn build_sand(min: Vec3, max: Vec3, y: f32) -> (Vec<PosVertex>, Vec<u32>) {
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
pub fn in_patch(x: f32, z: f32, min: Vec3, max: Vec3) -> bool {
    x >= min.x && x <= max.x && z >= min.z && z <= max.z
}

/// Build the footprint-decal mesh from the active footprint queue.
/// Each entry is one short quad oriented along the body yaw it was
/// stamped at; `age01` drives the fragment shader's fade.
pub fn build_decals(
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
        let cy_ = pos.y + 0.012;
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

/// The "magic surface" of one door — a quad inside the frame, facing
/// +z. Renderer draws each door's quad with its own portal texture
/// bound so each window into the destination room samples correctly.
pub fn build_portal_quad(center: Vec3) -> (Vec<Vertex>, Vec<u32>) {
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
