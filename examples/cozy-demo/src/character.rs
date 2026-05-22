//! The player character: its low-poly box mesh, the walk cycle, and
//! the two sword attacks.

use crate::geometry::{flash_color, push_box, push_rotated, Vertex};
use crate::weapon::Weapon;
use glam::{Mat4, Vec3};

pub const CHARGE_MIN: f32 = 0.25; // hold the attack this long for a heavy slash
const QUICK_DUR: f32 = 0.32;
const HEAVY_DUR: f32 = 0.58;

const SKIN: [f32; 3] = [0.86, 0.66, 0.52];
const SHIRT: [f32; 3] = [0.45, 0.33, 0.52];

/// Which of the two slashes is playing.
#[derive(Copy, Clone)]
pub enum Slash {
    Quick,
    Heavy,
}

impl Slash {
    pub fn duration(self) -> f32 {
        match self {
            Slash::Quick => QUICK_DUR,
            Slash::Heavy => HEAVY_DUR,
        }
    }
}

/// Sword-arm pitch keyframes (time 0..1, x-angle). The short, big-angle
/// strike segment plus the flat impact-hold segment give the weight.
const QUICK_X: [(f32, f32); 5] =
    [(0.00, 0.0), (0.20, 2.3), (0.40, -1.3), (0.56, -1.35), (1.00, 0.10)];
const HEAVY_X: [(f32, f32); 4] =
    [(0.00, 2.7), (0.30, -1.5), (0.52, -1.55), (1.00, 0.20)];
/// The heavy slash also sweeps a little sideways for a wide arc.
const HEAVY_YAW: [(f32, f32); 3] = [(0.00, 0.5), (0.32, -0.5), (1.00, -0.15)];

/// Sample a keyframe track at time `p` with smoothstep easing.
pub fn sample(keys: &[(f32, f32)], p: f32) -> f32 {
    if p <= keys[0].0 {
        return keys[0].1;
    }
    for w in keys.windows(2) {
        let (t0, v0) = w[0];
        let (t1, v1) = w[1];
        if p <= t1 {
            let l = ((p - t0) / (t1 - t0)).clamp(0.0, 1.0);
            return v0 + (v1 - v0) * (l * l * (3.0 - 2.0 * l));
        }
    }
    keys[keys.len() - 1].1
}

/// Rotation of the sword arm this frame — walk swing, charge pose, or
/// one of the two slash animations.
pub fn sword_arm(anim: Option<(Slash, f32)>, charging: bool, charge: f32, phase: f32, blend: f32) -> Mat4 {
    match anim {
        Some((Slash::Heavy, t)) => {
            let p = (t / HEAVY_DUR).clamp(0.0, 1.0);
            Mat4::from_rotation_y(sample(&HEAVY_YAW, p)) * Mat4::from_rotation_x(sample(&HEAVY_X, p))
        }
        Some((Slash::Quick, t)) => {
            Mat4::from_rotation_x(sample(&QUICK_X, (t / QUICK_DUR).clamp(0.0, 1.0)))
        }
        // winding up — sword raised overhead, lifts a touch with charge
        None if charging && charge > 0.08 => Mat4::from_rotation_x(2.6 + charge.min(1.0) * 0.3),
        None => Mat4::from_rotation_x(phase.sin() * blend * 0.45),
    }
}

/// Forward body lean — peaks mid-swing to sell the weight of a hit.
pub fn body_lean(anim: Option<(Slash, f32)>) -> f32 {
    match anim {
        Some((kind, t)) => {
            let p = (t / kind.duration()).clamp(0.0, 1.0);
            (p * std::f32::consts::PI).sin() * if matches!(kind, Slash::Heavy) { 0.30 } else { 0.16 }
        }
        None => 0.0,
    }
}

/// Everything `push_character` needs to pose the figure this frame.
pub struct CharacterPose {
    pub pos: Vec3,
    pub facing: f32,
    pub phase: f32,
    pub blend: f32,
    pub sword_arm: Mat4,
    pub lean: f32,
    pub flash: f32,
    pub weapon: Weapon,
}

/// One arm — shoulder, forearm, hand (and a sword if `with_sword`) —
/// rotated about the shoulder.
fn push_arm(local: &mut Vec<Vertex>, x: f32, rot: Mat4, held: Option<Weapon>) {
    let mut arm: Vec<Vertex> = Vec::new();
    push_box(&mut arm, Vec3::new(x, 1.05, 0.0), Vec3::new(0.06, 0.09, 0.08), SHIRT);
    push_box(&mut arm, Vec3::new(x, 0.83, 0.0), Vec3::new(0.052, 0.14, 0.065), SKIN);
    push_box(&mut arm, Vec3::new(x, 0.64, 0.0), Vec3::new(0.06, 0.06, 0.07), SKIN);
    let wood = [0.40, 0.28, 0.18];
    let metal = [0.74, 0.76, 0.80];
    match held {
        Some(Weapon::Sword) => {
            push_box(&mut arm, Vec3::new(x, 0.585, 0.0), Vec3::new(0.034, 0.085, 0.034), [0.34, 0.23, 0.15]);
            push_box(&mut arm, Vec3::new(x, 0.49, 0.0), Vec3::new(0.135, 0.025, 0.05), [0.22, 0.19, 0.16]);
            push_box(&mut arm, Vec3::new(x, 0.27, 0.0), Vec3::new(0.045, 0.20, 0.022), metal);
        }
        Some(Weapon::Spear) => {
            push_box(&mut arm, Vec3::new(x, 0.56, 0.0), Vec3::new(0.03, 0.09, 0.03), wood);
            push_box(&mut arm, Vec3::new(x, 0.20, 0.0), Vec3::new(0.022, 0.40, 0.022), wood);
            push_box(&mut arm, Vec3::new(x, -0.27, 0.0), Vec3::new(0.04, 0.10, 0.035), metal);
        }
        Some(Weapon::Bow) => {
            push_box(&mut arm, Vec3::new(x, 0.49, 0.0), Vec3::new(0.03, 0.10, 0.03), wood);
            push_box(&mut arm, Vec3::new(x, 0.70, 0.04), Vec3::new(0.022, 0.13, 0.028), wood);
            push_box(&mut arm, Vec3::new(x, 0.28, 0.04), Vec3::new(0.022, 0.13, 0.028), wood);
            push_box(&mut arm, Vec3::new(x, 0.49, 0.075), Vec3::new(0.006, 0.32, 0.006), [0.85, 0.83, 0.78]);
        }
        None => {}
    }
    push_rotated(local, &arm, Vec3::new(x, 1.14, 0.0), rot);
}

/// Emit the posed character into `out`. Built in local space (feet at
/// origin, facing +Z), then leaned, turned to face, and placed.
pub fn push_character(out: &mut Vec<Vertex>, pose: &CharacterPose) {
    let pants = [0.40, 0.45, 0.56];
    let shoe = [0.13, 0.12, 0.14];
    let hair = [0.26, 0.19, 0.14];

    let mut local: Vec<Vertex> = Vec::new();

    // body — does not swing
    push_box(&mut local, Vec3::new(0.0, 0.66, 0.0), Vec3::new(0.16, 0.09, 0.11), pants);
    push_box(&mut local, Vec3::new(0.0, 0.94, 0.0), Vec3::new(0.19, 0.20, 0.12), SHIRT);
    push_box(&mut local, Vec3::new(0.0, 1.18, 0.0), Vec3::new(0.06, 0.05, 0.06), SKIN);
    push_box(&mut local, Vec3::new(0.0, 1.39, 0.0), Vec3::new(0.15, 0.16, 0.13), SKIN);
    push_box(&mut local, Vec3::new(0.0, 1.57, 0.0), Vec3::new(0.16, 0.05, 0.135), hair);

    let swing = pose.phase.sin() * pose.blend;

    // legs (+ shoes) swing about the hips, left and right opposite
    for (side, dir) in [(-0.11f32, 1.0f32), (0.11, -1.0)] {
        let mut leg: Vec<Vertex> = Vec::new();
        push_box(&mut leg, Vec3::new(side, 0.34, 0.0), Vec3::new(0.085, 0.24, 0.085), pants);
        push_box(&mut leg, Vec3::new(side, 0.05, 0.02), Vec3::new(0.11, 0.05, 0.13), shoe);
        push_rotated(&mut local, &leg, Vec3::new(side, 0.58, 0.0), Mat4::from_rotation_x(swing * 0.55 * dir));
    }

    push_arm(&mut local, -0.25, Mat4::from_rotation_x(swing * -0.45), None);
    push_arm(&mut local, 0.25, pose.sword_arm, Some(pose.weapon));

    // forward lean, then turn to face the heading, then place in world
    let pivot = Vec3::new(0.0, 0.45, 0.0);
    let model = Mat4::from_translation(pose.pos)
        * Mat4::from_rotation_y(pose.facing)
        * Mat4::from_translation(pivot)
        * Mat4::from_rotation_x(pose.lean)
        * Mat4::from_translation(-pivot);
    let nrot = Mat4::from_rotation_y(pose.facing) * Mat4::from_rotation_x(pose.lean);
    for v in &local {
        out.push(Vertex {
            pos: model.transform_point3(Vec3::from(v.pos)).to_array(),
            normal: nrot.transform_vector3(Vec3::from(v.normal)).to_array(),
            color: flash_color(v.color, pose.flash),
        });
    }
}
