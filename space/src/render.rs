use std::sync::OnceLock;

use bytemuck::cast_slice;
use wgpu::{
    BindGroup, Buffer, BufferDescriptor, BufferUsages, CommandEncoder, Device,
    RenderPassDescriptor, ShaderModule, TextureView,
    util::{BufferInitDescriptor, DeviceExt},
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    ShaderConstants,
    camera::Camera,
    circle_pipeline::CircleDrawPipeline,
    objects::{OBJECT_STRIDE, Objects, TRAIL_MAX_LENGTH},
    pipeline::LineDrawPipeline,
    surface::SurfaceState,
};

pub static SHADER: OnceLock<ShaderModule> = OnceLock::new();

pub fn get_or_init_shader(device: &Device) -> &ShaderModule {
    SHADER.get_or_init(|| {
        let shader = wgpu::include_spirv_raw!(env!("shaders.spv"));
        unsafe { device.create_shader_module_passthrough(shader) }
    })
}

pub struct Renderer {
    surface: SurfaceState,
    window_size: PhysicalSize<u32>,
    point_buffer: Buffer,
    instance_buffer: Buffer,
    camera_bind_group: BindGroup,
    line_pipeline: LineDrawPipeline,
    circle_pipeline: CircleDrawPipeline,
}

impl Renderer {
    pub fn new(
        surface: SurfaceState,
        window: &Window,
        camera: &Camera,
        objects: &mut Objects,
    ) -> Self {
        let instance_buffer = surface.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: cast_slice(objects.descriptions_mut()),
            usage: BufferUsages::VERTEX,
        });
        let num_objects = objects.num_objects();

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

        let circle_pipeline = CircleDrawPipeline::new(&surface, &camera_layout);

        Self {
            surface,
            window_size: window.inner_size(),
            instance_buffer,
            camera_bind_group,
            point_buffer,
            line_pipeline,
            circle_pipeline,
        }
    }

    pub fn redraw(&mut self, tick: u32, camera: &mut Camera, objects: &mut Objects) {
        let Ok(surface_with_config) = &mut self.surface.surface else {
            return;
        };

        let output = match surface_with_config.surface.get_current_texture() {
            Ok(surface) => surface,
            Err(err) => {
                // eprintln!("get_current_texture error: {err:?}");
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

        /* let epos = objects.descriptions_mut()[1].position;
        let radius = objects.descriptions_mut()[1].radius;
        let proj_epos = camera.matrix() * Vector4::from((epos[0], epos[1], epos[2], 1.0));

        println!("{:?}", proj_epos);
        println!("{}", radius / proj_epos.z); */

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
                depth_slice: None,
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

        self.circle_pipeline.draw(
            &mut rpass,
            &self.camera_bind_group,
            objects.get_last_batch_range(),
            &self.point_buffer,
            &self.instance_buffer,
            &push_constants,
            objects.num_objects(),
        );
    }
}
