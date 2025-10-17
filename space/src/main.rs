use std::sync::{Arc, atomic::AtomicBool};

use bytemuck::{Pod, Zeroable};
use cgmath::{Point3, Vector3};
use eframe::egui;
use egui_wgpu::{WgpuConfiguration, WgpuSetupCreateNew};
use parameters::{
    AbsoluteCoords, RelativeCoords, RelativeOrAbsolute, StandardParams, convert_params,
};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::{
    batch_request::BatchRequest,
    constants::{AU, M0},
    event_loop::{SpaceApp, run_sim_loop},
    objects::Objects,
    sim::{ObjectBuffer, ObjectInfo},
    ui::SpaceEguiApp,
};

mod batch_request;
mod camera;
mod circle_pipeline;
mod constants;
mod event_loop;
mod objects;
mod parameters;
mod pipeline;
mod render;
mod sim;
mod surface;
mod ui;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C, packed)]
struct ShaderConstants {
    pub width: u32,
    pub height: u32,
    pub time: u32,
    pub total_buffer_size: u32,
    pub start_index: u32,
    pub end_index: u32,
    pub use_relative_position: u32,
    pub last_relative_position: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct Object {
    name: String,
    dat: ObjectInfo,
    color: Vector3<f32>,
    radius: f32,
}

#[allow(unused)]
fn earth_sun_basic() -> Vec<Object> {
    vec![
        Object {
            name: "sun".to_owned(),
            dat: ObjectInfo {
                pos: (0.0, 0.0, 0.0).into(),
                vel: (0.0, 1e3 / AU, 0.0).into(),
                mass: 333000.0,
            },
            color: (1.0, 1.0, 0.0).into(),
            radius: (696340e3 / AU) as f32,
        },
        Object {
            name: "earth".to_owned(),
            dat: ObjectInfo {
                pos: (1.0, 0.0, 0.0).into(),
                vel: (0.0, (29.8e3 + 1e3) / AU, 0.0).into(),
                mass: 1.0,
            },
            color: (0.0, 0.0, 1.0).into(),
            radius: (6371e3 / AU) as f32,
        },
    ]
}

fn earth_sun_mars_params() -> Vec<StandardParams> {
    vec![
        StandardParams {
            name: "sun".to_owned(),
            coordinates: RelativeOrAbsolute::Absolute(AbsoluteCoords {
                pos: [0.0, 0.0, 0.0],
                vel: [0.0, 0.0, 0.0],
            }),
            mass: 333000.0,
            radius: (696340e3 / AU) as f32,
            color: (1.0, 1.0, 0.0).into(),
        },
        StandardParams {
            name: "earth".to_owned(),
            coordinates: RelativeOrAbsolute::Relative(RelativeCoords {
                parent: "sun".to_owned(),
                semi_major_axis: 1.495365477412831E+08 * 1e3,
                eccentricity: 1.639588231990315E-02,
                inclination: 3.670030330713475E-03,
                arg_periapsis: 2.557573855355361E+02,
                long_asc_node: 2.087400227953831E+02,
                true_an: 3.450278328909303E+02,
            }),
            /* coordinates: RelativeOrAbsolute::Absolute(AbsoluteCoords {
                pos: [0.0, 0.0, 0.0],
                vel: [0.0, 0.0, 0.0],
            }), */
            mass: 1.0,
            radius: (6371e3 / AU) as f32,
            color: (0.0, 0.0, 1.0).into(),
        },
        StandardParams {
            name: "moon".to_owned(),
            coordinates: RelativeOrAbsolute::Relative(RelativeCoords {
                parent: "earth".to_owned(),
                semi_major_axis: 3.815880763110870E+05 * 1e3,
                eccentricity: 3.179523012872624E-02,
                inclination: 5.064604179512905E+00,
                arg_periapsis: 3.012277898101174E+02,
                long_asc_node: 2.229402837659016E+01,
                true_an: 6.454243862420770E+01,
            }),
            mass: 7.349e22 / M0,
            radius: (1737e3 / AU) as f32,
            color: (1.0, 1.0, 1.0).into(),
        },
        StandardParams {
            name: "mars".to_owned(),
            coordinates: RelativeOrAbsolute::Relative(RelativeCoords {
                parent: "sun".to_owned(),
                semi_major_axis: 227956E+6,
                eccentricity: 0.0935,
                inclination: 1.848,
                arg_periapsis: 286.5,
                long_asc_node: 49.578,
                true_an: 0.0, // TOOD
            }),
            mass: 0.107,
            radius: (3396.2e3 / AU) as f32,
            color: (1.0, 0.0, 0.0).into(),
        },
    ]
}

#[allow(clippy::excessive_precision, unused)] // Copy-pasted from online sources
fn earth_sun_mars() -> Vec<Object> {
    convert_params(earth_sun_mars_params())
        .into_iter()
        .map(|o| o.into())
        .collect()
}

fn big_boy_on_collision_course() -> Object {
    Object {
        name: "big_boy".to_owned(),
        dat: ObjectInfo {
            pos: (3.0, 0.0, 0.0).into(),
            vel: (-0.5e5 / AU, -0.2e5 / AU, 0.0).into(),
            mass: 100000.0,
        },
        color: (0.0, 1.0, 0.0).into(),
        radius: (1e6 / AU) as f32,
    }
}

fn earth_sun_mars_ast() -> Vec<Object> {
    let mut objs = earth_sun_mars_params();
    objs.append(&mut asteroid_belt(10000));
    convert_params(objs).into_iter().map(|o| o.into()).collect()
}

fn asteroid_belt(n_asteroids: usize) -> Vec<StandardParams> {
    let mut objs = Vec::new();
    for i in 0..n_asteroids {
        let col = 0.5 + rand::random_range(-0.2..0.2);
        objs.push(StandardParams {
            name: format!("asteroid_{i}"),
            coordinates: RelativeOrAbsolute::Relative(RelativeCoords {
                parent: "sun".to_owned(),
                semi_major_axis: 300000E+6 + rand::random_range(-1.0..1.0) * 25_000E+6,
                eccentricity: rand::random_range(0.0..0.15),
                inclination: rand::random_range(0.0..10.0),
                arg_periapsis: rand::random_range(0.0..360.0),
                long_asc_node: rand::random_range(0.0..360.0),
                true_an: rand::random_range(0.0..360.0),
            }),
            mass: rand::random_range(1e-10..1e-6),
            radius: rand::random_range((1e3 / AU)..(1e6 / AU)) as f32,
            color: (col, col, col).into(),
        });
    }
    objs
}

fn random_cloud(n_objects: usize) -> Vec<Object> {
    let mut objs = Vec::new();
    for i in 0..n_objects {
        let col = 0.5 + rand::random_range(-0.2..0.2);
        objs.push(Object {
            name: format!("particle_{i}"),
            dat: ObjectInfo {
                pos: Point3::new(
                    rand::random_range(-1e1..1e1),
                    rand::random_range(-1e1..1e1),
                    rand::random_range(-1e1..1e1),
                ),
                vel: Vector3::new(
                    rand::random_range(-1e3..1e3) / AU,
                    rand::random_range(-1e3..1e3) / AU,
                    rand::random_range(-1e3..1e3) / AU,
                ),
                mass: rand::random_range(1000.0..1000000.0),
            },
            color: (col, col, col).into(),
            radius: rand::random_range((1e3 / AU)..(1e6 / AU)) as f32,
        });
    }
    objs
}

fn graphics_direct(batch: Arc<BatchRequest>, objects: Objects) -> anyhow::Result<()> {
    let mut app = SpaceApp::new(1280.0, 640.0, objects, batch);

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop.run_app(&mut app)?;

    Ok(())
}

fn graphics_egui(batch: Arc<BatchRequest>, objects: Objects) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 1024.0])
            .with_drag_and_drop(true),

        renderer: eframe::Renderer::Wgpu,
        wgpu_options: WgpuConfiguration {
            wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(WgpuSetupCreateNew {
                device_descriptor: Arc::new(|_| wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::PUSH_CONSTANTS
                        | wgpu::Features::SPIRV_SHADER_PASSTHROUGH
                        | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                    required_limits: wgpu::Limits {
                        max_push_constant_size: 128,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "space",
        options,
        Box::new(|cc| Ok(Box::new(SpaceEguiApp::new(cc, batch, objects).unwrap()))),
    )
    .map_err(|e| anyhow::anyhow!("Err: {e}"))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    // let window = get_window(1280.0, 640.0)?;

    let mut objects = random_cloud(10000);
    // objects.push(big_boy_on_collision_course());

    println!("Running with {objects:?}");

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

    let handle = std::thread::spawn(|| run_sim_loop(sim, batch_clone, token_clone));

    let egui = true;
    if egui {
        graphics_egui(batch, buffer_data)?;
    } else {
        graphics_direct(batch, buffer_data)?;
    }

    token.store(true, std::sync::atomic::Ordering::Relaxed);
    println!("Wait for task completion");
    handle.join().unwrap();
    println!("Task completed");
    Ok(())
}
