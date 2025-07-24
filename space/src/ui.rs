use std::sync::Arc;

use eframe::egui::{self, Image, Key, TextureId, Vec2, load::SizedTexture};
use egui_wgpu::RenderState;
use wgpu::{FilterMode, TextureFormat, wgt::TextureViewDescriptor};
use winit::dpi::PhysicalSize;

use crate::{
    batch_request::BatchRequest, camera::Camera, event_loop::KeyboardState, objects::Objects,
    render::Renderer,
};

mod info;

pub struct SpaceEguiApp {
    camera: Camera,
    exchange: Arc<BatchRequest>,
    objects: Objects,
    tick: u32,
    keyboard_state: KeyboardState,
    renderer: Renderer,
    texture: IntermediateTexture,
    info_panel: info::InfoPanel,
}

impl SpaceEguiApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        exchange: Arc<BatchRequest>,
        mut objects: Objects,
    ) -> Option<Self> {
        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;

        let initial_size = Vec2::splat(300.0);
        let camera = Camera::new(
            PhysicalSize {
                width: initial_size.x as u32,
                height: initial_size.y as u32,
            },
            &wgpu_render_state.device,
        );
        let renderer = Renderer::new(
            &wgpu_render_state.device,
            TextureFormat::Bgra8Unorm,
            PhysicalSize {
                width: initial_size.x as u32,
                height: initial_size.y as u32,
            },
            &camera,
            &mut objects,
        );
        let texture = IntermediateTexture::new(
            &wgpu_render_state.device,
            PhysicalSize {
                width: initial_size.x as u32,
                height: initial_size.y as u32,
            },
            &wgpu_render_state,
        );

        Some(Self {
            camera,
            exchange,
            objects,
            tick: 0,
            keyboard_state: KeyboardState::default(),
            renderer,
            texture,
            info_panel: info::InfoPanel::new(),
        })
    }
}

impl eframe::App for SpaceEguiApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.tick += 1;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Neato space sim");

            let psize = PhysicalSize {
                width: ui.available_width() as u32 - 300,
                height: ui.available_height() as u32,
            };

            self.camera.resize(psize);
            self.renderer.resize(psize);
            let state = frame.wgpu_render_state().unwrap();
            self.texture.resize(&state.device, psize, &state);

            ui.input(|i| {
                for evt in &i.events {
                    match evt {
                        egui::Event::Key { key, pressed, .. } => match key {
                            Key::ArrowUp => self.keyboard_state.up = *pressed,
                            Key::ArrowDown => self.keyboard_state.down = *pressed,
                            Key::ArrowLeft => self.keyboard_state.left = *pressed,
                            Key::ArrowRight => self.keyboard_state.right = *pressed,
                            Key::Home => self.keyboard_state.home = *pressed,
                            Key::PageUp => self.keyboard_state.pgup = *pressed,
                            Key::Space => self.keyboard_state.space.event(*pressed),
                            Key::W => self.keyboard_state.w = *pressed,
                            Key::S => self.keyboard_state.s = *pressed,
                            Key::A => self.keyboard_state.a = *pressed,
                            Key::D => self.keyboard_state.d = *pressed,
                            Key::Minus => self.keyboard_state.minus = *pressed,
                            Key::Plus => self.keyboard_state.plus = *pressed,
                            Key::F => self.keyboard_state.f.event(*pressed),
                            Key::G => self.keyboard_state.g.event(*pressed),
                            Key::H => self.keyboard_state.h.event(*pressed),
                            Key::J => self.keyboard_state.j.event(*pressed),
                            Key::O => self.keyboard_state.o = *pressed,
                            Key::L => self.keyboard_state.l = *pressed,
                            _ => (),
                        },
                        _ => (),
                    }
                }
            });

            if self.keyboard_state.space.get_trigger() {
                self.objects.clear();
            }
            self.exchange.sample(&mut self.objects);

            self.camera.move_relative(&self.keyboard_state);
            self.camera.zoom(&self.keyboard_state);
            self.camera
                .set_focus(&mut self.keyboard_state, &mut self.objects);
            self.camera.rot(&self.keyboard_state);

            if self.keyboard_state.l {
                self.exchange.set_delta(self.exchange.delta() * 0.9);
            }
            if self.keyboard_state.o {
                self.exchange.set_delta(self.exchange.delta() * 1.1);
            }

            self.renderer.redraw(
                self.tick,
                &mut self.camera,
                &mut self.objects,
                &state.queue,
                &self.texture.texture,
                &state.device,
            );

            let outer_height = ui.available_height();

            ui.horizontal(|ui| {
                ui.add(Image::new(SizedTexture::new(
                    self.texture.id,
                    Vec2::new(ui.available_width() - 300.0, outer_height),
                )));
                self.info_panel.render(
                    ui,
                    &self.objects,
                    self.exchange.current_ticks(),
                    &self.camera,
                    self.tick,
                    self.exchange.delta(),
                );
            });
        });
        ctx.request_repaint();
    }
}

#[derive(Clone)]
struct IntermediateTexture {
    texture: wgpu::Texture,
    size: PhysicalSize<u32>,
    id: TextureId,
}

impl IntermediateTexture {
    pub fn new(device: &wgpu::Device, size: PhysicalSize<u32>, state: &RenderState) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Intermediate Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let id = state.renderer.write().register_native_texture(
            device,
            &texture.create_view(&TextureViewDescriptor::default()),
            FilterMode::Nearest,
        );

        Self { texture, id, size }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: PhysicalSize<u32>, state: &RenderState) {
        if self.size != size {
            self.size = size;
            self.texture.destroy();
            self.texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Intermediate Texture"),
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let mut renderer = state.renderer.write();
            renderer.free_texture(&self.id);
            self.id = renderer.register_native_texture(
                device,
                &self.texture.create_view(&TextureViewDescriptor::default()),
                FilterMode::Nearest,
            );
        }
    }
}
