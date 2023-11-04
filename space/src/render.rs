use futures::channel::oneshot;
use wgpu::{
    Buffer, BufferDescriptor, BufferSlice, BufferUsages, CommandEncoder, PrimitiveState,
    RenderPassDescriptor, RenderPipeline, TextureView, VertexAttribute, VertexBufferLayout,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{surface::SurfaceState, ShaderConstants};

const TRAIL_MAX_LENGTH: usize = 10_000;
const OBJECT_STRIDE: usize = TRAIL_MAX_LENGTH * std::mem::size_of::<Vertex>();

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pos: [f32; 3],
}

#[derive(Clone)]
pub struct ObjectTrailInner(pub [Vertex; TRAIL_MAX_LENGTH]);

impl Default for ObjectTrailInner {
    fn default() -> Self {
        Self([Vertex::default(); TRAIL_MAX_LENGTH])
    }
}

#[derive(Clone, Default)]
pub struct ObjectTrail {
    points: Box<ObjectTrailInner>,
    head: usize,
    tail: usize,
}

pub struct Renderer<'a> {
    surface: &'a mut SurfaceState,
    window: &'a Window,
    pipeline: RenderPipeline,
    objects: Vec<ObjectTrail>,
    buffer: Buffer,
}

impl<'a> Renderer<'a> {
    pub fn new(surface: &'a mut SurfaceState, window: &'a Window, num_objects: usize) -> Self {
        let buffer = surface.device.create_buffer(&BufferDescriptor {
            label: Some("pos_buffer"),
            size: (num_objects * OBJECT_STRIDE) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::MAP_WRITE,
            mapped_at_creation: false,
        });

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

        Self {
            surface,
            buffer,
            window,
            pipeline,
            objects: vec![Default::default(); num_objects],
        }
    }

    pub async fn write_to_buffer(&mut self) {
        let (send, recv) = oneshot::channel();
        let slice = self.buffer.slice(..);
        self.map_buffer(&slice, send);
        println!("Wait for buffer to map...");
        self.surface.device.poll(wgpu::MaintainBase::Wait);
        println!("Wait for buffer to map...");

        let ok = recv.await.unwrap();
        println!("Buffer mapped");
        if !ok {
            return;
        }

        let obj = self.objects.get_mut(0).unwrap();
        obj.points.0[0] = Vertex {
            pos: [0.0, 0.0, 0.0],
        };
        obj.points.0[1] = Vertex {
            pos: [1.0, 1.0, 0.0],
        };
        obj.points.0[2] = Vertex {
            pos: [1.0, 0.0, 0.0],
        };
        obj.points.0[3] = Vertex {
            pos: [0.0, 1.0, 0.0],
        };
        obj.tail = 2;

        let mut view = slice.get_mapped_range_mut();
        for (idx, obj) in self.objects.iter_mut().enumerate() {
            let start = idx * OBJECT_STRIDE;
            let end = start + OBJECT_STRIDE;

            let slice = bytemuck::cast_slice(&obj.points.0[..]);
            view[start..end].copy_from_slice(slice);
        }
        drop(view);
        self.buffer.unmap();
    }

    pub fn redraw(&mut self) {
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

        let mut output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .surface
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.pass(&mut encoder, &mut output_view);

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
        }
    }

    fn pass(&self, encoder: &mut CommandEncoder, output_view: &mut TextureView) {
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
            width: self.window.inner_size().width,
            height: self.window.inner_size().height,
            time: 0.0,
        };

        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.buffer.slice(..));
        rpass.set_push_constants(
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(&push_constants),
        );
        for (idx, obj) in self.objects.iter().enumerate() {
            if obj.tail > obj.head {
                let offset = (idx * TRAIL_MAX_LENGTH) as u32;
                rpass.draw(offset..(offset + obj.tail as u32), 0..1);
            }
        }
    }

    fn map_buffer(&self, slice: &BufferSlice<'_>, cb: oneshot::Sender<bool>) {
        slice.map_async(wgpu::MapMode::Write, |r| {
            let _ = cb.send(r.is_ok());
        })
    }

    // Assumes buffer is mapped
    fn flush_to_buffer(&mut self, buffer: BufferSlice<'_>) {
        let mut view = buffer.get_mapped_range_mut();
        for (idx, obj) in self.objects.iter_mut().enumerate() {
            let start = idx * OBJECT_STRIDE;
            let end = start + OBJECT_STRIDE;

            view[start..end].copy_from_slice(bytemuck::cast_slice(&obj.points.0[0..1]));
        }
        self.buffer.unmap();
    }
}
