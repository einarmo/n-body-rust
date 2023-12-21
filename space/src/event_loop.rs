use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::{
    batch_request::BatchRequest, camera::Camera, objects::Objects, render::Renderer,
    sim::ObjectBuffer,
};

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

pub fn run_winit_loop(
    evt_loop: EventLoop<()>,
    mut renderer: Renderer,
    mut camera: Camera,
    exchange: Arc<BatchRequest>,
    mut objects: Objects,
) -> anyhow::Result<()> {
    let mut next_tick = Instant::now();
    evt_loop.set_control_flow(ControlFlow::Poll);
    let next_tick_ref = &mut next_tick;

    let mut keyboard_state = KeyboardState::default();
    let mut tick = 0;
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
                renderer.resize(size);
                camera.resize(size);
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
                //*next_tick_ref = *next_tick_ref + Duration::from_millis(1000 / 60);
                //if Instant::now() > *next_tick_ref {
                //    *next_tick_ref = Instant::now() + Duration::from_millis(1000 / 60);
                //}
                //elwt.set_control_flow(ControlFlow::WaitUntil(*next_tick_ref));

                tick += 1;

                exchange.sample(&mut objects);

                camera.move_relative(&keyboard_state);
                camera.zoom(&keyboard_state);

                renderer.redraw(tick, &mut camera, &mut objects);

                let last_draw = next_tick_ref.clone();
                *next_tick_ref = Instant::now();
                // println!("Ticks since last: {:?}", *next_tick_ref - last_draw);
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
            }
            _ => (),
        }
    })?;
    Ok(())
}

const CHECK_INTERVAL: usize = 500;

pub fn run_sim_loop(mut sim: ObjectBuffer, exchange: Arc<BatchRequest>, token: Arc<AtomicBool>) {
    let mut i = 0;

    loop {
        i += 1;

        sim.exec_iter();
        if i % CHECK_INTERVAL == 0 {
            if exchange.should_store() {
                exchange.store(&sim);
                // println!("Iterations since last sample: {}", i);
                i = 0;
            } else if token.load(Ordering::Relaxed) {
                break;
            }
        }
    }
    println!("Event loop terminated");
}
