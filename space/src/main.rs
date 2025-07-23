use std::sync::{Arc, atomic::AtomicBool};

use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use eframe::egui;
use egui_wgpu::{WgpuConfiguration, WgpuSetupCreateNew};
use parameters::{
    AbsoluteCoords, RelativeCoords, RelativeOrAbsolute, StandardParams, convert_params,
};
use sim::M0;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::{
    batch_request::BatchRequest,
    event_loop::{SpaceApp, run_sim_loop},
    objects::Objects,
    sim::{AU, ObjectBuffer, ObjectInfo},
    ui::SpaceEguiApp,
};

mod batch_request;
mod camera;
mod circle_pipeline;
mod event_loop;
mod objects;
mod parameters;
mod pipeline;
mod render;
mod sim;
mod surface;
mod ui;

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

#[derive(Debug)]
pub struct Object {
    dat: ObjectInfo,
    color: Vector3<f32>,
    radius: f32,
}

#[allow(unused)]
fn earth_sun_basic() -> Vec<Object> {
    vec![
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
    ]
}

#[allow(clippy::excessive_precision)] // Copy-pasted from online sources
fn earth_sun_parameter() -> Vec<Object> {
    convert_params([
        StandardParams {
            name: Some("sun".to_owned()),
            coordinates: RelativeOrAbsolute::Absolute(AbsoluteCoords {
                pos: [0.0, 0.0, 0.0],
                vel: [0.0, 0.0, 0.0],
            }),
            mass: 333000.0,
            radius: (696340e3 / AU) as f32,
            color: (1.0, 1.0, 0.0).into(),
        },
        StandardParams {
            name: Some("earth".to_owned()),
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
            name: Some("moon".to_owned()),
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
        // Give the moon a tiny satelite
        StandardParams {
            name: Some("moon-satellite".to_owned()),
            coordinates: RelativeOrAbsolute::Relative(RelativeCoords {
                parent: "moon".to_owned(),
                semi_major_axis: 1.0e7, // 100km
                eccentricity: 0.0,
                inclination: 0.0,
                arg_periapsis: 0.0,
                long_asc_node: 0.0,
                true_an: 0.0,
            }),
            // Make it fucking huge
            mass: 7.349e22 / M0,        // 1000kg
            radius: (10.0 / AU) as f32, // 10m
            color: (0.5, 0.5, 0.5).into(),
        },
    ])
    .into_iter()
    .map(|o| o.into())
    .collect()
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

    Ok(eframe::run_native(
        "space",
        options,
        Box::new(|cc| Ok(Box::new(SpaceEguiApp::new(cc, batch, objects).unwrap()))),
    )
    .map_err(|e| anyhow::anyhow!("Err: {e}"))?)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    // let window = get_window(1280.0, 640.0)?;

    let objects = earth_sun_parameter();

    println!("Running with {:?}", objects);

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
