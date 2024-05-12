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
    keyboard::NamedKey,
};

use crate::{
    batch_request::BatchRequest, camera::Camera, objects::Objects, render::Renderer,
    sim::ObjectBuffer,
};

#[derive(Debug, Default, Clone)]
pub struct KeyTrigger {
    pressed: bool,
    trigger: bool,
}

impl KeyTrigger {
    pub fn event(&mut self, is_pressed: bool) {
        match (self.pressed, is_pressed) {
            (true, true) => (),
            (true, false) => self.pressed = false,
            (false, true) => {
                self.pressed = true;
                self.trigger = true;
            }
            (false, false) => (),
        }
    }

    pub fn get_trigger(&mut self) -> bool {
        let t = self.trigger;
        self.trigger = false;
        t
    }
}

#[derive(Default, Clone)]
pub struct KeyboardState {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub up: bool,
    pub left: bool,
    pub down: bool,
    pub right: bool,
    pub home: bool,
    pub pgup: bool,
    pub plus: bool,
    pub minus: bool,
    pub f: KeyTrigger,
    pub g: KeyTrigger,
    pub h: KeyTrigger,
    pub space: KeyTrigger,
}

impl KeyboardState {
    pub fn any_dir(&self) -> bool {
        self.w || self.a || self.s || self.d
    }

    pub fn any_zoom(&self) -> bool {
        self.plus || self.minus
    }

    pub fn any_rot(&self) -> bool {
        self.up || self.down || self.right || self.left || self.home || self.pgup
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
                let is_pressed = event.state == ElementState::Pressed;
                match event.logical_key {
                    winit::keyboard::Key::Named(key) => match key {
                        NamedKey::ArrowUp => keyboard_state.up = is_pressed,
                        NamedKey::ArrowLeft => keyboard_state.left = is_pressed,
                        NamedKey::ArrowDown => keyboard_state.down = is_pressed,
                        NamedKey::ArrowRight => keyboard_state.right = is_pressed,
                        NamedKey::Home => keyboard_state.home = is_pressed,
                        NamedKey::PageUp => keyboard_state.pgup = is_pressed,
                        NamedKey::Space => keyboard_state.space.event(is_pressed),
                        _ => (),
                    },
                    winit::keyboard::Key::Character(code) => match code.as_str() {
                        "w" => keyboard_state.w = is_pressed,
                        "a" => keyboard_state.a = is_pressed,
                        "s" => keyboard_state.s = is_pressed,
                        "d" => keyboard_state.d = is_pressed,
                        "-" => keyboard_state.minus = is_pressed,
                        "+" => keyboard_state.plus = is_pressed,
                        "f" => keyboard_state.f.event(is_pressed),
                        "g" => keyboard_state.g.event(is_pressed),
                        "h" => keyboard_state.h.event(is_pressed),
                        _ => (),
                    },
                    winit::keyboard::Key::Unidentified(_) => (),
                    winit::keyboard::Key::Dead(_) => (),
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
                camera.set_focus(&mut keyboard_state, &objects);
                camera.rot(&keyboard_state);
                if keyboard_state.space.get_trigger() {
                    objects.clear();
                }

                renderer.redraw(tick, &mut camera, &mut objects);

                let _last_draw = *next_tick_ref;
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
