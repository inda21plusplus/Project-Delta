use log::info;

use vek::mat::repr_c::Mat4;
use vek::quaternion::repr_c::Quaternion;
use vek::vec::repr_c::Vec3;

use pollster::FutureExt as _;
use raw_window_handle::HasRawWindowHandle;

use wgpu::util::DeviceExt;

mod model;
mod texture;

use model::{DrawModel, Vertex};

use image;

use std::iter;
use std::mem;

#[rustfmt::skip]
pub fn opengl_to_wgpu_matrix() -> Mat4<f32> {
    Mat4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.0, 0.0, 0.5, 1.0,
    )
}

const NUM_INSTANCES_PER_ROW: u32 = 10;

struct Camera {
    eye: Vec3<f32>,
    target: Vec3<f32>,
    up: Vec3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> Mat4<f32> {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_fov_rh_zo(self.fovy, 2.0, 2.0, self.znear, self.zfar);
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
            view_proj: unsafe { mem::transmute(Mat4::<f32>::identity()) },
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = unsafe {
            mem::transmute(opengl_to_wgpu_matrix() * camera.build_view_projection_matrix())
        };
    }
}

struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    fn update_camera(&self, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalized();
        let forward_mag = forward.magnitude();

        // Prevents glitching when camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the up/ down is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and eye so
            // that it doesn't change. The eye therefore still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalized() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalized() * forward_mag;
        }
    }
}

struct Instance {
    position: Vec3<f32>,
    rotation: Quaternion<f32>,
    scale: Vec3<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: unsafe {
                mem::transmute::<Mat4<f32>, _>(
                    Mat4::<f32>::translation_3d(self.position)
                        * Mat4::from(self.rotation)
                        * Mat4::new(
                            self.scale.x,
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                            self.scale.y,
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                            self.scale.z,
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                            1.0,
                        ),
                )
            },
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    #[allow(dead_code)]
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We don't have to do this in code though.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize,
    render_pipeline: wgpu::RenderPipeline,
    obj_model: model::Model,
    camera: Camera,
    camera_controller: CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    instances: Vec<Instance>,
    #[allow(dead_code)]
    instance_buffer: wgpu::Buffer,
    depth_texture: texture::Texture,
    start : std::time::Instant,
}

impl Renderer {
    pub fn new<W: HasRawWindowHandle>(window: &W, size: PhysicalSize) -> Self {
        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        info!("created instance");
        let surface = unsafe { instance.create_surface(window) };
        info!("created surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .block_on()
            .unwrap();
        info!("got adapter");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                    limits: wgpu::Limits::default(),
                },
                // Some(&std::path::Path::new("trace")), // Trace path
                None, // Trace path
            )
            .block_on()
            .unwrap();
        info!("got device");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
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
        let camera_controller = CameraController::new(0.2);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position = Vec3 { x, y: 0.0, z };
                    let scale = Vec3 {
                        x: 1f32,
                        y: 1f32,
                        z: 1.0f32,
                    };

                    let rotation = if position == Vec3::zero() {
                        Quaternion::rotation_3d(0.0, Vec3::unit_z())
                    } else {
                        Quaternion::rotation_3d(std::f32::consts::FRAC_PI_4, position.normalized())
                    };

                    Instance {
                        position,
                        rotation,
                        scale,
                    }
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        info!("got instance buffer");

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

        info!("made camera bind group");

        let res_dir = std::path::Path::new("./").join("res");
        let obj_model = model::Model::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            res_dir.join("cube.obj"),
        );

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("shader.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        info!("created shader");

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        info!("created pipeline layout");

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

        info!("created pipeline");

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            obj_model,
            camera,
            camera_controller,
            camera_buffer,
            camera_bind_group,
            camera_uniform,
            instances,
            instance_buffer,
            depth_texture,
            start : std::time::Instant::now()
        }
    }

    fn resize(&mut self, new_size: PhysicalSize) {
        if new_size.width > 0 && new_size.height > 0 {
            self.camera.aspect = self.config.width as f32 / self.config.height as f32;
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    pub fn update(&mut self) {
        let time = self.start.elapsed().as_secs_f32();
        let offset = time.sin();
        for x in &mut self.instances {
            x.scale = Vec3 {
                x: 1.1f32 + offset,
                y: 1.1f32 + offset,
                z: 1.1f32 + offset,
            };
        }
        let instance_data = self
            .instances
            .iter()
            .map(Instance::to_raw)
            .collect::<Vec<_>>();
        //let a = self.instance_buffer.slice(..).map_async(wgpu::MapMode::Write);
        // self.instance_buffer.slice(..).get_mapped_range_mut().copy_from_slice(bytemuck::cast_slice(&instance_data));
        self.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
        /*self.instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });*/
        //println!("IS GONnA UPdATE {} from {}",offset,time);
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
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

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw_model_instanced(
                &self.obj_model,
                0..self.instances.len() as u32,
                &self.camera_bind_group,
            );
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
