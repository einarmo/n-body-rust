use std::sync::Arc;

use wgpu::{Adapter, CreateSurfaceError, Device, Queue, Surface, SurfaceConfiguration};
use winit::{dpi::LogicalSize, event_loop::ActiveEventLoop, window::Window};

pub struct WindowState {
    pub window: Arc<Window>,
}

pub fn get_window(
    event_loop: &ActiveEventLoop,
    init_w: f32,
    init_h: f32,
) -> anyhow::Result<WindowState> {
    let window = event_loop.create_window(
        Window::default_attributes().with_inner_size(LogicalSize::new(init_w, init_h)),
    )?;
    Ok(WindowState {
        window: Arc::new(window),
    })
}

pub struct SurfaceState {
    pub surface: Result<SurfaceWithConfig, CreateSurfaceError>,
    #[expect(unused)]
    pub adapter: Adapter,
    pub device: Arc<Device>,
    pub queue: Queue,
}

impl SurfaceWithConfig {
    pub fn configure(&mut self, device: &Device) {
        self.surface.configure(device, &self.config);
    }
}

pub async fn get_surface(window: Arc<Window>) -> anyhow::Result<SurfaceState> {
    let backends = wgpu::Backends::from_env().unwrap_or(wgpu::Backends::VULKAN);
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });

    for inst in instance.enumerate_adapters(backends) {
        println!("{:?}", inst.get_info());
    }

    let surface = instance.create_surface(window.clone());
    let adapter = wgpu::util::initialize_adapter_from_env_or_default(
        &instance, // Request an adapter which can render to our surface
        surface.as_ref().ok(),
    )
    .await?;

    println!("using: {:?}", adapter.get_info());

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::SPIRV_SHADER_PASSTHROUGH
                | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
            required_limits: wgpu::Limits {
                max_push_constant_size: 128,
                ..Default::default()
            },
            ..Default::default()
        })
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
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
}

fn auto_configure_surface(
    adapter: &Adapter,
    device: &Device,
    surface: wgpu::Surface<'static>,
    size: winit::dpi::PhysicalSize<u32>,
) -> SurfaceWithConfig {
    let mut surface_config = surface
        .get_default_config(adapter, size.width, size.height)
        .unwrap();

    surface_config.present_mode = wgpu::PresentMode::Fifo;

    surface.configure(device, &surface_config);

    SurfaceWithConfig {
        surface,
        config: surface_config,
    }
}
