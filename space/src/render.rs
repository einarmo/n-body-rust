use std::sync::Arc;

use bytemuck::cast_slice;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BlendComponent, BlendFactor, BlendState, Buffer, BufferUsages, CommandEncoder, Device,
    PrimitiveState, RenderPassDescriptor, RenderPipeline, TextureView,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    objects::{ObjectInstance, Objects, PointBatch, Vertex, TRAIL_MAX_LENGTH},
    surface::SurfaceState,
    ShaderConstants,
};

pub struct Renderer {
    surface: SurfaceState,
    window_size: PhysicalSize<u32>,
    pipeline: RenderPipeline,
    objects: Objects,
    index_buffer: Buffer,
    instance_buffer: Buffer,
}

impl Renderer {
    pub fn new(surface: SurfaceState, window: &Window, num_objects: usize) -> Self {
        let shader = wgpu::include_spirv_raw!(env!("shaders.spv"));

        /* let color_buffer = surface.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Color buffer"),
            contents: bytemuck::cast_slice(&[1.0f32, 1.0f32, 1.0f32, 1.0f32, 0.0f32, 0.0f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let color_bind_group_layout =
            surface
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Color buffer layout"),
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let color_bind_group = surface.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Color buffer bind group"),
            layout: &color_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: color_buffer.as_entire_binding(),
            }],
        }); */

        let mut objects = Objects::new(num_objects);
        objects.descriptions_mut()[0].color = [1.0, 0.0, 0.0];
        objects.descriptions_mut()[1].color = [1.0, 1.0, 0.0];

        let instance_buffer = surface.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: cast_slice(objects.descriptions_mut()),
            usage: BufferUsages::VERTEX,
        });

        let pipeline_layout =
            surface
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        range: 0..std::mem::size_of::<ShaderConstants>() as u32,
                    }],
                });

        let shader_module = unsafe { surface.device.create_shader_module_spirv(&shader) };

        let color_format = surface.surface.as_ref().map_or_else(
            |_: &wgpu::CreateSurfaceError| wgpu::TextureFormat::Rgba8UnormSrgb,
            |c| c.config.format,
        );

        let pipeline = surface
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "main_vs",
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
                    entry_point: "main_fs",
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
            surface,
            window_size: window.inner_size(),
            pipeline,
            objects,
            index_buffer,
            instance_buffer,
        }
    }

    pub fn push_point_batch(&mut self, batch: PointBatch) {
        self.objects.push_items(batch);
    }

    pub fn redraw(&mut self, buffer: &Buffer, tick: u32) {
        let Ok(surface_with_config) = &mut self.surface.surface else {
            return;
        };

        let output = match surface_with_config.surface.get_current_texture() {
            Ok(surface) => surface,
            Err(err) => {
                eprintln!("get_current_texture error: {err:?}");
                match err {
                    wgpu::SurfaceError::Lost => {
                        surface_with_config.configure(&self.surface.device);
                    }
                    wgpu::SurfaceError::OutOfMemory => {
                        println!("Out of memory!");
                        return;
                    }
                    _ => (),
                }
                return;
            }
        };

        self.objects.flush_to_buffer(&buffer, &self.surface.queue);

        let mut output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .surface
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.pass(&mut encoder, &mut output_view, buffer, tick);

        self.surface.queue.submit(Some(encoder.finish()));

        output.present();
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width != 0 && size.height != 0 {
            // Recreate the swap chain with the new size
            if let Ok(surface_with_config) = &mut self.surface.surface {
                surface_with_config.config.width = size.width;
                surface_with_config.config.height = size.height;
                surface_with_config.configure(&self.surface.device);
            }
            self.window_size = size;
        }
    }

    pub fn device(&self) -> Arc<Device> {
        self.surface.device.clone()
    }

    fn pass(
        &self,
        encoder: &mut CommandEncoder,
        output_view: &mut TextureView,
        buffer: &Buffer,
        tick: u32,
    ) {
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        let index_range = self.objects.get_index_range();

        let push_constants = ShaderConstants {
            width: self.window_size.width,
            height: self.window_size.height,
            time: tick,
            total_buffer_size: TRAIL_MAX_LENGTH as u32,
            start_index: index_range.start,
            end_index: index_range.end,
        };

        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, buffer.slice(..));
        rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        rpass.set_push_constants(
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(&push_constants),
        );

        for idx in 0..(self.objects.num_objects()) {
            let idxu = idx as u32;
            rpass.draw_indexed(index_range.clone(), idx as i32, idxu..(idxu + 1));
        }
    }
}
