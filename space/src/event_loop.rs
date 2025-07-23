use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use pollster::FutureExt;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::NamedKey,
};

use crate::{
    batch_request::BatchRequest,
    camera::Camera,
    objects::Objects,
    render::Renderer,
    sim::{ObjectBuffer, compute_elapsed_time},
    surface::{SurfaceState, WindowState, get_surface, get_window},
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
    pub j: KeyTrigger,
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

pub struct SpaceApp {
    inner: Option<SpaceAppInner>,
    size: LogicalSize<f32>,
    exchange: Arc<BatchRequest>,
    objects: Objects,
    tick: u32,
    keyboard_state: KeyboardState,
}

impl SpaceApp {
    pub fn new(init_w: f32, init_h: f32, objects: Objects, exchange: Arc<BatchRequest>) -> Self {
        Self {
            inner: None,
            size: LogicalSize::new(init_w, init_h),
            exchange,
            objects,
            tick: 0,
            keyboard_state: KeyboardState::default(),
        }
    }
}

struct SpaceAppInner {
    #[allow(unused)]
    window: WindowState,
    renderer: Renderer,
    camera: Camera,
    surface: SurfaceState,
}

impl SpaceAppInner {
    fn new(
        event_loop: &ActiveEventLoop,
        size: &LogicalSize<f32>,
        objects: &mut Objects,
    ) -> Result<Self, anyhow::Error> {
        let window = get_window(event_loop, size.width, size.height)?;
        let surface = get_surface(window.window.clone()).block_on()?;
        let camera = Camera::new(window.window.inner_size(), &surface.device);
        let renderer = Renderer::new(
            &surface.device,
            surface.texture_format(),
            window.window.inner_size(),
            &camera,
            objects,
        );

        Ok(Self {
            surface,
            window,
            renderer,
            camera,
        })
    }
}

impl ApplicationHandler<()> for SpaceApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.inner.is_none() {
            match SpaceAppInner::new(event_loop, &self.size, &mut self.objects) {
                Ok(v) => self.inner = Some(v),
                Err(e) => {
                    eprintln!("Failed to initialize app: {e}");
                    event_loop.exit();
                    return;
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested) {
            println!("The close button was pressed; stopping");
            event_loop.exit();
            return;
        }

        let Some(inner) = &mut self.inner else {
            println!("Not initialized!");
            return;
        };

        match event {
            WindowEvent::Resized(size) => {
                inner.surface.resize(size);
                inner.renderer.resize(size);
                inner.camera.resize(size);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let is_pressed = event.state == ElementState::Pressed;
                match event.logical_key {
                    winit::keyboard::Key::Named(key) => match key {
                        NamedKey::ArrowUp => self.keyboard_state.up = is_pressed,
                        NamedKey::ArrowLeft => self.keyboard_state.left = is_pressed,
                        NamedKey::ArrowDown => self.keyboard_state.down = is_pressed,
                        NamedKey::ArrowRight => self.keyboard_state.right = is_pressed,
                        NamedKey::Home => self.keyboard_state.home = is_pressed,
                        NamedKey::PageUp => self.keyboard_state.pgup = is_pressed,
                        NamedKey::Space => self.keyboard_state.space.event(is_pressed),
                        _ => (),
                    },
                    winit::keyboard::Key::Character(code) => match code.as_str() {
                        "w" => self.keyboard_state.w = is_pressed,
                        "a" => self.keyboard_state.a = is_pressed,
                        "s" => self.keyboard_state.s = is_pressed,
                        "d" => self.keyboard_state.d = is_pressed,
                        "-" => self.keyboard_state.minus = is_pressed,
                        "+" => self.keyboard_state.plus = is_pressed,
                        "f" => self.keyboard_state.f.event(is_pressed),
                        "g" => self.keyboard_state.g.event(is_pressed),
                        "h" => self.keyboard_state.h.event(is_pressed),
                        "j" => self.keyboard_state.j.event(is_pressed),
                        _ => (),
                    },
                    winit::keyboard::Key::Unidentified(_) => (),
                    winit::keyboard::Key::Dead(_) => (),
                }
            }
            WindowEvent::RedrawRequested => {
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

                self.tick += 1;

                self.exchange.sample(&mut self.objects);

                inner.camera.move_relative(&self.keyboard_state);
                inner.camera.zoom(&self.keyboard_state);
                inner
                    .camera
                    .set_focus(&mut self.keyboard_state, &mut self.objects);
                inner.camera.rot(&self.keyboard_state);
                if self.keyboard_state.space.get_trigger() {
                    self.objects.clear();
                }

                if let Some(texture) = inner.surface.get_current_texture() {
                    inner.renderer.redraw(
                        self.tick,
                        &mut inner.camera,
                        &mut self.objects,
                        &inner.surface.queue,
                        &texture.texture,
                        &inner.surface.device,
                    );
                    texture.present();
                } else {
                    println!("Failed to get current texture");
                }

                /* use cgmath::{InnerSpace, Vector3, Vector4};
                let earth = &self.objects.descriptions_mut()[1];
                let earth_pos =
                    Vector4::from((earth.position[0], earth.position[1], earth.position[2], 1.0));
                let earth_view = inner.camera.view() * earth_pos;
                let earth_proj = inner.camera.projection() * earth_view;
                println!("Earth view: {:?}", earth_view);
                println!("Earth proj {:?}", inner.camera.matrix() * earth_pos);
                println!(
                    "Earth distance from camera: {}",
                    earth_view.truncate().magnitude()
                );
                println!(
                    "Earth radius over distance: {}",
                    earth.radius / earth_view.truncate().magnitude()
                );
                // Add a point radius away from the target position, then apply the projection matrix.
                let pert_point = earth_view
                    + (earth_view
                        .truncate()
                        .cross(Vector3::new(1.0, 0.0, 0.0))
                        .normalize()
                        * earth.radius)
                        .extend(0.0);
                let pert_proj = inner.camera.projection() * pert_point;
                println!("Perturbed point: {:?}", pert_proj);
                println!(
                    "Distance to perturbed point: {}",
                    (pert_proj - earth_proj).truncate().magnitude()
                ); */

                //
                // let _last_draw = *next_tick_ref;
                // *next_tick_ref = Instant::now();

                if self.tick % 60 == 0 {
                    let sim_ticks = self.exchange.current_ticks();
                    let actual_time = compute_elapsed_time(sim_ticks);

                    println!("Elapsed time: {actual_time}");
                }
                // println!("Ticks since last: {:?}", *next_tick_ref - last_draw);

                inner.window.window.request_redraw();
            }
            _ => (),
        }
    }
}

const CHECK_INTERVAL: u64 = 500;

pub fn run_sim_loop(mut sim: ObjectBuffer, exchange: Arc<BatchRequest>, token: Arc<AtomicBool>) {
    let mut i = 0u64;

    loop {
        i += 1;

        sim.exec_iter();
        if i % CHECK_INTERVAL == 0 {
            if exchange.should_store() {
                exchange.store(&sim, i);
            } else if token.load(Ordering::Relaxed) {
                break;
            }
        }
    }
    println!("Event loop terminated");
}
