use std::time::{Duration, Instant};

use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};
use wgpu::{Buffer, BufferDescriptor, BufferUsages};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::{
    camera::Camera,
    objects::{Objects, OBJECT_STRIDE},
    render::Renderer,
    sim::{ObjectBuffer, DELTA},
    surface::SurfaceState,
};

pub enum RuntimeEvent {
    Resize(PhysicalSize<u32>),
    Redraw(KeyboardState),
    Close,
}

#[derive(Default, Clone, Copy)]
pub struct KeyboardState {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub plus: bool,
    pub minus: bool,
}

impl KeyboardState {
    pub fn any_dir(&self) -> bool {
        self.w || self.a || self.s || self.d
    }

    pub fn any_zoom(&self) -> bool {
        self.plus || self.minus
    }
}

pub fn run_winit_loop(evt_loop: EventLoop<()>, send: Sender<RuntimeEvent>) -> anyhow::Result<()> {
    let mut next_tick = Instant::now();
    evt_loop.set_control_flow(ControlFlow::WaitUntil(next_tick));
    let next_tick_ref = &mut next_tick;

    let mut keyboard_state = KeyboardState::default();
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
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } => {
                let winit::keyboard::Key::Character(code) = &event.logical_key else {
                    return;
                };
                let is_pressed = event.state == ElementState::Pressed;
                match code.as_str() {
                    "w" => keyboard_state.w = is_pressed,
                    "a" => keyboard_state.a = is_pressed,
                    "s" => keyboard_state.s = is_pressed,
                    "d" => keyboard_state.d = is_pressed,
                    "-" => keyboard_state.minus = is_pressed,
                    "+" => keyboard_state.plus = is_pressed,
                    _ => (),
                }
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

                let _ = send.blocking_send(RuntimeEvent::Redraw(keyboard_state));
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
                let _ = send.blocking_send(RuntimeEvent::Redraw(keyboard_state));
            }
            _ => (),
        }
    })?;
    Ok(())
}

const TARGET_PER_TICK: usize = 1_000_000;
const CHECK_INTERVAL: usize = 500;

pub async fn run_event_loop(
    renderer: Renderer,
    _send: Sender<RuntimeEvent>,
    mut recv: Receiver<RuntimeEvent>,
    buffer: BufferWrapper,
    mut camera: Camera,
    mut sim: ObjectBuffer,
    mut objects: Objects,
) {
    let mut rolling_average = 0.0;
    let mut tick = 0;
    let mut i = 0;
    let mut total_ticks = 0;
    let mut state = EventLoopState::WaitingOnTick(renderer);
    let mut set_size: Option<PhysicalSize<u32>> = None;

    loop {
        let evt = loop {
            i += 1;

            sim.exec_iter();
            if i % CHECK_INTERVAL == 0 {
                match state {
                    EventLoopState::WaitingOnDraw(x) => {
                        if x.is_finished() {
                            let renderer = x.await.unwrap();
                            state = EventLoopState::WaitingOnTick(renderer);
                        } else {
                            state = EventLoopState::WaitingOnDraw(x);
                            if let Ok(evt) = recv.try_recv() {
                                break Some(evt);
                            }
                        }
                    }
                    EventLoopState::WaitingOnTick(r) => {
                        state = EventLoopState::WaitingOnTick(r);

                        if let Ok(evt) = recv.try_recv() {
                            break Some(evt);
                        }
                    }
                }
            }
            if i == TARGET_PER_TICK {
                break None;
            }
        };

        let evt = match evt {
            Some(x) => x,
            None => {
                let Some(evt) = recv.recv().await else {
                    println!("Channel dropped!");
                    break;
                };
                evt
            }
        };
        match evt {
            RuntimeEvent::Resize(size) => {
                set_size = Some(size);
                camera.resize(size);
            }
            RuntimeEvent::Redraw(e) => {
                tick += 1;

                sim.sample(&mut objects);

                camera.move_relative(&e);
                camera.zoom(&e);

                match state {
                    EventLoopState::WaitingOnDraw(x) => {
                        if x.is_finished() {
                            let mut renderer = x.await.unwrap();

                            if let Some(size) = set_size {
                                set_size = None;
                                renderer.resize(size);
                            }

                            match renderer.redraw(&buffer.buffer, tick, &mut camera, &mut objects) {
                                Ok(r) => state = EventLoopState::WaitingOnDraw(r),
                                // Failed to render for some reason, skip frame
                                Err(r) => state = EventLoopState::WaitingOnTick(r),
                            }
                        } else {
                            // Still waiting on draw, skip frame
                            state = EventLoopState::WaitingOnDraw(x)
                        }
                    }
                    EventLoopState::WaitingOnTick(mut renderer) => {
                        if let Some(size) = set_size {
                            set_size = None;
                            renderer.resize(size);
                        }
                        match renderer.redraw(&buffer.buffer, tick, &mut camera, &mut objects) {
                            Ok(r) => state = EventLoopState::WaitingOnDraw(r),
                            Err(r) => state = EventLoopState::WaitingOnTick(r),
                        }
                    }
                }

                rolling_average /= 2.0;
                rolling_average += i as f32;
                println!("{} ticks in iteration", rolling_average / 2.0);

                total_ticks += i;
                let time_passed = total_ticks as f64 * DELTA;
                // println!("{:?} days passed", time_passed / (60.0 * 60.0 * 24.0));

                i = 0;
            }
            RuntimeEvent::Close => {
                println!("Close event loop");
                break;
            }
        }
    }
    drop(buffer);
    drop(state);
    println!("Event loop terminated");
}

enum EventLoopState {
    WaitingOnDraw(JoinHandle<Renderer>),
    WaitingOnTick(Renderer),
}

impl EventLoopState {
    pub fn renderer(&mut self) -> Option<&mut Renderer> {
        match self {
            EventLoopState::WaitingOnDraw(x) => None,
            EventLoopState::WaitingOnTick(x) => Some(x),
        }
    }
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
