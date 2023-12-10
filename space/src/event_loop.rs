use std::time::{Duration, Instant};

use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};
use wgpu::{Buffer, BufferDescriptor, BufferUsages};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::{
    render::{Renderer, OBJECT_STRIDE, TRAIL_MAX_LENGTH},
    surface::SurfaceState,
};

pub enum RuntimeEvent {
    Resize(PhysicalSize<u32>),
    Redraw,
    Close,
}

pub fn run_winit_loop(evt_loop: EventLoop<()>, send: Sender<RuntimeEvent>) -> anyhow::Result<()> {
    let mut next_tick = Instant::now();
    evt_loop.set_control_flow(ControlFlow::WaitUntil(next_tick));
    let next_tick_ref = &mut next_tick;
    evt_loop.run(move |event, elwt| {
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
                let _ = send.blocking_send(RuntimeEvent::Resize(size));
            }
            Event::AboutToWait => {
                // Application update code.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw, in
                // applications which do not always need to. Applications that redraw continuously
                // can just render here instead.
                // window.request_redraw();
                *next_tick_ref = *next_tick_ref + Duration::from_millis(1000 / 60);
                if Instant::now() > *next_tick_ref {
                    *next_tick_ref = Instant::now() + Duration::from_millis(1000 / 60);
                }
                elwt.set_control_flow(ControlFlow::WaitUntil(*next_tick_ref));

                let _ = send.blocking_send(RuntimeEvent::Redraw);
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.
                let _ = send.blocking_send(RuntimeEvent::Redraw);
            }
            _ => (),
        }
    })?;
    Ok(())
}

pub async fn run_event_loop(
    mut renderer: Renderer,
    send: Sender<RuntimeEvent>,
    mut recv: Receiver<RuntimeEvent>,
    buffer: BufferWrapper,
) {
    let mut tick = 0;
    loop {
        select! {
            evt = recv.recv() => {
                let Some(evt) = evt else {
                    println!("Channel dropped!");
                    break;
                };

                match evt {
                    RuntimeEvent::Resize(size) => {
                        renderer.resize(size);
                    },
                    RuntimeEvent::Redraw => {
                        tick += 1;
                        let r = 1.0 - ((tick as f32) / (TRAIL_MAX_LENGTH as f32));
                        renderer.push_point_batch(vec![
                            [(tick as f32 / 10.0).sin() * r, (tick as f32 / 10.0).cos() * r, 0.0],
                            [(tick as f32 / 10.0).cos() * r, (tick as f32 / 10.0).sin() * r, 0.0]
                        ]);
                        renderer.redraw(&buffer.buffer, tick);
                    },
                    RuntimeEvent::Close => {
                        println!("Close event loop");
                        break;
                    }
                }
            }
        }
    }
    drop(buffer);
    drop(renderer);
    println!("Event loop terminated");
}

pub struct BufferWrapper {
    buffer: Buffer,
}

impl BufferWrapper {
    pub fn new(num_objects: usize, surface: &SurfaceState) -> Self {
        let buffer = surface.device.create_buffer(&BufferDescriptor {
            label: Some("pos_buffer"),
            size: (num_objects * OBJECT_STRIDE) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self { buffer }
    }
}
