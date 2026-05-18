use crate::geometry::PosVertex;
use crate::pipelines::{make_layout, make_shader};

const FADE_SHADER: &str = include_str!("../shaders/fade.wgsl");

pub struct FadeSystem {
    pub pipeline: wgpu::RenderPipeline,
    pub vbuf: wgpu::Buffer,
}

pub fn build_fade_pipeline(
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
