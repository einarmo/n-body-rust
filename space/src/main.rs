use std::sync::{Arc, atomic::AtomicBool};

use eframe::egui;
use egui_wgpu::{WgpuConfiguration, WgpuSetupCreateNew};

use winit::event_loop::{ControlFlow, EventLoop};

use space::{BatchRequest, Objects, SpaceApp, presets, run_sim_loop_erased, ui::SpaceEguiApp};

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

    #[allow(unused_mut)]
    let mut objects = presets::fixed_cloud(10000);
    // let mut objects = fixed_shell(100000);
    // let mut objects = earth_sun_mars();
    // objects.push(big_boy_on_collision_course());

    println!("Running with {} objects", objects.len());

    let num_objects = objects.len();

    let mut object_infos = Vec::new();
    let mut buffer_data = Objects::new(&objects);
    let descs = buffer_data.descriptions_mut();

    for (idx, obj) in objects.into_iter().enumerate() {
        object_infos.push(obj.dat);
        descs[idx].color = obj.color.into();
    }
    let batch = Arc::new(BatchRequest::new(num_objects));
    let batch_clone = batch.clone();
    let token = Arc::new(AtomicBool::new(false));
    let token_clone = token.clone();

    let handle = std::thread::spawn(|| run_sim_loop_erased(object_infos, batch_clone, token_clone));

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
