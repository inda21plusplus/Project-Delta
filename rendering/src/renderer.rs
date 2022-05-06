use std::iter;
use std::mem;

use egui;
use pollster::FutureExt;
use raw_window_handle::HasRawWindowHandle;

use crate::model::ModelIndex;
use crate::model::ModelManager;
use crate::{texture, Camera, RenderingError};
use common::{Mat3, Mat4, Transform, Vec3, Vec4};

pub mod gbuffer;
pub mod world;
use world::World;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawTranslationMatrix {
    model: [[f32; 4]; 4],
    rotation: [[f32; 3]; 3],
}

impl RawTranslationMatrix {
    pub fn new(transform: Transform) -> Self {
        let Vec3 { x, y, z } = transform.scale;

        Self {
            model: unsafe {
                mem::transmute::<Mat4, _>(
                    Mat4::translation_3d(transform.position)
                        * Mat4::from(transform.rotation)
                        * Mat4::with_diagonal(Vec4::new(x, y, z, 1.0)),
                )
            },
            rotation: unsafe { mem::transmute::<Mat3, _>(Mat3::from(transform.rotation)) },
        }
    }
}

impl RawTranslationMatrix {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<RawTranslationMatrix>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 4 * 4,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 2 * 4 * 4,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 3 * 4 * 4,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Renderer {
    pub camera: Camera,
    painter: crate::ui::Painter,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (u32, u32),
    line_render_pipeline_layout: wgpu::PipelineLayout,
    line_shader: wgpu::ShaderModule,
    shader: wgpu::ShaderModule,
    render_pipeline_layout: wgpu::PipelineLayout,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    light_bind_group_layout: wgpu::BindGroupLayout,
    depth_texture: texture::Texture,
    worlds: Vec<World>,
    clear_color: [f64; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Line {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Vec3,
}

impl Line {
    fn into_raw(self) -> RawLine {
        let Line {
            start:
                Vec3 {
                    x: s_x,
                    y: s_y,
                    z: s_z,
                },
            end:
                Vec3 {
                    x: e_x,
                    y: e_y,
                    z: e_z,
                },
            color: Vec3 { x: r, y: g, z: b },
        } = self;
        let color = [r, g, b];
        RawLine {
            start: RawLineVertex {
                pos: [s_x, s_y, s_z],
                color,
            },
            end: RawLineVertex {
                pos: [e_x, e_y, e_z],
                color,
            },
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RawLine {
    start: RawLineVertex,
    end: RawLineVertex,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RawLineVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}

impl RawLineVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<RawLineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl Renderer {
    pub fn new<W: HasRawWindowHandle>(
        window: &W,
        size: (u32, u32),
        clear_color: [f64; 3],
    ) -> Result<Self, RenderingError> {
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .block_on()
            .ok_or(RenderingError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .block_on()?;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.0,
            height: size.1,
            present_mode: wgpu::PresentMode::Fifo,
        };

        surface.configure(&device, &config);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture bind group layout"),
            });

        let camera = Camera {
            eye: (0.0, 5.0, -10.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: Vec3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45f32.to_radians(),
            znear: 0.1,
            zfar: 2000.0,
        };

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("light_bind_group_layout"),
            });

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("shader.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shader.wgsl").into()),
        });
        let line_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("line_shader.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../line_shader.wgsl").into()),
        });

        let depth_texture = texture::Texture::new_depth_texture(&device, &config, true);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let line_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Line Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let painter = crate::ui::Painter::new(&device, &queue, &config);

        let mut this = Self {
            surface,
            device,
            queue,
            config,
            size,
            shader,
            render_pipeline_layout,
            line_shader,
            line_render_pipeline_layout,
            camera,
            camera_bind_group_layout,
            light_bind_group_layout,
            depth_texture,
            clear_color,
            worlds: vec![],
            painter,
        };

        this.worlds = vec![World::new(&this, camera)];

        Ok(this)
    }

    pub fn resize(&mut self, (width, height): (u32, u32)) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.camera.aspect = width as f32 / height as f32;
            self.size = (width, height);
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::new_depth_texture(&self.device, &self.config, true);
            self.painter.resize(&self.device, &self.queue, &self.config);

            for world in &mut self.worlds {
                world.resize(&self.device, &self.config);
            }
        }
    }

    /// Load an obj file and all its associate files.
    pub fn load_model<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
    ) -> Result<ModelIndex, RenderingError> {
        self.worlds[0].load_model(&self.device, &self.queue, path)
    }

    pub fn get_models_mut(&mut self) -> ModelManager {
        self.worlds[0].get_models_mut(&self.device, &self.queue)
    }

    pub fn update_camera(&mut self) {
        self.worlds[0].camera = self.camera.clone();
        self.worlds[0].update_camera(&self.queue, self.painter.last_aspect);
    }

    #[deprecated = "use the model manager for this functionality instead"]
    pub fn update_instances(&mut self, instances: &[(ModelIndex, &[Transform])]) {
        self.worlds[0].update_instances(&self.device, &self.queue, instances)
    }

    pub fn make_egui_render_target(&mut self, ctx: &egui::Context) -> egui::TextureHandle {
        let tex_mgr = ctx.tex_manager();
        let id = {
            let mut lock = tex_mgr.write();
            lock.alloc(
                "scene render texture".to_string(),
                egui::ImageData::Color(egui::ColorImage::new(
                    [self.config.width as usize, self.config.height as usize],
                    egui::Color32::BLACK,
                )),
            )
        };
        let handle = egui::TextureHandle::new(tex_mgr, id);

        let render_texture = texture::Texture::new_render_target(
            "egui ui texture",
            &self.device,
            (self.config.width, self.config.height),
            self.config.format,
        );

        let ui_tex = self.painter.make_ui_texture(&self.device, render_texture);
        self.painter.set_render_texture(id, ui_tex);

        handle
    }

    // TODO: pass some kind of Scene object to renderer instead, or make it a part of renderer
    // this would help in allowing the renderer to be more configurable, and would alleviate
    // some of the potential creep in just getting more and more arguments.
    // This also conflicts design-wise with the existing model manager, as we now have two
    // entirely distinct ways to interact with what is being rendered.
    pub fn render(
        &mut self,
        lines: &[Line],
        lights: &[world::Light],
        ui: &egui::Context,
        egui_output: egui::FullOutput,
        pixels_per_point: f32,
        deferred: bool,
    ) -> Result<(), RenderingError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let render_tex = if let Some((_, render_tex)) = self.painter.get_render_texture() {
            &render_tex.tex.view
        } else {
            &view
        };
        if deferred {
            self.worlds[0].render_deferred(&self.device, lines, lights, render_tex, &self.queue)?;
        } else {
            self.worlds[0].render(&self.device, lines, render_tex, None, &self.queue)?;
        }

        {
            let mut ui_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI render pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load, // ::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            self.painter
                .update_textures(&self.device, &self.queue, egui_output.textures_delta.set);
            let meshes = ui.tessellate(egui_output.shapes);

            self.painter.paint(
                &self.device,
                &self.queue,
                &mut ui_render_pass,
                meshes,
                pixels_per_point,
                self.config.height,
                self.config.width,
            );
        }

        self.painter.free_textures(egui_output.textures_delta.free);

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
