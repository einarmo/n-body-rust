use std::sync::{atomic::AtomicBool, Arc};

use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use event_loop::run_winit_loop;
use pollster::FutureExt;
use surface::{get_surface, get_window};

use crate::{
    batch_request::BatchRequest,
    camera::Camera,
    event_loop::run_sim_loop,
    objects::Objects,
    render::Renderer,
    sim::{ObjectBuffer, ObjectInfo, AU},
};

mod batch_request;
mod camera;
mod event_loop;
mod objects;
mod pipeline;
mod render;
mod sim;
mod surface;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct ShaderConstants {
    pub width: u32,
    pub height: u32,
    pub time: u32,
    pub total_buffer_size: u32,
    pub start_index: u32,
    pub end_index: u32,
}

pub struct Object {
    dat: ObjectInfo,
    color: Vector3<f32>,
    radius: f32,
}

fn main() -> anyhow::Result<()> {
    let window = get_window(1280.0, 640.0)?;
    let objects = vec![
        Object {
            dat: ObjectInfo {
                pos: (0.0, 0.0, 0.0).into(),
                vel: (0.0, 1e3 / AU, 0.0).into(),
                mass: 333000.0,
            },
            color: (1.0, 1.0, 0.0).into(),
            radius: (696340e3 / AU) as f32,
        },
        Object {
            dat: ObjectInfo {
                pos: (1.0, 0.0, 0.0).into(),
                vel: (0.0, (29.8e3 + 1e3) / AU, 0.0).into(),
                mass: 1.0,
            },
            color: (0.0, 0.0, 1.0).into(),
            radius: (6371e3 / AU) as f32,
        },
    ];

    let num_objects = objects.len();

    let mut object_infos = Vec::new();
    let mut buffer_data = Objects::new(&objects);
    let descs = buffer_data.descriptions_mut();

    for (idx, obj) in objects.into_iter().enumerate() {
        object_infos.push(obj.dat);
        descs[idx].color = obj.color.into();
    }

    let sim = ObjectBuffer::new(object_infos);

    let batch = Arc::new(BatchRequest::new(num_objects));
    let batch_clone = batch.clone();
    let token = Arc::new(AtomicBool::new(false));
    let token_clone = token.clone();

    let surface = get_surface(&window.window).block_on()?;
    let camera = Camera::new(window.window.inner_size(), &surface.device);
    let renderer = Renderer::new(
        surface,
        &window.window,
        num_objects,
        &camera,
        &mut buffer_data,
    );

    let handle = std::thread::spawn(|| run_sim_loop(sim, batch_clone, token_clone));

    run_winit_loop(window.event_loop, renderer, camera, batch, buffer_data)?;

    token.store(true, std::sync::atomic::Ordering::Relaxed);
    println!("Wait for task completion");
    handle.join().unwrap();
    println!("Task completed");

    println!("Drop window");
    drop(window.window);
    println!("Window dropped");
    Ok(())
}
