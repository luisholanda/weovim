use super::transform::Transformation;
use std::io::Cursor;
use std::mem;

pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    constants: wgpu::BindGroup,
    transform: wgpu::Buffer,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    instances: wgpu::Buffer,
    quads: Vec<Quad>,
}

fn load_modules() -> (Vec<u32>, Vec<u32>) {
    use shaderc::*;

    let mut compiler = Compiler::new().expect("Failed to initalize SPIR-V compiler!");

    let frag = compiler.compile_into_spirv(
        include_str!("shader/quad.frag"),
        ShaderKind::Fragment,
        "quad.frag",
        "main",
        None).unwrap();

    let vert = compiler.compile_into_spirv(
        include_str!("shader/quad.vert"),
        ShaderKind::Vertex,
        "quad.vert",
        "main",
        None
    ).unwrap();

    let frag_spirv = wgpu::read_spirv(Cursor::new(frag.as_binary_u8())).expect("Failed to read SPIR-V fragment shader");
    let vert_spirv = wgpu::read_spirv(Cursor::new(vert.as_binary_u8())).expect("Failed to read SPIR-V fragment shader");

    (frag_spirv, vert_spirv)
}

impl Pipeline {
    pub fn new(device: &mut wgpu::Device) -> Pipeline {
        let (transform, constants, layout) = {
            let constant_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    bindings: &[wgpu::BindGroupLayoutBinding {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    }],
                });

            let matrix: [f32; 16] = Transformation::identity().into();

            let transform = device
                .create_buffer_mapped(16, wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST)
                .fill_from_slice(&matrix[..]);

            let constants = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &constant_layout,
                bindings: &[wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &transform,
                        range: 0..(16 * mem::size_of::<f32>() as u64),
                    },
                }],
            });

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&constant_layout],
            });

            (transform, constants, layout)
        };

        let (vs_module, fs_module) = {
            let (frag_spirv, vert_spirv) = load_modules();

            (device.create_shader_module(&vert_spirv), device.create_shader_module(&frag_spirv))
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &layout,
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
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[
                wgpu::VertexBufferDescriptor {
                    stride: mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[wgpu::VertexAttributeDescriptor {
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float2,
                        offset: 0,
                    }],
                },
                wgpu::VertexBufferDescriptor {
                    stride: mem::size_of::<Quad>() as u64,
                    step_mode: wgpu::InputStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float2,
                            offset: 0,
                        },
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float2,
                            offset: 4 * 2,
                        },
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float4,
                            offset: 4 * (2 + 2),
                        },
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 4,
                            format: wgpu::VertexFormat::Uint,
                            offset: 4 * (2 + 2 + 4),
                        },
                    ],
                },
            ],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let vertices = device
            .create_buffer_mapped(QUAD_VERTS.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&QUAD_VERTS);

        let indices = device
            .create_buffer_mapped(QUAD_INDICES.len(), wgpu::BufferUsage::INDEX)
            .fill_from_slice(&QUAD_INDICES);

        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            size: mem::size_of::<Quad>() as u64 * Quad::MAX as u64,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        Self {
            pipeline,
            constants,
            transform,
            vertices,
            indices,
            instances,
            quads: Vec::new(),
        }
    }

    pub fn enqueue(&mut self, quad: Quad) {
        self.quads.push(quad)
    }

    pub fn draw(
        &mut self,
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        transformation: Transformation,
    ) {
        let matrix: [f32; 16] = transformation.into();

        let transform_buffer = device
            .create_buffer_mapped(16, wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&matrix[..]);

        encoder.copy_buffer_to_buffer(&transform_buffer, 0, &self.transform, 0, 16 * 4);

        let mut i = 0;
        let total = self.quads.len();

        while i < total {
            let end = (i + Quad::MAX).min(total);
            let amount = end - i;

            let instance_buffer = device
                .create_buffer_mapped(amount, wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(&self.quads[i..end]);

            encoder.copy_buffer_to_buffer(
                &instance_buffer,
                0,
                &self.instances,
                0,
                (mem::size_of::<Quad>() * amount) as u64,
            );

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: target,
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Load,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color::TRANSPARENT,
                    }],
                    depth_stencil_attachment: None,
                });

                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_bind_group(0, &self.constants, &[]);
                render_pass.set_index_buffer(&self.indices, 0);
                render_pass.set_vertex_buffers(0, &[(&self.vertices, 0), (&self.instances, 0)]);

                render_pass.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..amount as u32);
            }

            i += Quad::MAX;
        }
    }
}

pub type Vertex = [f32; 2];

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

const QUAD_VERTS: [Vertex; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

#[derive(Debug, Clone, Copy)]
pub struct Quad {
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub color: [f32; 4],
    pub border_radius: f32,
}

impl Quad {
    // TODO: Check a better value for `Quad::MAX`
    // BODY: As we will mostly render text. The current value should be more than enough
    // BODY: to properly render all the quads that we will have.
    // BODY: Decreasing this value will decrease the amount of I/O that we do with the GPU.
    const MAX: usize = 5_000;
}
