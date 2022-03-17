use std::iter;
use std::mem;
use std::ops::Range;

use pollster::FutureExt;
use raw_window_handle::HasRawWindowHandle;
use wgpu::util::DeviceExt;

mod model;
mod texture;

use crate::error::RenderingError;
use crate::{Mat4, Quaternion, Vec3, Vec4};
use model::{DrawModel, Vertex};

type ModelIndex = usize;

#[rustfmt::skip]
pub fn opengl_to_wgpu_matrix() -> Mat4 {
    Mat4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.0, 0.0, 0.5, 1.0,
    )
}

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_fov_rh_zo(self.fovy, 1.6, 0.9, self.znear, self.zfar);
        proj * view
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: unsafe { mem::transmute(Mat4::identity()) },
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = unsafe {
            mem::transmute(opengl_to_wgpu_matrix() * camera.build_view_projection_matrix())
        };
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quaternion,
    pub scale: Vec3,
}

impl Transform {
    fn as_raw(&self) -> InstanceRaw {
        let Vec3 { x, y, z } = self.scale;

        InstanceRaw {
            model: unsafe {
                mem::transmute::<Mat4, _>(
                    Mat4::translation_3d(self.position)
                        * Mat4::from(self.rotation)
                        * Mat4::with_diagonal(Vec4::new(x, y, z, 1.0)),
                )
            },
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0 * 4 * 4,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 1 * 4 * 4,
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

#[derive(Debug, Default)]
pub struct ModelManager {
    models: Vec<model::Model>,
    instances: Vec<Vec<Transform>>,
    instance_buffers: Vec<wgpu::Buffer>,
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            models: vec![],
            instances: vec![],
            instance_buffers: vec![],
        }
    }

    pub fn add_model(
        &mut self,
        device: &wgpu::Device,
        model: model::Model,
        n_instances: u64,
    ) -> ModelIndex {
        let idx = self.models.len();
        self.models.push(model);
        self.instances.push(vec![]);
        self.instance_buffers
            .push(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Instance buffer {}", self.models.len())),
                size: n_instances * 4 * 4 * mem::size_of::<f32>() as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            }));
        idx
    }

    pub fn get_transforms(&self, model: ModelIndex, range: Range<usize>) -> &[Transform] {
        &self.instances[model][range]
    }

    pub fn modify_transforms_with<F>(
        &mut self,
        model: ModelIndex,
        range: Range<usize>,
        f: F,
        queue: &wgpu::Queue,
    ) where
        F: FnOnce(&mut [Transform]),
    {
        f(&mut self.instances[model][range.clone()]);
        let raw: Vec<_> = self.instances[model][range.clone()]
            .iter()
            .map(Transform::as_raw)
            .collect();
        queue.write_buffer(
            &self.instance_buffers[model],
            range.start as u64 * mem::size_of::<InstanceRaw>() as u64,
            bytemuck::cast_slice(&raw[..]),
        );
    }

    pub fn set_transforms(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: ModelIndex,
        new_transforms: Vec<Transform>,
    ) {
        let old_len = self.instances[model].len();
        let raw: Vec<_> = new_transforms.iter().map(Transform::as_raw).collect();
        if old_len < self.instances[model].len() {
            let new_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Instance buffer for model {}", model)),
                contents: bytemuck::cast_slice(&raw),
                usage: wgpu::BufferUsages::VERTEX,
            });
            self.instance_buffers[model] = new_buffer;
        } else {
            queue.write_buffer(&self.instance_buffers[model], 0, bytemuck::cast_slice(&raw));
        }
        self.instances[model] = new_transforms;
    }
}

pub struct Renderer {
    pub camera: Camera,

    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (u32, u32),
    render_pipeline: wgpu::RenderPipeline,
    model_manager: ModelManager,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
    clear_color: [f64; 3],
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
                label: Some("texture_bind_group_layout"),
            });

        let camera = Camera {
            eye: (0.0, 5.0, -10.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: Vec3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: std::f32::consts::FRAC_PI_4,
            znear: 0.1,
            zfar: 100.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

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
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let depth_texture = texture::Texture::new_depth_texture(&device, &config);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[model::ModelVertex::desc(), InstanceRaw::desc()],
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

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            model_manager: ModelManager::new(),
            texture_bind_group_layout,
            camera,
            camera_buffer,
            camera_bind_group,
            camera_uniform,
            depth_texture,
            clear_color,
        })
    }

    pub fn resize(&mut self, (width, height): (u32, u32)) {
        if width > 0 && height > 0 {
            self.camera.aspect = self.config.width as f32 / self.config.height as f32;
            self.size = (width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = texture::Texture::new_depth_texture(&self.device, &self.config);
        }
    }

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

    pub fn update_camera(&mut self) {
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn update_instances(&mut self, instances: &[(ModelIndex, &[Transform])]) {
        for (idx, data) in instances {
            self.model_manager
                .set_transforms(&self.device, &self.queue, *idx, data.to_vec());
        }
    }

    pub fn render(&mut self) -> Result<(), RenderingError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

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

            for (c, obj_model) in self.model_manager.models.iter().enumerate() {
                render_pass.set_vertex_buffer(1, self.model_manager.instance_buffers[c].slice(..));
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.draw_model_instanced(
                    obj_model,
                    0..self.model_manager.instances[c].len() as u32,
                    &self.camera_bind_group,
                );
            }
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
