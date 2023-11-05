use std::sync::Arc;

use wgpu::{
    util::DeviceExt, Buffer, CommandEncoder, Device, PrimitiveState, Queue, RenderPassDescriptor,
    RenderPipeline, TextureView, VertexAttribute, VertexBufferLayout,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{surface::SurfaceState, ShaderConstants};

pub const TRAIL_MAX_LENGTH: usize = 100;
pub const OBJECT_STRIDE: usize = TRAIL_MAX_LENGTH * std::mem::size_of::<Vertex>();

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
}

#[derive(Clone)]
pub struct ObjectTrailInner(pub [Vertex; TRAIL_MAX_LENGTH]);

impl Default for ObjectTrailInner {
    fn default() -> Self {
        Self([Vertex::default(); TRAIL_MAX_LENGTH])
    }
}

pub struct Objects {
    buff: Vec<Vertex>,
    num_objects: usize,
    head: usize,
    tail: usize,
    pending_head: usize,
    pending_tail: usize,
}

impl Objects {
    pub fn new(num_objects: usize) -> Self {
        Self {
            buff: vec![Default::default(); num_objects * TRAIL_MAX_LENGTH],
            num_objects,
            head: 0,
            tail: 0,
            pending_head: 0,
            pending_tail: 0,
        }
    }

    fn inc_circular(head: &mut usize, tail: &mut usize, len: usize) {
        *tail = (*tail + 1) % len;
        if *tail == *head {
            *head = (*head + 1) % len;
        }
    }

    pub fn push_items(&mut self, batch: PointBatch) {
        assert!(batch.len() == self.num_objects);

        for point in batch.into_iter() {
            self.buff[self.pending_tail] = point;

            Self::inc_circular(
                &mut self.pending_head,
                &mut self.pending_tail,
                TRAIL_MAX_LENGTH * self.num_objects,
            );
        }

        Self::inc_circular(&mut self.head, &mut self.tail, TRAIL_MAX_LENGTH);
    }

    pub fn flush_to_buffer(&mut self, buffer: &Buffer, queue: &Queue) {
        let offset = (self.pending_head * std::mem::size_of::<Vertex>()) as u64;
        if self.pending_tail > self.pending_head {
            let slice = &self.buff[self.pending_head..self.pending_tail];
            queue.write_buffer(buffer, offset, bytemuck::cast_slice(slice));
        } else if self.pending_tail < self.pending_head {
            queue.write_buffer(
                buffer,
                offset,
                bytemuck::cast_slice(&self.buff[self.pending_head..]),
            );
            queue.write_buffer(
                buffer,
                0,
                bytemuck::cast_slice(&self.buff[0..self.pending_tail]),
            );
        }
        self.pending_head = self.pending_tail;
    }
}

pub struct Renderer {
    surface: SurfaceState,
    window_size: PhysicalSize<u32>,
    pipeline: RenderPipeline,
    objects: Objects,
    index_buffer: Buffer,
}

pub type PointBatch = Vec<Vertex>;

impl Renderer {
    pub fn new(surface: SurfaceState, window: &Window, num_objects: usize) -> Self {
        let shader = wgpu::include_spirv_raw!(env!("shaders.spv"));

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
                    buffers: &[VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        }],
                    }],
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
                        blend: None,
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
            objects: Objects::new(num_objects),
            index_buffer,
        }
    }

    pub fn push_point_batch(&mut self, batch: PointBatch) {
        self.objects.push_items(batch);
    }

    pub fn redraw(&mut self, buffer: &Buffer) {
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

        self.pass(&mut encoder, &mut output_view, buffer);

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

    fn pass(&self, encoder: &mut CommandEncoder, output_view: &mut TextureView, buffer: &Buffer) {
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

        let push_constants = ShaderConstants {
            width: self.window_size.width,
            height: self.window_size.height,
            time: 0.0,
        };

        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        rpass.set_push_constants(
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(&push_constants),
        );
        let head = self.objects.head as u32;
        let index_range = if self.objects.tail >= self.objects.head {
            head..(head + self.objects.tail as u32)
        } else {
            head..((TRAIL_MAX_LENGTH + self.objects.tail) as u32)
        };
        for idx in 0..(self.objects.num_objects) {
            rpass.draw_indexed(index_range.clone(), idx as i32, 0..1);
        }
    }
}
