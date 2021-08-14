use super::gpu::Gpu;
use crate::color::Color;
use zerocopy::AsBytes;

#[derive(Copy, Clone, Debug, Default, AsBytes)]
#[repr(C)]
pub struct Quad {
    position: [f32; 2],
    width: f32,
    height: f32,
    pub color: Color,
}

impl Quad {
    pub const fn new(x: f32, y: f32, width: f32, height: f32, color: Color) -> Self {
        Self {
            position: [x, y],
            width,
            height,
            color,
        }
    }

    const fn x(&self) -> f32 {
        self.position[0]
    }

    const fn y(&self) -> f32 {
        self.position[1]
    }

    fn set_x(&mut self, x: f32) {
        self.position[0] = x;
    }

    fn set_y(&mut self, y: f32) {
        self.position[1] = y;
    }

    const fn vertex_buffer_descriptior() -> wgpu::VertexBufferDescriptor<'static> {
        const COLOR_VERTEX_DESCRIPTOR: wgpu::VertexAttributeDescriptor = Color::vertex_attribute_descriptor(3, 8 + 4 + 4);

        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float2,
                    offset: 0,
                },
                wgpu::VertexAttributeDescriptor {
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float,
                    offset: 8, /* size of Float2 */
                },
                wgpu::VertexAttributeDescriptor {
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float,
                    offset: 8 /* size of Float2 */ + 4, /* size of Float */
                },
                COLOR_VERTEX_DESCRIPTOR
            ],
        }
    }
}

pub struct QuadRenderer {
    pipeline: wgpu::RenderPipeline,
    indices: wgpu::Buffer,
    instances: wgpu::Buffer,
    pending: Vec<Quad>,
}

impl QuadRenderer {
    const MAX_INSTANCES: usize = 10_000;
    const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];
}

impl QuadRenderer {
    pub fn new(gpu: &mut Gpu) -> Self {
        let vs_module =
            gpu.create_shader_module(wgpu::include_spirv!("../../shaders/quad.vert.spv"));
        let fs_module =
            gpu.create_shader_module(wgpu::include_spirv!("../../shaders/quad.frag.spv"));

        let render_pipeline_layout = gpu.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("weovim-quad-pipeline-layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = gpu.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("weovim-quad-renderer-pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Cw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
                clamp_depth: false,
            }),
            color_states: &[wgpu::ColorStateDescriptor {
                format: gpu.color_format(),
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[Quad::vertex_buffer_descriptior()],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let indices = gpu.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("weovim::ui::renderers::quad indices buffer"),
            usage: wgpu::BufferUsage::INDEX,
            contents: Self::QUAD_INDICES.as_bytes(),
        });

        let instances = gpu.create_buffer(&wgpu::BufferDescriptor {
            label: Some("weovim::ui::renderers:: quad instances buffer"),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            size: std::mem::size_of::<Quad>() as u64 * Self::MAX_INSTANCES as u64,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            indices,
            instances,
            pending: Vec::with_capacity(1024),
        }
    }

    pub fn queue(&mut self, quad: Quad) {
        self.pending.push(quad);
    }

    pub fn render_in(
        &mut self,
        frame: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        gpu: &mut Gpu,
    ) {
        let mut i = 0;
        let total = self.pending.len();

        while i < total {
            let end = (i + Self::MAX_INSTANCES).min(total);
            let amount = end - i;

            let instances_bytes = self.pending[i..end].as_bytes();

            let size = wgpu::BufferSize::new(instances_bytes.len() as u64).unwrap();
            gpu.write_buffer(encoder, &self.instances, 0, size)
                .copy_from_slice(instances_bytes);

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: frame,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_index_buffer(self.indices.slice(..));
                render_pass.set_vertex_buffer(0, self.instances.slice(..));

                render_pass.draw_indexed(0..Self::QUAD_INDICES.len() as u32, 0, 0..amount as u32);
            }

            i += Self::MAX_INSTANCES;
        }
    }
}
