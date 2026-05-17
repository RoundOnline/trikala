//! Skinned glTF character — loads a rigged humanoid (CesiumMan as
//! the bundled default; swap the .glb with a Mixamo download to
//! get more animation clips) and produces:
//!   - a vertex buffer of `SkinVertex` (pos/normal/color +
//!     joint indices + weights),
//!   - an index buffer,
//!   - a storage buffer of per-joint world matrices updated each
//!     frame to drive vertex skinning in the shader.
//!
//! The animation library holds every clip from the glTF, keyed by
//! name. Callers ask `update()` to play one or two clips at given
//! weights — cross-fading between states (idle ↔ walk ↔ jump ↔
//! attack) is done at the caller side and expressed as two
//! weighted samples here.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SkinVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub emissive: f32,
    pub joints: [u32; 4],
    pub weights: [f32; 4],
}

#[derive(Clone)]
enum AnimValues {
    Translations(Vec<Vec3>),
    Rotations(Vec<Quat>),
    Scales(Vec<Vec3>),
}

#[derive(Clone)]
struct AnimChannel {
    target_node: usize,
    times: Vec<f32>,
    values: AnimValues,
}

#[derive(Clone)]
struct AnimClip {
    duration: f32,
    channels: Vec<AnimChannel>,
}

pub struct Character {
    pub vbuf: wgpu::Buffer,
    pub ibuf: wgpu::Buffer,
    pub index_count: u32,
    pub bone_buffer: wgpu::Buffer,
    pub bone_bind_group: wgpu::BindGroup,

    node_count: usize,
    node_parents: Vec<Option<usize>>,
    node_base_trs: Vec<(Vec3, Quat, Vec3)>,
    joint_node_indices: Vec<usize>,
    inverse_bind_matrices: Vec<Mat4>,

    clips: HashMap<String, AnimClip>,
    /// The name of whichever clip the renderer should use for
    /// "walking". Resolved at load time: a clip literally called
    /// "Walking" wins; otherwise we fall back to the first clip in
    /// the file (so single-clip assets like CesiumMan still work
    /// transparently).
    walk_clip: Option<String>,

    pub model_pre_transform: Mat4,
}

impl Character {
    pub fn load(
        device: &wgpu::Device,
        path: &str,
        bone_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let (doc, buffers, _) = gltf::import(path).expect("failed to load glb");

        // Find the first node that owns both a mesh and a skin.
        // Joint indices in a primitive's JOINTS_0 are relative to
        // its parent node's skin, so we have to pin the loader to a
        // single (node, skin) pair to keep them consistent. Then we
        // concatenate every primitive in that one mesh — RobotExpressive
        // is split into several primitives (body, joints, etc.) but
        // they all share the same skeleton.
        let skinned_node = doc
            .nodes()
            .find(|n| n.skin().is_some() && n.mesh().is_some())
            .expect("glTF has no skinned mesh node");
        let mesh = skinned_node.mesh().unwrap();
        let skin = skinned_node.skin().unwrap();

        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut joints_raw: Vec<[u16; 4]> = Vec::new();
        let mut weights_raw: Vec<[f32; 4]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|b| Some(&buffers[b.index()]));
            let prim_joints = match reader.read_joints(0) {
                Some(j) => j,
                None => continue,
            };
            let prim_weights = match reader.read_weights(0) {
                Some(w) => w,
                None => continue,
            };
            let prim_positions: Vec<[f32; 3]> = match reader.read_positions() {
                Some(it) => it.collect(),
                None => continue,
            };
            let prim_normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|it| it.collect::<Vec<[f32; 3]>>())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; prim_positions.len()]);
            let prim_indices: Vec<u32> = reader
                .read_indices()
                .map(|it| it.into_u32().collect())
                .unwrap_or_else(|| (0..prim_positions.len() as u32).collect());

            let base = positions.len() as u32;
            positions.extend(prim_positions);
            normals.extend(prim_normals);
            joints_raw.extend(prim_joints.into_u16());
            weights_raw.extend(prim_weights.into_f32());
            indices.extend(prim_indices.iter().map(|i| i + base));
        }
        assert!(!positions.is_empty(), "no skinned primitives in glTF");

        let vertices: Vec<SkinVertex> = (0..positions.len())
            .map(|i| SkinVertex {
                pos: positions[i],
                normal: normals[i],
                color: [0.85, 0.72, 0.58],
                emissive: 0.0,
                joints: [
                    joints_raw[i][0] as u32,
                    joints_raw[i][1] as u32,
                    joints_raw[i][2] as u32,
                    joints_raw[i][3] as u32,
                ],
                weights: weights_raw[i],
            })
            .collect();

        let joint_node_indices: Vec<usize> = skin.joints().map(|n| n.index()).collect();
        let bone_count = joint_node_indices.len();
        let ibm_reader = skin.reader(|b| Some(&buffers[b.index()]));
        let inverse_bind_matrices: Vec<Mat4> = ibm_reader
            .read_inverse_bind_matrices()
            .map(|it| it.map(|m| Mat4::from_cols_array_2d(&m)).collect())
            .unwrap_or_else(|| vec![Mat4::IDENTITY; bone_count]);

        let node_count = doc.nodes().count();
        let mut node_parents: Vec<Option<usize>> = vec![None; node_count];
        for node in doc.nodes() {
            for child in node.children() {
                node_parents[child.index()] = Some(node.index());
            }
        }
        let mut node_base_trs = vec![(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE); node_count];
        for node in doc.nodes() {
            let (t, r, s) = node.transform().decomposed();
            node_base_trs[node.index()] = (
                Vec3::from_array(t),
                Quat::from_xyzw(r[0], r[1], r[2], r[3]),
                Vec3::from_array(s),
            );
        }

        // Load every clip in the file, keyed by name.
        let mut clips: HashMap<String, AnimClip> = HashMap::new();
        let mut clip_order: Vec<String> = Vec::new();
        for (i, animation) in doc.animations().enumerate() {
            let name = animation
                .name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("anim_{}", i));
            let mut channels = Vec::new();
            let mut duration = 0.0_f32;
            for channel in animation.channels() {
                let target_node = channel.target().node().index();
                let reader = channel.reader(|b| Some(&buffers[b.index()]));
                let times: Vec<f32> = match reader.read_inputs() {
                    Some(it) => it.collect(),
                    None => continue,
                };
                if let Some(&last) = times.last() {
                    duration = duration.max(last);
                }
                let outputs = match reader.read_outputs() {
                    Some(o) => o,
                    None => continue,
                };
                let values = match outputs {
                    gltf::animation::util::ReadOutputs::Translations(iter) => {
                        AnimValues::Translations(iter.map(Vec3::from_array).collect())
                    }
                    gltf::animation::util::ReadOutputs::Rotations(rotations) => AnimValues::Rotations(
                        rotations
                            .into_f32()
                            .map(|r| Quat::from_xyzw(r[0], r[1], r[2], r[3]))
                            .collect(),
                    ),
                    gltf::animation::util::ReadOutputs::Scales(iter) => {
                        AnimValues::Scales(iter.map(Vec3::from_array).collect())
                    }
                    _ => continue,
                };
                channels.push(AnimChannel { target_node, times, values });
            }
            if duration <= 0.0 {
                duration = 1.0;
            }
            clip_order.push(name.clone());
            clips.insert(name, AnimClip { duration, channels });
        }

        // Resolve the walking clip — try common variants used by
        // Mixamo / three.js / CesiumMan assets, then fall back to
        // the first clip in the file.
        let walk_clip = ["Walking", "Walk", "walk", "walking"]
            .iter()
            .find(|n| clips.contains_key(**n))
            .map(|n| n.to_string())
            .or_else(|| clip_order.first().cloned());

        eprintln!(
            "[character] loaded {} joints, {} clips: {:?}",
            bone_count,
            clips.len(),
            clip_order
        );
        if let Some(w) = walk_clip.as_deref() {
            eprintln!("[character] walk clip resolved to: {}", w);
        }

        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("char-v"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("char-i"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let bone_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("char-bones"),
            size: (bone_count.max(1) * std::mem::size_of::<[[f32; 4]; 4]>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bone_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("char-bone-bg"),
            layout: bone_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bone_buffer.as_entire_binding(),
            }],
        });

        // CesiumMan ships facing +Z; our camera looks toward -Z at
        // yaw=0, so flip by 180° around Y so the character's front
        // aligns with player forward.
        let model_pre_transform =
            Mat4::from_rotation_y(std::f32::consts::PI) * Mat4::from_scale(Vec3::splat(1.0));

        Character {
            vbuf, ibuf, index_count: indices.len() as u32,
            bone_buffer, bone_bind_group,
            node_count, node_parents, node_base_trs,
            joint_node_indices, inverse_bind_matrices,
            clips, walk_clip,
            model_pre_transform,
        }
    }

    /// The resolved name of the walking clip (or None if the asset
    /// has no animations). Callers reference clips by name and
    /// this is the one they should use for the "walk" state.
    pub fn walk_clip_name(&self) -> Option<&str> {
        self.walk_clip.as_deref()
    }

    /// Look up the duration (in seconds) of a named clip.
    #[allow(dead_code)]
    pub fn clip_duration(&self, name: &str) -> Option<f32> {
        self.clips.get(name).map(|c| c.duration)
    }

    /// Return the first candidate clip name that this character
    /// actually has. Lets the caller try common Mixamo/asset
    /// names like `["Punch", "Attack", "Slash"]` without checking
    /// each one explicitly.
    pub fn find_clip<'a>(&self, candidates: &'a [&'a str]) -> Option<&'a str> {
        candidates.iter().copied().find(|c| self.clips.contains_key(*c))
    }

    /// Drive the skin. Each entry in `samples` is `(clip_name,
    /// anim_time, weight)`; samples are applied in order, each
    /// blending the named clip's pose on top of the running result
    /// by `weight` (1.0 = fully replace, 0.0 = no contribution).
    /// Unknown clip names are silently skipped, so the caller can
    /// safely request optional clips like "Jump" without checking.
    pub fn update(
        &self,
        queue: &wgpu::Queue,
        player_pos: Vec3,
        body_yaw: f32,
        samples: &[(&str, f32, f32)],
        extra_pre: Mat4,
    ) {
        let mut local_trs = self.node_base_trs.clone();
        for &(name, time, weight) in samples {
            if weight <= 0.0 {
                continue;
            }
            if let Some(clip) = self.clips.get(name) {
                Self::sample_clip_into(&mut local_trs, clip, time, weight.min(1.0));
            }
        }

        let mut global = vec![Mat4::IDENTITY; self.node_count];
        for i in 0..self.node_count {
            let (t, r, s) = local_trs[i];
            let local = Mat4::from_scale_rotation_translation(s, r, t);
            global[i] = match self.node_parents[i] {
                Some(p) => global[p] * local,
                None => local,
            };
        }

        let model = Mat4::from_translation(player_pos)
            * Mat4::from_rotation_y(body_yaw)
            * extra_pre
            * self.model_pre_transform;

        let bones: Vec<[[f32; 4]; 4]> = self
            .joint_node_indices
            .iter()
            .enumerate()
            .map(|(j, &node_idx)| {
                let m = model * global[node_idx] * self.inverse_bind_matrices[j];
                m.to_cols_array_2d()
            })
            .collect();

        queue.write_buffer(&self.bone_buffer, 0, bytemuck::cast_slice(&bones));
    }

    fn sample_clip_into(
        local_trs: &mut [(Vec3, Quat, Vec3)],
        clip: &AnimClip,
        time: f32,
        weight: f32,
    ) {
        let t = if clip.duration > 0.0 {
            time.rem_euclid(clip.duration)
        } else {
            0.0
        };
        for ch in &clip.channels {
            let (a, b, ft) = sample_keyframe(&ch.times, t);
            let base = local_trs[ch.target_node];
            match &ch.values {
                AnimValues::Translations(vs) => {
                    let v = if a == b { vs[a] } else { vs[a].lerp(vs[b], ft) };
                    local_trs[ch.target_node].0 = base.0.lerp(v, weight);
                }
                AnimValues::Rotations(qs) => {
                    let q = if a == b { qs[a] } else { qs[a].slerp(qs[b], ft) };
                    local_trs[ch.target_node].1 = base.1.slerp(q, weight);
                }
                AnimValues::Scales(vs) => {
                    let v = if a == b { vs[a] } else { vs[a].lerp(vs[b], ft) };
                    local_trs[ch.target_node].2 = base.2.lerp(v, weight);
                }
            }
        }
    }
}

fn sample_keyframe(times: &[f32], t: f32) -> (usize, usize, f32) {
    if times.is_empty() {
        return (0, 0, 0.0);
    }
    if times.len() == 1 || t <= times[0] {
        return (0, 0, 0.0);
    }
    let last = times.len() - 1;
    if t >= times[last] {
        return (last, last, 0.0);
    }
    for i in 0..last {
        if t >= times[i] && t < times[i + 1] {
            let span = times[i + 1] - times[i];
            let f = if span > 0.0 { (t - times[i]) / span } else { 0.0 };
            return (i, i + 1, f);
        }
    }
    (0, 0, 0.0)
}
