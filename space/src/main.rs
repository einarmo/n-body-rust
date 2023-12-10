use bytemuck::{Pod, Zeroable};
use event_loop::run_winit_loop;
use surface::{get_surface, get_window};
use tokio::runtime::Builder;

use crate::{
    camera::Camera,
    event_loop::{run_event_loop, BufferWrapper},
    render::Renderer,
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

fn main() -> anyhow::Result<()> {
    let window = get_window(1280.0, 640.0)?;
    let num_objects = 2;

    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
    let (send, recv) = tokio::sync::mpsc::channel(1);

    let task = runtime.block_on(async {
        let surface = get_surface(&window.window).await.unwrap();
        let buffer = BufferWrapper::new(num_objects, &surface);
        let camera = Camera::new(window.window.inner_size(), &surface.device);
        let renderer = Renderer::new(surface, &window.window, num_objects, &camera);

        tokio::spawn(run_event_loop(renderer, send.clone(), recv, buffer, camera))
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
