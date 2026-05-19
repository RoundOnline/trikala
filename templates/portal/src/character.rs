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
    pub uv: [f32; 2],
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

/// One drawable slice of the character — one primitive's worth of
/// indices into the shared `vbuf`/`ibuf`, plus a bind group that
/// binds the shared bone buffer alongside that primitive's diffuse
/// texture. Erika Archer has four such submeshes (body, clothes,
/// eyes, eyelashes); simpler characters typically have one.
pub struct SubMesh {
    pub index_start: u32,
    pub index_count: u32,
    pub bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    texture: wgpu::Texture,
    #[allow(dead_code)]
    sampler: wgpu::Sampler,
}

pub struct Character {
    pub vbuf: wgpu::Buffer,
    pub ibuf: wgpu::Buffer,
    pub index_count: u32,
    pub bone_buffer: wgpu::Buffer,
    /// Bind group used by the shadow pass — it samples no textures
    /// but still expects the bone buffer at slot 0; we just point its
    /// texture/sampler slots at the first submesh as a placeholder.
    pub bone_bind_group: wgpu::BindGroup,
    /// One entry per skinned primitive in the source asset. The
    /// main render pass iterates these so each part draws with its
    /// own diffuse texture.
    pub submeshes: Vec<SubMesh>,

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
        queue: &wgpu::Queue,
        path: &str,
        bone_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let (doc, buffers, images) = gltf::import(path).expect("failed to load glb");

        // Erika Archer (and most fully-detailed Mixamo characters)
        // split the figure across multiple nodes — body, hair, eyes,
        // eyelashes, outfit, each its own mesh + skin pair sharing
        // the same skeleton. We have to concatenate every such node,
        // not just the first one, or the visible result is e.g. the
        // eyelashes hovering in space. We grab the skin from the
        // first skinned-mesh node we see and trust that all the
        // other skinned-mesh nodes share it.
        let skin = doc
            .nodes()
            .find(|n| n.skin().is_some() && n.mesh().is_some())
            .and_then(|n| n.skin())
            .expect("glTF has no skinned mesh node");

        // First pass: walk every skinned primitive once, appending its
        // vertices/indices into the shared buffers and remembering
        // each primitive's index range + which glTF image it wants
        // for its diffuse texture. Then in a second pass we'll turn
        // those records into GPU textures + bind groups.
        struct SubMeshDesc {
            index_start: u32,
            index_count: u32,
            tex_index: Option<usize>,
        }
        let mut sub_descs: Vec<SubMeshDesc> = Vec::new();
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut joints_raw: Vec<[u16; 4]> = Vec::new();
        let mut weights_raw: Vec<[f32; 4]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for node in doc.nodes() {
            let (Some(mesh), Some(_)) = (node.mesh(), node.skin()) else { continue };
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
                let prim_uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|it| it.into_f32().collect::<Vec<[f32; 2]>>())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; prim_positions.len()]);
                let prim_indices: Vec<u32> = reader
                    .read_indices()
                    .map(|it| it.into_u32().collect())
                    .unwrap_or_else(|| (0..prim_positions.len() as u32).collect());

                let tex_index = primitive
                    .material()
                    .pbr_metallic_roughness()
                    .base_color_texture()
                    .map(|info| info.texture().source().index());

                let base = positions.len() as u32;
                positions.extend(prim_positions);
                normals.extend(prim_normals);
                joints_raw.extend(prim_joints.into_u16());
                weights_raw.extend(prim_weights.into_f32());
                uvs.extend(prim_uvs);
                let index_start = indices.len() as u32;
                indices.extend(prim_indices.iter().map(|i| i + base));
                let index_count = indices.len() as u32 - index_start;
                sub_descs.push(SubMeshDesc { index_start, index_count, tex_index });
            }
        }
        println!("[character] {} skinned submeshes", sub_descs.len());
        assert!(!positions.is_empty(), "no skinned primitives in glTF");

        // Quick AABB of vertex positions — helps catch scale/origin
        // mismatches (Mixamo exports are often 100× larger than the
        // scene because of cm/m unit confusion).
        let mut mn = [f32::INFINITY; 3];
        let mut mx = [f32::NEG_INFINITY; 3];
        for p in &positions {
            for i in 0..3 {
                if p[i] < mn[i] { mn[i] = p[i]; }
                if p[i] > mx[i] { mx[i] = p[i]; }
            }
        }
        println!(
            "[character] mesh aabb min=({:.3},{:.3},{:.3}) max=({:.3},{:.3},{:.3}) size=({:.3},{:.3},{:.3})",
            mn[0], mn[1], mn[2], mx[0], mx[1], mx[2],
            mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]
        );

        let vertices: Vec<SkinVertex> = (0..positions.len())
            .map(|i| SkinVertex {
                pos: positions[i],
                normal: normals[i],
                color: [1.0, 1.0, 1.0], // multiply with texture sample
                emissive: 0.0,
                joints: [
                    joints_raw[i][0] as u32,
                    joints_raw[i][1] as u32,
                    joints_raw[i][2] as u32,
                    joints_raw[i][3] as u32,
                ],
                weights: weights_raw[i],
                uv: uvs[i],
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
        let mut hips_node: Option<usize> = None;
        for node in doc.nodes() {
            let (t, r, s) = node.transform().decomposed();
            node_base_trs[node.index()] = (
                Vec3::from_array(t),
                Quat::from_xyzw(r[0], r[1], r[2], r[3]),
                Vec3::from_array(s),
            );
            if hips_node.is_none() {
                if let Some(name) = node.name() {
                    if name.to_lowercase().contains("hips") {
                        hips_node = Some(node.index());
                    }
                }
            }
        }
        if let Some(h) = hips_node {
            println!("[character] hips node index = {} (root-motion translation will be stripped)", h);
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
                let is_hips_translation = matches!(
                    (hips_node, &outputs),
                    (Some(h), gltf::animation::util::ReadOutputs::Translations(_)) if h == target_node
                );
                if is_hips_translation {
                    // Mixamo bakes root motion into the Hips translation —
                    // the character literally walks forward in clip-local
                    // space, which would double up with gameplay-driven
                    // player.pos and cause a per-loop snap-back. Skip the
                    // channel so the Hips stays parented at the bind-pose
                    // origin; rotation channels still drive the gait.
                    continue;
                }
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

        // Build one diffuse texture + sampler + bind group per glTF
        // primitive. Each submesh's own material's base-color texture
        // wins; primitives without a texture fall back to 1×1 white.
        let make_texture = |tex_index: Option<usize>| -> (wgpu::Texture, wgpu::Sampler) {
            let (tex_w, tex_h, tex_pixels) = if let Some(idx) = tex_index {
                let img = &images[idx];
                let pixels = match img.format {
                    gltf::image::Format::R8G8B8 => {
                        let mut rgba = Vec::with_capacity(img.pixels.len() / 3 * 4);
                        for chunk in img.pixels.chunks(3) {
                            rgba.extend_from_slice(chunk);
                            rgba.push(255);
                        }
                        rgba
                    }
                    gltf::image::Format::R8G8B8A8 => img.pixels.clone(),
                    _ => vec![255; (img.width * img.height * 4) as usize],
                };
                (img.width, img.height, pixels)
            } else {
                (1, 1, vec![255, 255, 255, 255])
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("char-diffuse"),
                size: wgpu::Extent3d { width: tex_w, height: tex_h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &tex_pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(tex_w * 4),
                    rows_per_image: Some(tex_h),
                },
                wgpu::Extent3d { width: tex_w, height: tex_h, depth_or_array_layers: 1 },
            );
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("char-diffuse-samp"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            (texture, sampler)
        };

        let submeshes: Vec<SubMesh> = sub_descs
            .into_iter()
            .map(|d| {
                let (texture, sampler) = make_texture(d.tex_index);
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("char-submesh-bg"),
                    layout: bone_bgl,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: bone_buffer.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                    ],
                });
                SubMesh {
                    index_start: d.index_start,
                    index_count: d.index_count,
                    bind_group,
                    texture,
                    sampler,
                }
            })
            .collect();

        // Shadow pass bind group — uses the same layout but doesn't
        // sample the texture; we just point it at the first submesh's
        // resources as a placeholder so the layout type-checks.
        let bone_bind_group = if let Some(first) = submeshes.first() {
            // Recreate using first submesh's texture/sampler via a
            // separate bind group bound to the same layout. Easiest
            // way: reuse the first submesh's bind group directly.
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("char-shadow-bg"),
                layout: bone_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: bone_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &first.texture.create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&first.sampler) },
                ],
            })
        } else {
            unreachable!("no submeshes after assertion above");
        };

        // Erika Archer (and most Mixamo characters out of FBX2glTF)
        // is authored facing +Z, so rotate 180° around Y to make her
        // face -Z which matches the player's forward at yaw=0. The
        // 1.15× scale brings the proportions up to a slightly larger
        // hero-shot read in third-person without distorting the rig
        // (uniform scale preserves bone lengths and joint angles).
        let model_pre_transform =
            Mat4::from_rotation_y(std::f32::consts::PI) * Mat4::from_scale(Vec3::splat(1.15));

        Character {
            vbuf, ibuf, index_count: indices.len() as u32,
            bone_buffer, bone_bind_group, submeshes,
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
