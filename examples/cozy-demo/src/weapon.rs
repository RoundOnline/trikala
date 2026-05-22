//! The three weapons the player can carry, swapped by walking onto a
//! pickup. A weapon decides what the character holds (see character.rs)
//! and how far the attack reaches.

use crate::geometry::{push_box, Vertex};
use glam::{Mat4, Vec3};

#[derive(Copy, Clone, PartialEq)]
pub enum Weapon {
    Sword,
    Spear,
    Bow,
}

impl Weapon {
    /// How far the weapon's strike reaches past the player, toward an
    /// enemy's surface (added to the enemy's own radius when hit-testing).
    pub fn reach(self) -> f32 {
        match self {
            Weapon::Sword => 1.3,
            Weapon::Spear => 2.4,
            Weapon::Bow => 1.6,
        }
    }
}

/// Append a small floating model of `weapon` at `center`, spun by
/// `spin` radians — the world pickup the player walks onto to equip it.
pub fn push_weapon_icon(out: &mut Vec<Vertex>, weapon: Weapon, center: Vec3, spin: f32) {
    let wood = [0.40, 0.28, 0.18];
    let metal = [0.74, 0.76, 0.80];
    let mut m: Vec<Vertex> = Vec::new();
    match weapon {
        Weapon::Sword => {
            push_box(&mut m, Vec3::new(0.0, -0.17, 0.0), Vec3::new(0.03, 0.07, 0.03), wood);
            push_box(&mut m, Vec3::new(0.0, -0.08, 0.0), Vec3::new(0.11, 0.022, 0.04), wood);
            push_box(&mut m, Vec3::new(0.0, 0.13, 0.0), Vec3::new(0.04, 0.21, 0.02), metal);
        }
        Weapon::Spear => {
            push_box(&mut m, Vec3::new(0.0, -0.04, 0.0), Vec3::new(0.022, 0.34, 0.022), wood);
            push_box(&mut m, Vec3::new(0.0, 0.36, 0.0), Vec3::new(0.045, 0.11, 0.04), metal);
        }
        Weapon::Bow => {
            push_box(&mut m, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.03, 0.11, 0.03), wood);
            push_box(&mut m, Vec3::new(0.0, 0.22, 0.06), Vec3::new(0.024, 0.14, 0.03), wood);
            push_box(&mut m, Vec3::new(0.0, -0.22, 0.06), Vec3::new(0.024, 0.14, 0.03), wood);
            push_box(&mut m, Vec3::new(0.0, 0.0, 0.10), Vec3::new(0.007, 0.34, 0.007), [0.86, 0.84, 0.78]);
        }
    }
    let model = Mat4::from_translation(center)
        * Mat4::from_rotation_y(spin)
        * Mat4::from_rotation_x(0.45);
    for v in &m {
        out.push(Vertex {
            pos: model.transform_point3(Vec3::from(v.pos)).to_array(),
            normal: model.transform_vector3(Vec3::from(v.normal)).to_array(),
            color: v.color,
        });
    }
}
