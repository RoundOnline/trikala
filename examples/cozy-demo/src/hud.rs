//! The HUD — flat health bars drawn over the scene. Vertices are
//! emitted straight in clip space (-1..1); see hud.wgsl. Reuses the
//! scene's vertex type, so no extra buffer or GPU resources.

use crate::geometry::Vertex;
use crate::world::tile_height;
use glam::Vec3;

/// Emit one flat rectangle in clip space with an opacity — hud.wgsl
/// reads the alpha from normal.x.
fn push_rect_a(out: &mut Vec<Vertex>, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 3], alpha: f32) {
    let n = [alpha, 0.0, 0.0];
    let corner = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    for i in [0usize, 1, 2, 0, 2, 3] {
        out.push(Vertex { pos: [corner[i][0], corner[i][1], 0.0], normal: n, color });
    }
}

/// An opaque flat rectangle in clip space.
fn push_rect(out: &mut Vec<Vertex>, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 3]) {
    push_rect_a(out, x0, y0, x1, y1, color, 1.0);
}

/// One health bar: a dark backing, then a fill scaled by `frac` (0..1).
fn push_bar(out: &mut Vec<Vertex>, x0: f32, y0: f32, x1: f32, y1: f32, frac: f32, fill: [f32; 3]) {
    push_rect(out, x0, y0, x1, y1, [0.06, 0.05, 0.08]);
    let pad = 0.008;
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        let inner = (x1 - x0) - pad * 2.0;
        push_rect(out, x0 + pad, y0 + pad, x0 + pad + inner * f, y1 - pad, fill);
    }
}

/// Append the HUD: the player's health bottom-left, and the boss's
/// health top-centre while it is awake.
pub fn push_hud(out: &mut Vec<Vertex>, player_hp: f32, boss_hp: f32, boss_awake: bool, loot: u32) {
    push_bar(out, -0.96, -0.95, -0.54, -0.88, player_hp, [0.46, 0.79, 0.41]);
    if boss_awake {
        push_bar(out, -0.40, 0.86, 0.40, 0.93, boss_hp, [0.82, 0.36, 0.26]);
    }
    // collected loot — a row of gold pips just above the health bar
    for i in 0..loot.min(16) {
        let x = -0.96 + i as f32 * 0.046;
        push_rect(out, x, -0.855, x + 0.034, -0.815, [0.95, 0.82, 0.32]);
    }
}

/// A compact, semi-transparent minimap in the top-right — the terrain
/// shaded by elevation, a blip for everything in range, and the player
/// fixed at the centre. Each blip is a world position, colour and size.
pub fn push_minimap(out: &mut Vec<Vertex>, player: Vec3, blips: &[(Vec3, [f32; 3], f32)]) {
    let (cx, cy) = (0.83, 0.72); // panel centre — tucked into the corner
    let hs = 0.145; // panel half-size
    let range = 30.0; // world units from the centre to the panel edge

    // translucent backing
    push_rect_a(out, cx - hs, cy - hs, cx + hs, cy + hs, [0.07, 0.08, 0.10], 0.42);

    // terrain shaded by height — bright hills, dark hollows
    let n = 16;
    let cell = 2.0 * hs / n as f32;
    for j in 0..n {
        for i in 0..n {
            let x0 = cx - hs + i as f32 * cell;
            let y0 = cy - hs + j as f32 * cell;
            let wx = player.x + (x0 + cell * 0.5 - cx) / hs * range;
            let wz = player.z - (y0 + cell * 0.5 - cy) / hs * range;
            let h = tile_height(wx.floor() as i32, wz.floor() as i32);
            let t = (h / 9.0).clamp(0.0, 1.0);
            let col = [0.15 + 0.40 * t, 0.30 + 0.30 * t, 0.19 + 0.08 * t];
            push_rect_a(out, x0, y0, x0 + cell, y0 + cell, col, 0.7);
        }
    }

    // blips for points of interest, drawn solid on top
    for &(p, col, size) in blips {
        let rx = (p.x - player.x) / range;
        let rz = (p.z - player.z) / range;
        if rx.abs() < 0.94 && rz.abs() < 0.94 {
            let (bx, by) = (cx + rx * hs, cy - rz * hs);
            push_rect(out, bx - size, by - size, bx + size, by + size, col);
        }
    }
    // the player, always dead centre
    push_rect(out, cx - 0.016, cy - 0.016, cx + 0.016, cy + 0.016, [0.97, 0.97, 1.0]);
}
