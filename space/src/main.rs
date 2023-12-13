use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use event_loop::run_winit_loop;
use surface::{get_surface, get_window};
use tokio::runtime::Builder;

use crate::{
    camera::Camera,
    event_loop::{run_event_loop, BufferWrapper},
    objects::Objects,
    render::Renderer,
    sim::{ObjectBuffer, ObjectInfo, AU},
};

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

    let sim = ObjectBuffer::new(object_infos, 1);

    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
    let (send, recv) = tokio::sync::mpsc::channel(1);

    let task = runtime.block_on(async {
        let surface = get_surface(&window.window).await.unwrap();
        let buffer = BufferWrapper::new(num_objects, &surface);
        let camera = Camera::new(window.window.inner_size(), &surface.device);
        let renderer = Renderer::new(
            surface,
            &window.window,
            num_objects,
            &camera,
            &mut buffer_data,
        );

        tokio::spawn(run_event_loop(
            renderer,
            send.clone(),
            recv,
            buffer,
            camera,
            sim,
            buffer_data,
        ))
    });

    run_winit_loop(window.event_loop, send.clone())?;

    send.blocking_send(event_loop::RuntimeEvent::Close)?;
    println!("Wait for task completion");
    runtime.block_on(task)?;
    println!("Task completed");

    println!("Drop window");
    drop(window.window);
    println!("Window dropped");
    Ok(())
}
