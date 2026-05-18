//! The two-door teleport portal: each door has its own offscreen
//! colour target (where the destination room is rendered each frame),
//! plus a shared door-frame mesh and a magic-surface quad inside the
//! frame that samples the right colour target.

use crate::geometry::Vertex;
use crate::pipelines::{depth_opaque, make_layout, make_shader, VERTEX_ATTRS};

const PORTAL_SHADER: &str = include_str!("../shaders/portal.wgsl");

/// Render-to-texture portals + the door frame + magic-surface mesh.
/// Both doors share one mesh; indices [0..N) are door A, [N..2N) are
/// door B, with N stored as `indices_per_door`.
pub struct PortalSystem {
    /// Per-door offscreen colour target where the destination room is
    /// rendered each frame. [0] = door A (bit-0 axis), [1] = door B.
    pub color_views: [wgpu::TextureView; 2],
    pub depth_views: [wgpu::TextureView; 2],
    pub tex_bgs: [wgpu::BindGroup; 2],
    pub sampler: wgpu::Sampler,
    pub bgl1: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
    pub indices_per_door: u32,
    pub vbuf: wgpu::Buffer,
    pub ibuf: wgpu::Buffer,
    pub door_vbuf: wgpu::Buffer,
    pub door_ibuf: wgpu::Buffer,
    pub door_index_count: u32,
}

pub fn build_portal_pipeline(
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
