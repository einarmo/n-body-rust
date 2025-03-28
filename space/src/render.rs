use bytemuck::cast_slice;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, Buffer, BufferDescriptor, BufferUsages, CommandEncoder, RenderPassDescriptor,
    TextureView,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    camera::Camera,
    objects::{Objects, OBJECT_STRIDE, TRAIL_MAX_LENGTH},
    pipeline::LineDrawPipeline,
    surface::SurfaceState,
    ShaderConstants,
};

pub struct Renderer {
    surface: SurfaceState,
    window_size: PhysicalSize<u32>,
    point_buffer: Buffer,
    instance_buffer: Buffer,
    camera_bind_group: BindGroup,
    line_pipeline: LineDrawPipeline,
}

impl Renderer {
    pub fn new(
        surface: SurfaceState,
        window: &Window,
        num_objects: usize,
        camera: &Camera,
        objects: &mut Objects,
    ) -> Self {
        let instance_buffer = surface.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: cast_slice(objects.descriptions_mut()),
            usage: BufferUsages::VERTEX,
        });

        let camera_layout = surface
            .device
            .create_bind_group_layout(&Camera::bind_group_layout());
        let camera_bind_group = camera.create_bind_group(&camera_layout, &surface.device);

        let line_pipeline = LineDrawPipeline::new(&surface, &camera_layout, num_objects);

        let point_buffer = surface.device.create_buffer(&BufferDescriptor {
            label: Some("pos_buffer"),
            size: (num_objects * OBJECT_STRIDE) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            surface,
            window_size: window.inner_size(),
            instance_buffer,
            camera_bind_group,
            point_buffer,
            line_pipeline,
        }
    }

    pub fn redraw(&mut self, tick: u32, camera: &mut Camera, objects: &mut Objects) {
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
        objects.flush_to_buffer(&self.point_buffer, &self.surface.queue);
        camera.flush_if_needed(&self.surface.queue);

        let mut output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .surface
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.pass(&mut encoder, &mut output_view, tick, objects);

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

    fn pass(
        &self,
        encoder: &mut CommandEncoder,
        output_view: &mut TextureView,
        tick: u32,
        objects: &Objects,
    ) {
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        let index_range = objects.get_index_range();

        let push_constants = ShaderConstants {
            width: self.window_size.width,
            height: self.window_size.height,
            time: tick,
            total_buffer_size: TRAIL_MAX_LENGTH as u32,
            start_index: index_range.start,
            end_index: index_range.end,
        };

        self.line_pipeline.draw(
            &mut rpass,
            &self.camera_bind_group,
            &self.point_buffer,
            &self.instance_buffer,
            &push_constants,
            index_range,
            objects.num_objects(),
        );
    }
}
