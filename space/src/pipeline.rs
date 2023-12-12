use std::ops::Range;

use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, BlendComponent, BlendFactor, BlendState, Buffer,
    PipelineLayoutDescriptor, PrimitiveState, RenderPass, RenderPipeline, RenderPipelineDescriptor,
};

use crate::{
    objects::{ObjectInstance, Vertex, TRAIL_MAX_LENGTH},
    surface::SurfaceState,
    ShaderConstants,
};

pub(crate) struct LineDrawPipeline {
    index_buffer: Buffer,
    pipeline: RenderPipeline,
}

impl LineDrawPipeline {
    pub fn new(
        surface: &SurfaceState,
        camera_layout: &BindGroupLayout,
        num_objects: usize,
    ) -> Self {
        let pipeline_layout = surface
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[camera_layout],
                push_constant_ranges: &[wgpu::PushConstantRange {
                    stages: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    range: 0..std::mem::size_of::<ShaderConstants>() as u32,
                }],
            });

        let shader = wgpu::include_spirv_raw!(env!("shaders.spv"));
        let shader_module = unsafe { surface.device.create_shader_module_spirv(&shader) };

        let color_format = surface.surface.as_ref().map_or_else(
            |_: &wgpu::CreateSurfaceError| wgpu::TextureFormat::Rgba8UnormSrgb,
            |c| c.config.format,
        );

        let pipeline = surface
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("line pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "line_vs",
                    buffers: &[Vertex::layout(), ObjectInstance::layout()],
                },
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
                    module: &shader_module,
                    entry_point: "line_fs",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
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
                }),
                multiview: None,
            });

        let mut index_list: Vec<u32> = Vec::with_capacity(TRAIL_MAX_LENGTH * 2);

        for _ in 0..2 {
            for i in 0..TRAIL_MAX_LENGTH {
                index_list.push((i * num_objects) as u32);
            }
        }

        let index_buffer = surface
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&index_list),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self {
            pipeline,
            index_buffer,
        }
    }

    pub fn draw<'a: 'b, 'b>(
        &'a self,
        rpass: &mut RenderPass<'b>,
        camera: &'b BindGroup,
        buffer: &'b Buffer,
        instance_buffer: &'b Buffer,
        push_constants: &ShaderConstants,
        index_range: Range<u32>,
        num_objects: usize,
    ) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, buffer.slice(..));
        rpass.set_vertex_buffer(1, instance_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        rpass.set_bind_group(0, &camera, &[]);

        rpass.set_push_constants(
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(push_constants),
        );

        for idx in 0..num_objects {
            let idxu = idx as u32;
            rpass.draw_indexed(index_range.clone(), idx as i32, idxu..(idxu + 1));
        }
    }
}
