use bytemuck::{Pod, Zeroable};
use surface::{get_surface, get_window};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::render::Renderer;

mod pipeline;
mod render;
mod surface;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct ShaderConstants {
    pub width: u32,
    pub height: u32,
    pub time: f32,
}

async fn run(window: &Window, event_loop: EventLoop<()>) -> anyhow::Result<()> {
    let mut surface = get_surface(&window).await?;

    let mut renderer = Renderer::new(&mut surface, &window, 1);

    event_loop.set_control_flow(ControlFlow::Wait);

    renderer.write_to_buffer().await;

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                elwt.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                renderer.resize(size);
            }
            Event::AboutToWait => {
                // Application update code.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw, in
                // applications which do not always need to. Applications that redraw continuously
                // can just render here instead.
                // window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                renderer.redraw();
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.
            }
            _ => (),
        }
    })?;
    drop(surface);
    println!("Dropped surface");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let window = get_window(1280.0, 640.0)?;

    futures::executor::block_on(run(&window.window, window.event_loop)).unwrap();
    println!("Exited run...");
    drop(window.window);
    println!("Dropped window");
    Ok(())
}
