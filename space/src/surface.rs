use std::sync::Arc;

use anyhow::anyhow;
use wgpu::{Adapter, CreateSurfaceError, Device, Queue, Surface, SurfaceConfiguration};
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub struct WindowState {
    pub event_loop: EventLoop<()>,
    pub window: Window,
}

pub fn get_window(init_w: f32, init_h: f32) -> anyhow::Result<WindowState> {
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Rust GPU - wgpu")
        .with_inner_size(winit::dpi::LogicalSize::new(init_w, init_h))
        .build(&event_loop)?;

    Ok(WindowState { event_loop, window })
}

pub struct SurfaceState {
    pub surface: Result<SurfaceWithConfig, CreateSurfaceError>,
    pub adapter: Adapter,
    pub device: Arc<Device>,
    pub queue: Queue,
}

impl SurfaceWithConfig {
    pub fn configure(&mut self, device: &Device) {
        self.surface.configure(device, &self.config);
    }
}

pub async fn get_surface(window: &Window) -> anyhow::Result<SurfaceState> {
    let backends = wgpu::util::backend_bits_from_env().unwrap_or(wgpu::Backends::VULKAN);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });

    let surface = unsafe { instance.create_surface(&window) };
    let adapter = wgpu::util::initialize_adapter_from_env_or_default(
        &instance, // Request an adapter which can render to our surface
        surface.as_ref().ok(),
    )
    .await
    .ok_or(anyhow!("Failed to find an appropriate adapter"))?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::PUSH_CONSTANTS
                    | wgpu::Features::SPIRV_SHADER_PASSTHROUGH
                    | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                limits: wgpu::Limits {
                    max_push_constant_size: 128,
                    ..Default::default()
                },
            },
            None,
        )
        .await?;

    let surface = surface
        .map(|surface| auto_configure_surface(&adapter, &device, surface, window.inner_size()));

    Ok(SurfaceState {
        surface,
        adapter,
        device: Arc::new(device),
        queue,
    })
}

pub struct SurfaceWithConfig {
    pub surface: Surface,
    pub config: SurfaceConfiguration,
}

fn auto_configure_surface(
    adapter: &Adapter,
    device: &Device,
    surface: wgpu::Surface,
    size: winit::dpi::PhysicalSize<u32>,
) -> SurfaceWithConfig {
    let mut surface_config = surface
        .get_default_config(adapter, size.width, size.height)
        .unwrap();

    surface_config.present_mode = wgpu::PresentMode::AutoVsync;

    surface.configure(device, &surface_config);

    SurfaceWithConfig {
        surface,
        config: surface_config,
    }
}

impl SurfaceState {}
