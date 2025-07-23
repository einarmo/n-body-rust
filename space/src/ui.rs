use std::sync::Arc;

use eframe::egui::{self, Key, Sense, Vec2};
use egui_wgpu::{Callback, CallbackTrait};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BlendComponent, BlendFactor, BlendState, Device, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, RenderPass, RenderPipeline, TextureFormat,
    wgt::{SamplerDescriptor, TextureViewDescriptor},
};
use winit::dpi::PhysicalSize;

use crate::{
    batch_request::BatchRequest,
    camera::Camera,
    event_loop::KeyboardState,
    objects::Objects,
    render::{Renderer, get_or_init_shader},
};

pub struct SpaceEguiApp {
    camera: Camera,
    exchange: Arc<BatchRequest>,
    objects: Objects,
    tick: u32,
    keyboard_state: KeyboardState,
    renderer: Renderer,
    texture: IntermediateTexture,
    pipeline: Arc<CopyImagePipeline>,
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
            TextureFormat::Bgra8UnormSrgb,
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
        );
        let pipeline = Arc::new(CopyImagePipeline::new(
            &wgpu_render_state.device,
            TextureFormat::Bgra8Unorm,
            &texture.layout,
        ));

        Some(Self {
            camera,
            exchange,
            objects,
            tick: 0,
            keyboard_state: KeyboardState::default(),
            renderer,
            texture,
            pipeline,
        })
    }
}

impl eframe::App for SpaceEguiApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.tick += 1;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Neato space sim");
            egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                let (size, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), ui.available_height()),
                    Sense::empty(),
                );

                let psize = PhysicalSize {
                    width: size.width() as u32,
                    height: size.height() as u32,
                };
                self.camera.resize(psize);
                self.renderer.resize(psize);
                let state = frame.wgpu_render_state().unwrap();
                self.texture.resize(&state.device, psize);

                self.exchange.sample(&mut self.objects);

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
                                _ => (),
                            },
                            _ => (),
                        }
                    }
                });

                self.camera.move_relative(&self.keyboard_state);
                self.camera.zoom(&self.keyboard_state);
                self.camera
                    .set_focus(&mut self.keyboard_state, &self.objects);
                self.camera.rot(&self.keyboard_state);
                if self.keyboard_state.space.get_trigger() {
                    self.objects.clear();
                }

                self.renderer.redraw(
                    self.tick,
                    &mut self.camera,
                    &mut self.objects,
                    &state.queue,
                    &self.texture.texture,
                    &state.device,
                );

                ui.painter().add(Callback::new_paint_callback(
                    size,
                    SimRenderCallback {
                        pipeline: self.pipeline.clone(),
                        texture: self.texture.clone(),
                    },
                ))
            });
        });
        ctx.request_repaint();
    }
}

struct SimRenderCallback {
    pipeline: Arc<CopyImagePipeline>,
    texture: IntermediateTexture,
}

impl CallbackTrait for SimRenderCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        _callback_resources: &egui_wgpu::CallbackResources,
    ) {
        self.pipeline.draw(render_pass, &self.texture.bind_group);
    }
}

#[derive(Clone)]
struct IntermediateTexture {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    layout: wgpu::BindGroupLayout,
    size: PhysicalSize<u32>,
}

impl IntermediateTexture {
    pub fn new(device: &wgpu::Device, size: PhysicalSize<u32>) -> Self {
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
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let texture_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &texture_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&TextureViewDescriptor::default()),
                ),
            }],
        });

        Self {
            texture,
            bind_group,
            layout: texture_layout,
            size,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: PhysicalSize<u32>) {
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
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &self.texture.create_view(&TextureViewDescriptor::default()),
                    ),
                }],
            });
        }
    }
}

struct CopyImagePipeline {
    pipeline: RenderPipeline,
    sampler_bind_group: wgpu::BindGroup,
}

impl CopyImagePipeline {
    pub fn new(
        device: &Device,
        texture_format: TextureFormat,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let sampler_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            }],
        });
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let sampler_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &sampler_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&sampler),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[texture_layout, &sampler_layout],
            push_constant_ranges: &[],
        });
        let shader_module = get_or_init_shader(device);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("circle pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader_module,
                entry_point: Some("copy_texture_vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            cache: None,
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("copy_texture_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            multiview: None,
        });

        Self {
            pipeline,
            sampler_bind_group,
        }
    }

    pub fn draw(&self, rpass: &mut RenderPass<'_>, texture_bind_group: &wgpu::BindGroup) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, texture_bind_group, &[]);
        rpass.set_bind_group(1, &self.sampler_bind_group, &[]);

        rpass.draw(0..6, 0..1);
    }
}
