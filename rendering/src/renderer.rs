use std::iter;
use std::mem;

use egui;
use pollster::FutureExt;
use raw_window_handle::HasRawWindowHandle;
use wgpu::util::DeviceExt;

use crate::model::ModelIndex;
use crate::model::ModelManager;
use crate::model::{self, DrawModel, Vertex};
use crate::{camera, texture, Camera, RenderingError};
use common::{Mat4, Transform, Vec3, Vec4};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawTranslationMatrix {
    model: [[f32; 4]; 4],
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
    line_render_pipeline: wgpu::RenderPipeline,
    line_vertex_buffer: wgpu::Buffer,
    n_lines: u32,
    render_pipeline: wgpu::RenderPipeline,
    model_manager: ModelManager,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
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
            zfar: 100.0,
        };

        let camera_uniform = camera::CameraUniform::new(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("shader.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shader.wgsl").into()),
        });

        let line_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("line_shader.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../line_shader.wgsl").into()),
        });

        let depth_texture = texture::Texture::new_depth_texture(&device, &config);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let n_lines = 16;
        let line_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
            size: n_lines as wgpu::BufferAddress * mem::size_of::<RawLine>() as wgpu::BufferAddress,
        });
        let line_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Line Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });
        // Render pipeline for the lines, this is largely the same as the normal one
        // with a few explicit differences
        let line_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&line_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: "vs_main",
                buffers: &[RawLineVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        // we want our lines to be rendered
                        // over other geometry
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                // we explicitly wish to use the GPUs built in
                // line rendering hardware
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // culling doesn't matter
                cull_mode: None,
                // this shouldn't matter, but anything besides
                // fill requires specific GPU features.
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: false,
                // unlike the normal pipeline, our depth test always
                // succeeds, to make sure we always draw lines on top
                // of other geometry. This might change in the future,
                // or be made configurable.
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[model::ModelVertex::desc(), RawTranslationMatrix::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });

        let painter = crate::ui::Painter::new(&device, &queue, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            line_render_pipeline,
            line_vertex_buffer,
            n_lines,
            model_manager: ModelManager::new(),
            texture_bind_group_layout,
            camera,
            camera_buffer,
            camera_bind_group,
            camera_uniform,
            depth_texture,
            clear_color,
            painter,
        })
    }

    pub fn resize(&mut self, (width, height): (u32, u32)) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.camera.aspect = width as f32 / height as f32;
            self.size = (width, height);
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = texture::Texture::new_depth_texture(&self.device, &self.config);
        }
        // TODO: also resize UI stuff
    }

    /// Load an obj file and all its associate files.
    pub fn load_model<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
    ) -> Result<ModelIndex, RenderingError> {
        let model = model::Model::load(
            &self.device,
            &self.queue,
            &self.texture_bind_group_layout,
            path,
        )?;

        let idx = self.model_manager.add_model(&self.device, model, 16);

        Ok(idx)
    }

    pub fn get_models_mut(&mut self) -> &mut ModelManager {
        &mut self.model_manager
    }

    pub fn update_camera(&mut self) {
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    #[deprecated = "use the model manager for this functionality instead"]
    pub fn update_instances(&mut self, instances: &[(ModelIndex, &[Transform])]) {
        for (idx, data) in instances {
            self.model_manager
                .set_transforms(&self.device, &self.queue, *idx, data.to_vec());
        }
    }

    // TODO: pass some kind of Scene object to renderer instead, or make it a part of renderer
    // this would help in allowing the renderer to be more configurable, and would alleviate
    // some of the potential creep in just getting more and more arguments.
    // This also conflicts design-wise with the existing model manager, as we now have two
    // entirely distinct ways to interact with what is being rendered.
    pub fn render(
        &mut self,
        lines: &[Line],
        ui: &egui::Context,
        egui_output: egui::FullOutput,
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

        // update the line buffer to accomodate potential extra lines
        if lines.len() > self.n_lines as usize {
            self.n_lines = lines.len() as u32;
            self.line_vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Line Buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
                size: self.n_lines as wgpu::BufferAddress
                    * mem::size_of::<RawLine>() as wgpu::BufferAddress,
            });
        }

        // write the lines into the buffer immediately
        // normally you don't want to do this every frame
        // but in this case
        self.queue.write_buffer(
            &self.line_vertex_buffer,
            0,
            bytemuck::cast_slice(
                &lines
                    .iter()
                    .cloned()
                    .map(Line::into_raw)
                    .collect::<Vec<_>>(),
            ),
        );

        {
            let [r, g, b] = self.clear_color;
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            for (c, obj_model) in self.model_manager.models().iter().enumerate() {
                render_pass
                    .set_vertex_buffer(1, self.model_manager.instance_buffers()[c].slice(..));
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.draw_model_instanced(
                    obj_model,
                    0..self.model_manager.instances()[c].len() as u32,
                    &self.camera_bind_group,
                );
            }

            render_pass.set_pipeline(&self.line_render_pipeline);
            render_pass.set_vertex_buffer(0, self.line_vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.draw(0..lines.len() as u32, 0..1);

            self.painter
                .update_textures(&self.device, &self.queue, egui_output.textures_delta.set);
            let meshes = ui.tessellate(egui_output.shapes);
            self.painter.paint(
                &self.device,
                &self.queue,
                &mut render_pass,
                meshes,
                1.0,
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
