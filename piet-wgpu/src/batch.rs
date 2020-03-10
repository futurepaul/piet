use crate::{
    gradient::{GradientHandle, GradientStore},
    WgpuBrush, WgpuCtx, WgpuVertex, MSAA_SAMPLES,
};
use lyon::tessellation::geometry_builder::*;
use zerocopy::AsBytes;

pub struct RenderPipelines {
    solid_pipeline: wgpu::RenderPipeline,
    solid_bind_group_layout: wgpu::BindGroupLayout,
    linear_gradient_pipeline: wgpu::RenderPipeline,
    linear_gradient_bind_group_layout: wgpu::BindGroupLayout,
}

impl RenderPipelines {
    pub fn new(
        device: &wgpu::Device,
        global_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> RenderPipelines {
        let (solid_pipeline, solid_bind_group_layout) = {
            let vs_bytes = include_bytes!("../shaders/solid.vert.spv");
            let vs_spv = wgpu::read_spirv(std::io::Cursor::new(&vs_bytes[..])).unwrap();
            let vs_module = device.create_shader_module(&vs_spv);
            let fs_bytes = include_bytes!("../shaders/solid.frag.spv");
            let fs_spv = wgpu::read_spirv(std::io::Cursor::new(&fs_bytes[..])).unwrap();
            let fs_module = device.create_shader_module(&fs_spv);

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    bindings: &[wgpu::BindGroupLayoutBinding {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    }],
                });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[global_bind_group_layout, &bind_group_layout],
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                layout: &pipeline_layout,
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fs_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::None,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[wgpu::ColorStateDescriptor {
                    format: wgpu::TextureFormat::Rgba8Unorm,
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
                index_format: wgpu::IndexFormat::Uint32,
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<WgpuVertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            offset: 0,
                            format: wgpu::VertexFormat::Float2,
                            shader_location: 0,
                        },
                        wgpu::VertexAttributeDescriptor {
                            offset: 8,
                            format: wgpu::VertexFormat::Uint,
                            shader_location: 1,
                        },
                    ],
                }],
                sample_count: MSAA_SAMPLES,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });
            (pipeline, bind_group_layout)
        };

        let (linear_gradient_pipeline, linear_gradient_bind_group_layout) = {
            let vs_bytes = include_bytes!("../shaders/linear_gradient.vert.spv");
            let vs_spv = wgpu::read_spirv(std::io::Cursor::new(&vs_bytes[..])).unwrap();
            let vs_module = device.create_shader_module(&vs_spv);
            let fs_bytes = include_bytes!("../shaders/linear_gradient.frag.spv");
            let fs_spv = wgpu::read_spirv(std::io::Cursor::new(&fs_bytes[..])).unwrap();
            let fs_module = device.create_shader_module(&fs_spv);

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    bindings: &[
                        wgpu::BindGroupLayoutBinding {
                            binding: 0,
                            visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                        },
                        wgpu::BindGroupLayoutBinding {
                            binding: 1,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::SampledTexture {
                                multisampled: false,
                                dimension: wgpu::TextureViewDimension::D1,
                            },
                        },
                        wgpu::BindGroupLayoutBinding {
                            binding: 2,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::Sampler,
                        },
                    ],
                });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[global_bind_group_layout, &bind_group_layout],
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                layout: &pipeline_layout,
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fs_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::None,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[wgpu::ColorStateDescriptor {
                    format: wgpu::TextureFormat::Rgba8Unorm,
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
                index_format: wgpu::IndexFormat::Uint32,
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<WgpuVertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            offset: 0,
                            format: wgpu::VertexFormat::Float2,
                            shader_location: 0,
                        },
                        wgpu::VertexAttributeDescriptor {
                            offset: 8,
                            format: wgpu::VertexFormat::Uint,
                            shader_location: 1,
                        },
                    ],
                }],
                sample_count: MSAA_SAMPLES,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });
            (pipeline, bind_group_layout)
        };

        RenderPipelines {
            solid_pipeline,
            solid_bind_group_layout,
            linear_gradient_pipeline,
            linear_gradient_bind_group_layout,
        }
    }
}

pub struct BatchList {
    batches: Vec<Batch>,
}

impl BatchList {
    pub fn new() -> BatchList {
        BatchList {
            batches: Vec::new(),
        }
    }

    pub fn request_mesh(
        &mut self,
        device: &wgpu::Device,
        pipelines: &RenderPipelines,
        brush: &WgpuBrush,
    ) -> (&mut VertexBuffers<WgpuVertex, u32>, u32) {
        let has_compatible_batch = self
            .batches
            .last()
            .map(|batch| batch.kind.is_compatible_with_brush(brush))
            .unwrap_or(false);

        if !has_compatible_batch {
            self.batches
                .push(Batch::new_for_brush(brush, device, pipelines));
        }

        // Now we have a compatible batch at the top of the batch stack
        let batch = self.batches.last_mut().unwrap();

        match (&mut batch.kind, brush) {
            (BatchKind::Solid(batch_info), WgpuBrush::Solid(color)) => {
                batch_info.primitives.push(SolidPrimitive {
                    color: [
                        color.r as f32,
                        color.g as f32,
                        color.b as f32,
                        color.a as f32,
                    ],
                });
                (
                    &mut batch_info.mesh,
                    (batch_info.primitives.len() - 1) as u32,
                )
            }
            (BatchKind::LinearGradient(batch_info), WgpuBrush::LinearGradient(gradient)) => {
                batch_info.primitives.push(LinearGradientPrimitive {
                    start: [gradient.start.x as f32, gradient.start.y as f32],
                    end: [gradient.end.x as f32, gradient.end.y as f32],
                });
                (
                    &mut batch_info.mesh,
                    (batch_info.primitives.len() - 1) as u32,
                )
            }
            _ => panic!("Tried to add to invalid batch"),
        }
    }

    pub fn prepare_for_render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        pipelines: &RenderPipelines,
        gradient_store: &mut GradientStore,
    ) {
        for batch in &mut self.batches {
            batch.prepare_for_render(device, encoder, pipelines, gradient_store);
        }
    }

    pub fn render<'a>(&'a self, pipelines: &'a RenderPipelines, rpass: &mut wgpu::RenderPass<'a>) {
        for batch in &self.batches {
            batch.render(pipelines, rpass);
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, AsBytes)]
pub struct SolidPrimitive {
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, AsBytes)]
pub struct LinearGradientPrimitive {
    start: [f32; 2],
    end: [f32; 2],
}

pub struct Batch {
    kind: BatchKind,
}

impl Batch {
    fn new_for_brush(
        brush: &WgpuBrush,
        device: &wgpu::Device,
        pipelines: &RenderPipelines,
    ) -> Batch {
        match brush {
            WgpuBrush::Solid(..) => Batch {
                kind: BatchKind::Solid(SolidBatch::new(device, pipelines)),
            },
            WgpuBrush::LinearGradient(gradient) => Batch {
                kind: BatchKind::LinearGradient(LinearGradientBatch::new(
                    device,
                    pipelines,
                    gradient.handle.clone(),
                )),
            },
        }
    }

    fn prepare_for_render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        pipelines: &RenderPipelines,
        gradient_store: &mut GradientStore,
    ) {
        match &mut self.kind {
            BatchKind::Solid(batch) => batch.prepare_for_render(device, encoder),
            BatchKind::LinearGradient(batch) => {
                batch.prepare_for_render(device, encoder, pipelines, gradient_store)
            }
        }
    }

    pub fn render<'a>(&'a self, pipelines: &'a RenderPipelines, rpass: &mut wgpu::RenderPass<'a>) {
        match &self.kind {
            BatchKind::Solid(batch) => batch.render(pipelines, rpass),
            BatchKind::LinearGradient(batch) => batch.render(pipelines, rpass),
        }
    }
}

pub enum BatchKind {
    Solid(SolidBatch),
    LinearGradient(LinearGradientBatch),
}

impl BatchKind {
    fn is_compatible_with_brush(&self, brush: &WgpuBrush) -> bool {
        match self {
            BatchKind::Solid(..) => brush.is_solid(),
            // TODO: Check color!
            BatchKind::LinearGradient(batch) => {
                if !brush.is_linear_gradient() {
                    return false;
                }
                let gradient_handle = match brush {
                    WgpuBrush::LinearGradient(gradient) => &gradient.handle,
                    _ => unreachable!(),
                };
                &batch.gradient_handle == gradient_handle
            },
        }
    }
}

const MAX_VERTEX_BUFFER_SIZE: usize = 35000;
const MAX_INDEX_BUFFER_SIZE: usize = 92000;
const MAX_PRIM_BUFFER_SIZE: usize = 92000;

pub struct SolidBatch {
    mesh: VertexBuffers<WgpuVertex, u32>,
    primitives: Vec<SolidPrimitive>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    prim_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl SolidBatch {
    fn new(device: &wgpu::Device, pipelines: &RenderPipelines) -> SolidBatch {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: (MAX_VERTEX_BUFFER_SIZE * std::mem::size_of::<WgpuVertex>()) as u64,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: (MAX_INDEX_BUFFER_SIZE * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
        });

        let prim_buffer_size =
            (MAX_PRIM_BUFFER_SIZE * std::mem::size_of::<SolidPrimitive>()) as u64;
        let prim_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: prim_buffer_size,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &pipelines.solid_bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &prim_buffer,
                    range: 0..prim_buffer_size,
                },
            }],
        });

        SolidBatch {
            mesh: VertexBuffers::new(),
            primitives: Vec::new(),
            vertex_buffer,
            index_buffer,
            prim_buffer,
            bind_group,
        }
    }

    fn prepare_for_render(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
        assert!(self.mesh.vertices.len() < MAX_VERTEX_BUFFER_SIZE);
        let temp_buffer = device.create_buffer_with_data(
            &self.mesh.vertices.as_bytes(),
            wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_SRC,
        );

        encoder.copy_buffer_to_buffer(
            &temp_buffer,
            0,
            &self.vertex_buffer,
            0,
            (self.mesh.vertices.len() * std::mem::size_of::<WgpuVertex>()) as u64,
        );

        assert!(self.mesh.indices.len() < MAX_INDEX_BUFFER_SIZE);
        let temp_buffer = device.create_buffer_with_data(
            &self.mesh.indices.as_bytes(),
            wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_SRC,
        );

        encoder.copy_buffer_to_buffer(
            &temp_buffer,
            0,
            &self.index_buffer,
            0,
            (self.mesh.indices.len() * std::mem::size_of::<u32>()) as u64,
        );

        assert!(self.primitives.len() < MAX_PRIM_BUFFER_SIZE);
        let temp_buffer = device.create_buffer_with_data(
            &self.primitives.as_bytes(),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_SRC,
        );

        encoder.copy_buffer_to_buffer(
            &temp_buffer,
            0,
            &self.prim_buffer,
            0,
            (self.primitives.len() * std::mem::size_of::<SolidPrimitive>()) as u64,
        );
    }

    pub fn render<'a>(&'a self, pipelines: &'a RenderPipelines, rpass: &mut wgpu::RenderPass<'a>) {
        rpass.set_pipeline(&pipelines.solid_pipeline);
        // Bind the info needed for this batch
        rpass.set_bind_group(1, &self.bind_group, &[]);
        rpass.set_index_buffer(&self.index_buffer, 0);
        rpass.set_vertex_buffers(0, &[(&self.vertex_buffer, 0)]);
        rpass.draw_indexed(0..(self.mesh.indices.len() as u32), 0, 0..1);
    }
}

pub struct LinearGradientBatch {
    mesh: VertexBuffers<WgpuVertex, u32>,
    primitives: Vec<LinearGradientPrimitive>,
    gradient_handle: GradientHandle,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    prim_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,

    // Will be filled in during prepare for render
    bind_group: Option<wgpu::BindGroup>,
    gradient_view: Option<wgpu::TextureView>,
}

impl LinearGradientBatch {
    fn new(
        device: &wgpu::Device,
        pipelines: &RenderPipelines,
        gradient_handle: GradientHandle,
    ) -> LinearGradientBatch {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: (MAX_VERTEX_BUFFER_SIZE * std::mem::size_of::<WgpuVertex>()) as u64,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: (MAX_INDEX_BUFFER_SIZE * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
        });

        let prim_buffer_size =
            (MAX_PRIM_BUFFER_SIZE * std::mem::size_of::<SolidPrimitive>()) as u64;
        let prim_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: prim_buffer_size,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare_function: wgpu::CompareFunction::Always,
        });

        LinearGradientBatch {
            mesh: VertexBuffers::new(),
            primitives: Vec::new(),
            gradient_handle,
            sampler,
            vertex_buffer,
            index_buffer,
            prim_buffer,
            bind_group: None,
            gradient_view: None,
        }
    }

    fn prepare_for_render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        pipelines: &RenderPipelines,
        gradient_store: &mut GradientStore,
    ) {
        assert!(self.mesh.vertices.len() < MAX_VERTEX_BUFFER_SIZE);
        let prim_buffer_size =
            (MAX_PRIM_BUFFER_SIZE * std::mem::size_of::<SolidPrimitive>()) as u64;

        let gradient = gradient_store.get_texture(device, encoder, &self.gradient_handle);
        self.gradient_view = Some(gradient.create_default_view());
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &pipelines.linear_gradient_bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &self.prim_buffer,
                        range: 0..prim_buffer_size,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        self.gradient_view.as_ref().unwrap(),
                    ),
                },
                wgpu::Binding {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));

        let temp_buffer = device.create_buffer_with_data(
            &self.mesh.vertices.as_bytes(),
            wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_SRC,
        );

        encoder.copy_buffer_to_buffer(
            &temp_buffer,
            0,
            &self.vertex_buffer,
            0,
            (self.mesh.vertices.len() * std::mem::size_of::<WgpuVertex>()) as u64,
        );

        assert!(self.mesh.indices.len() < MAX_INDEX_BUFFER_SIZE);
        let temp_buffer = device.create_buffer_with_data(
            &self.mesh.indices.as_bytes(),
            wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_SRC,
        );

        encoder.copy_buffer_to_buffer(
            &temp_buffer,
            0,
            &self.index_buffer,
            0,
            (self.mesh.indices.len() * std::mem::size_of::<u32>()) as u64,
        );

        assert!(self.primitives.len() < MAX_PRIM_BUFFER_SIZE);
        let temp_buffer = device.create_buffer_with_data(
            &self.primitives.as_bytes(),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_SRC,
        );

        encoder.copy_buffer_to_buffer(
            &temp_buffer,
            0,
            &self.prim_buffer,
            0,
            (self.primitives.len() * std::mem::size_of::<LinearGradientPrimitive>()) as u64,
        );
    }

    pub fn render<'a>(&'a self, pipelines: &'a RenderPipelines, rpass: &mut wgpu::RenderPass<'a>) {
        rpass.set_pipeline(&pipelines.linear_gradient_pipeline);
        // Bind the info needed for this batch
        rpass.set_bind_group(1, self.bind_group.as_ref().unwrap(), &[]);
        rpass.set_index_buffer(&self.index_buffer, 0);
        rpass.set_vertex_buffers(0, &[(&self.vertex_buffer, 0)]);
        rpass.draw_indexed(0..(self.mesh.indices.len() as u32), 0, 0..1);
    }
}
