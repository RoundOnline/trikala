//! Shared pipeline helpers + the world / shadow / character pipelines.
//!
//! Per-subsystem pipelines (grass, water, sand, fade, portal) live in
//! their own `subsystems::*` module — each owns its shader, its
//! `RenderPipeline`, and the small struct that ties them together.

use crate::character::SkinVertex;
use crate::geometry::Vertex;

const MAIN_SHADER:        &str = include_str!("shaders/main.wgsl");
const SHADOW_SHADER:      &str = include_str!("shaders/shadow.wgsl");
const SKIN_SHADER:        &str = include_str!("shaders/skin.wgsl");
const SKIN_SHADOW_SHADER: &str = include_str!("shaders/skin_shadow.wgsl");

pub const VERTEX_ATTRS: [wgpu::VertexAttribute; 4] = [
    wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 36, shader_location: 3, format: wgpu::VertexFormat::Float32 },
];

pub const SKIN_VERTEX_ATTRS: [wgpu::VertexAttribute; 7] = [
    wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 36, shader_location: 3, format: wgpu::VertexFormat::Float32 },
    wgpu::VertexAttribute { offset: 40, shader_location: 4, format: wgpu::VertexFormat::Uint32x4 },
    wgpu::VertexAttribute { offset: 56, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
    wgpu::VertexAttribute { offset: 72, shader_location: 6, format: wgpu::VertexFormat::Float32x2 },
];

pub fn make_shader(device: &wgpu::Device, label: &str, src: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(src.into()),
    })
}

pub fn make_layout(
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

pub fn depth_opaque() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: Default::default(),
        bias: Default::default(),
    }
}

/// Bind-group layout for the skinned character's per-mesh resources
/// (bone storage + diffuse texture + sampler).
pub fn build_bone_bgl(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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

pub fn build_main_pipeline(
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

pub fn build_shadow_pipeline(
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

pub fn build_skin_pipeline(
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

pub fn build_skin_shadow_pipeline(
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
