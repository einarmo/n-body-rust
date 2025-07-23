use wgpu::{
    BindGroup, BindGroupLayout, BlendComponent, BlendFactor, BlendState, Buffer, Device,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, TextureFormat,
};

use crate::{
    ShaderConstants,
    objects::{ObjectInstance, Vertex},
    render::get_or_init_shader,
};

pub(crate) struct CircleDrawPipeline {
    pipeline: RenderPipeline,
}

impl CircleDrawPipeline {
    pub fn new(
        device: &Device,
        texture_format: TextureFormat,
        camera_layout: &BindGroupLayout,
    ) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[camera_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                range: 0..std::mem::size_of::<ShaderConstants>() as u32,
            }],
        });

        let shader_module = get_or_init_shader(device);

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("circle pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader_module,
                entry_point: Some("circle_vs"),
                buffers: &[Vertex::layout::<false, 0>(), ObjectInstance::layout::<2>()],
                compilation_options: Default::default(),
            },
            cache: None,
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("circle_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            multiview: None,
        });

        Self { pipeline }
    }

    pub fn draw(
        &self,
        rpass: &mut RenderPass<'_>,
        camera: &BindGroup,
        last_batch_range: std::ops::Range<u64>,
        point_buffer: &Buffer,
        instance_buffer: &Buffer,
        push_constants: &ShaderConstants,
        num_objects: usize,
    ) {
        let last_batch_range =
            (last_batch_range.start * Vertex::size())..(last_batch_range.end * Vertex::size());

        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, point_buffer.slice(last_batch_range.clone()));
        rpass.set_vertex_buffer(1, instance_buffer.slice(..));

        rpass.set_bind_group(0, camera, &[]);

        rpass.set_push_constants(
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(push_constants),
        );

        rpass.draw(0..6, 0..(num_objects as u32));
    }
}
