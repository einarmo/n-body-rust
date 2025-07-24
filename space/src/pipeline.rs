use std::ops::Range;

use wgpu::{
    BindGroup, BindGroupLayout, BlendComponent, BlendFactor, BlendState, Buffer, Device,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, TextureFormat, util::DeviceExt,
};

use crate::{
    ShaderConstants,
    objects::{ObjectInstance, TRAIL_MAX_LENGTH, Vertex},
    render::get_or_init_shader,
};

pub(crate) struct LineDrawPipeline {
    index_buffer: Buffer,
    pipeline: RenderPipeline,
}

impl LineDrawPipeline {
    pub fn new(
        device: &Device,
        texture_format: TextureFormat,
        camera_layout: &BindGroupLayout,
        num_objects: usize,
    ) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[camera_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                range: 0..std::mem::size_of::<ShaderConstants>() as u32,
            }],
        });

        let mut index_list: Vec<u32> = Vec::with_capacity(TRAIL_MAX_LENGTH * 2);

        for _ in 0..2 {
            for i in 0..TRAIL_MAX_LENGTH {
                index_list.push((i * num_objects) as u32);
            }
        }

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_list),
            usage: wgpu::BufferUsages::INDEX,
        });

        let shader_module = get_or_init_shader(device);
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("line pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader_module,
                entry_point: Some("line_vs"),
                buffers: &[
                    Vertex::layout::<true, 0>(),
                    ObjectInstance::layout::<2>(),
                    Vertex::layout::<true, 4>(),
                ],
                compilation_options: PipelineCompilationOptions::default(),
            },
            cache: None,
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
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
                module: shader_module,
                entry_point: Some("line_fs"),
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

        Self {
            pipeline,
            index_buffer,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        rpass: &mut RenderPass<'_>,
        camera: &BindGroup,
        buffer: &Buffer,
        instance_buffer: &Buffer,
        push_constants: &ShaderConstants,
        index_range: Range<u32>,
        num_objects: usize,
        target_object: Option<usize>,
    ) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, buffer.slice(..));
        rpass.set_vertex_buffer(1, instance_buffer.slice(..));
        if let Some(target) = target_object {
            rpass.set_vertex_buffer(
                2,
                buffer.slice(((target * std::mem::size_of::<Vertex>()) as u64)..),
            );
        } else {
            rpass.set_vertex_buffer(2, buffer.slice(..));
        }
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        rpass.set_bind_group(0, camera, &[]);

        rpass.set_push_constants(
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(push_constants),
        );

        if target_object.is_some() {
            // re-bind the vertex buffer for each object, since we can't use base_vertex.
            for idx in 0..num_objects {
                let idxu = idx as u32;
                rpass.set_vertex_buffer(
                    0,
                    buffer.slice(((idx * std::mem::size_of::<Vertex>()) as u64)..),
                );

                rpass.draw_indexed(index_range.clone(), 0, idxu..(idxu + 1));
            }
        } else {
            for idx in 0..num_objects {
                let idxu = idx as u32;

                rpass.draw_indexed(index_range.clone(), idx as i32, idxu..(idxu + 1));
            }
        }
    }
}
