use std::mem::size_of;

use cgmath::{InnerSpace, SquareMatrix, Vector3, Zero};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, BufferDescriptor, BufferUsages, Device, Queue,
};
use winit::dpi::PhysicalSize;

use crate::event_loop::KeyboardState;

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    matrix: cgmath::Matrix4<f32>,
    changed: bool,
    camera_buffer: Buffer,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl Camera {
    pub fn new(size: PhysicalSize<u32>, device: &Device) -> Self {
        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Camera buffer"),
            size: size_of::<CameraUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            eye: (0.0, 0.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: size.width as f32 / size.height as f32,
            fovy: 45.0,
            changed: true,
            matrix: cgmath::Matrix4::from_diagonal((1.0, 1.0, 1.0, 1.0).into()),
            camera_buffer,
        }
    }

    pub fn flush_if_needed(&mut self, queue: &Queue) {
        if self.changed {
            self.matrix = self.build_view_projection_matrix();
        } else {
            return;
        }

        self.changed = false;

        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&self.get_uniform_buffer().view_proj),
        );
    }

    fn get_uniform_buffer(&self) -> CameraUniform {
        CameraUniform {
            view_proj: self.matrix.into(),
        }
    }

    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        let e = 1.0 / ((self.fovy / 2.0).tan());
        let a = self.aspect;
        let epsilon = 1e-20;
        #[rustfmt::skip]
        let mut inf_proj = cgmath::Matrix4::new(
            e, 0.0, 0.0, 0.0,
            0.0, e * a, 0.0, 0.0,
            0.0, 0.0, epsilon-1.0, (epsilon - 2.0) * 0.0,
            0.0, 0.0, -1.0, 0.0);
        inf_proj.transpose_self();

        inf_proj * view
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.aspect = size.width as f32 / size.height as f32;
        self.changed = true;
    }

    pub fn bind_group_layout() -> BindGroupLayoutDescriptor<'static> {
        BindGroupLayoutDescriptor {
            label: Some("Camera buffer layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        }
    }

    pub fn create_bind_group(&self, layout: &BindGroupLayout, device: &Device) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera buffer layout"),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: self.camera_buffer.as_entire_binding(),
            }],
        })
    }

    pub fn move_relative(&mut self, keys: &KeyboardState) {
        if !keys.any_dir() {
            return;
        }

        const LOOK_REL: f32 = 0.1f32;
        let look_dir = (self.target - self.eye).normalize();
        let look_lr = self.up.cross(look_dir);

        let mut rel = Vector3::zero();
        if keys.a {
            rel += look_lr * LOOK_REL;
        }
        if keys.w {
            rel += self.up * LOOK_REL;
        }
        if keys.s {
            rel -= self.up * LOOK_REL;
        }
        if keys.d {
            rel -= look_lr * LOOK_REL;
        }
        self.target += rel;
        self.eye += rel;

        self.changed = true;
    }

    pub fn zoom(&mut self, keys: &KeyboardState) {
        if !keys.any_zoom() {
            return;
        }

        const ZOOM_REL: f32 = 0.1f32;

        let look_dir = (self.target - self.eye).normalize();

        let mut rel = Vector3::zero();
        if keys.plus {
            rel += look_dir * ZOOM_REL;
        }
        if keys.minus {
            rel -= look_dir * ZOOM_REL;
        }
        self.target += rel;
        self.eye += rel;

        self.changed = true;
    }
}
