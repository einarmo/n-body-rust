use std::mem::size_of;

use cgmath::{InnerSpace, Rad, SquareMatrix, Vector3, Zero};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, BufferDescriptor, BufferUsages, Device, Queue,
};
use winit::dpi::PhysicalSize;

use crate::{event_loop::KeyboardState, objects::Objects};

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    focus: Option<i64>,
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
            focus: None,
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

    pub fn set_focus(&mut self, keys: &mut KeyboardState, objects: &Objects) {
        if keys.f.get_trigger() {
            self.focus =
                Some((self.focus.unwrap_or(1) - 1).rem_euclid(objects.num_objects() as i64));
        }
        if keys.g.get_trigger() {
            self.focus =
                Some((self.focus.unwrap_or(-1) + 1).rem_euclid(objects.num_objects() as i64));
        }
        if keys.h.get_trigger() {
            self.focus = None;
        }

        if let Some(focus) = &self.focus {
            let pos = objects.position_of(*focus as usize);
            let rel = self.eye - self.target;
            self.target.x = pos[0];
            self.target.y = pos[1];
            self.target.z = pos[2];
            self.eye = self.target + rel;
            self.changed = true;
        }
    }

    pub fn zoom(&mut self, keys: &KeyboardState) {
        if !keys.any_zoom() {
            return;
        }

        let look = self.target - self.eye;
        let look_dir = look.normalize();
        let look_mag = look.magnitude();
        let zoom_rel = look_mag / 10.0;

        let mut rel = Vector3::zero();
        if keys.plus {
            rel += look_dir * zoom_rel;
        }
        if keys.minus {
            rel -= look_dir * zoom_rel;
        }
        self.eye += rel;

        self.changed = true;
    }

    pub fn rot(&mut self, keys: &KeyboardState) {
        if !keys.any_rot() {
            return;
        }

        // Do not precompute any vectors, since they might change if multiple keys are held
        // at the same time.

        if keys.home {
            let look = self.target - self.eye;
            let look_dir = look.normalize();
            let rot = cgmath::Matrix3::from_axis_angle(look_dir, Rad(0.02));
            self.up = rot * self.up;
        }
        if keys.pgup {
            let look = self.target - self.eye;
            let look_dir = look.normalize();
            let rot = cgmath::Matrix3::from_axis_angle(look_dir, Rad(-0.02));
            self.up = rot * self.up;
        }

        if keys.up {
            let look = self.target - self.eye;
            let look_dir = look.normalize();
            // Rotate the inverse look vector around the perpendicular up vector
            let look_perp = look_dir.cross(self.up);
            let rot = cgmath::Matrix3::from_axis_angle(look_perp, Rad(0.02));
            let new_rel = rot * (-look);

            self.eye = self.target + new_rel;
            self.up = rot * self.up;
        }
        if keys.down {
            let look = self.target - self.eye;
            let look_dir = look.normalize();
            let look_perp = look_dir.cross(self.up);
            let rot = cgmath::Matrix3::from_axis_angle(look_perp, Rad(-0.02));
            let new_rel = rot * (-look);

            self.eye = self.target + new_rel;
            self.up = rot * self.up;
        }

        if keys.left {
            let look = self.target - self.eye;
            let rot = cgmath::Matrix3::from_axis_angle(self.up, Rad(-0.02));
            let new_rel = rot * (-look);

            self.eye = self.target + new_rel;
        }
        if keys.right {
            let look = self.target - self.eye;
            let rot = cgmath::Matrix3::from_axis_angle(self.up, Rad(0.02));
            let new_rel = rot * (-look);

            self.eye = self.target + new_rel;
        }

        self.changed = true;
    }
}
