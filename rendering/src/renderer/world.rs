use wgpu::util::DeviceExt;

use super::{gbuffer::GBuffer, Line, RawLine};
use crate::camera::{Camera, CameraUniform};
use crate::error::RenderingError;
use crate::model::{DrawModel, Model, ModelIndex, ModelStorage, ModelVertex, Vertex};
use crate::renderer::Renderer;
use crate::texture;
use common::{Transform, Vec3};

use std::mem;

// A world of "things" that can be rendered to an area of the screen
pub struct World {
    line_render_pipeline: wgpu::RenderPipeline, // Pipeline to render lines
    line_vertex_buffer: wgpu::Buffer,           // Line vertex buffer
    n_lines: u32,                               // How many lines the line buffer can fit

    camera_bind_group: wgpu::BindGroup, // the camera's bindgroup
    camera_uniform: CameraUniform,      // the matching uniform
    camera_buffer: wgpu::Buffer,        // the camera's buffer

    render_pipeline: wgpu::RenderPipeline, // the main (forward) render pipeline

    texture_bind_group_layout: wgpu::BindGroupLayout, // the bindgroup for all model textures

    forward_light_bind_group: wgpu::BindGroup, // the forward-renderer's light bindgroup
    forward_light_buffer: wgpu::Buffer,        // the matching buffer

    depth_texture: texture::Texture, // the depth texture for this world
    depth_bind_group: wgpu::BindGroup, // its bind group (for shader use)
    depth_bind_group_layout: wgpu::BindGroupLayout, // the matching layout

    deferred_pipeline: wgpu::RenderPipeline, // the render pipeline that renders to the GBuffer
    // this pipeline copies the diffuse GBuffer to the screen.
    // We do this to make sure that no objects become completely invisible.
    copy_pipeline: wgpu::RenderPipeline,
    screen_quad: ScreenQuad, // the screenquad that the copy pipeline and all screenspace effects use
    light_pipeline: wgpu::RenderPipeline, // the main lighting pass pipeline
    light_spheres: LightSpheres, // the manager for all lights and their corresponding light volumes
    gbuffer: GBuffer,        // the G-Buffer itself

    model_storage: ModelStorage, // where all models are kept and managed

    pub deferred: bool,

    clear_color: [f64; 3],
}

impl World {
    pub fn new(rd: &Renderer) -> Self {
        let camera_uniform = CameraUniform::default();

        let camera_buffer = rd
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let forward_light_buffer =
            rd.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Forward light buffer"),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    contents: bytemuck::cast_slice(&[LightUniform {
                        world_pos: [0.0; 3],
                        radius: 13.0,
                        color: [1.0; 4],
                        ks: [1.0, 0.35, 0.44, 0.0],
                    }]),
                });

        let camera_bind_group = rd.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &rd.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let forward_light_bind_group = rd.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &rd.light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: forward_light_buffer.as_entire_binding(),
            }],
            label: Some("light_bind_group"),
        });

        let model_storage = ModelStorage::new();

        let n_lines = 32;

        let line_vertex_buffer = rd.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
            size: n_lines as wgpu::BufferAddress
                * mem::size_of::<super::RawLine>() as wgpu::BufferAddress,
        });

        // Render pipeline for the lines, this is largely the same as the normal one
        // with a few explicit differences
        let render_pipeline = rd
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&rd.render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &rd.shader,
                    entry_point: "vs_main",
                    buffers: &[ModelVertex::desc(), super::RawTranslationMatrix::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &rd.shader,
                    entry_point: "fs_main",
                    targets: &[wgpu::ColorTargetState {
                        format: rd.config.format,
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
                    cull_mode: Some(wgpu::Face::Back), // Some(wgpu::Face::Back),
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

        let line_render_pipeline =
            rd.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Line Render Pipeline"),
                    layout: Some(&rd.line_render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &rd.line_shader,
                        entry_point: "vs_main",
                        buffers: &[super::RawLineVertex::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &rd.line_shader,
                        entry_point: "fs_main",
                        targets: &[wgpu::ColorTargetState {
                            format: rd.config.format,
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

        let texture_bind_group_layout =
            rd.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let depth_texture = texture::Texture::new_depth_texture(&rd.device, &rd.config, false);
        let depth_bind_group_layout =
            rd.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("deferred depth texture bind group layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            count: None,
                        },
                    ],
                });
        let depth_bind_group = rd.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Deferred depth bind group"),
            layout: &depth_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&depth_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&depth_texture.sampler),
                },
            ],
        });

        let gbuffer = GBuffer::new(&rd.device, (rd.config.width, rd.config.height));

        let deferred_shader = rd
            .device
            .create_shader_module(&wgpu::include_wgsl!("../../shaders/deferred.wgsl"));

        let blend = Some(wgpu::BlendState {
            color: wgpu::BlendComponent::REPLACE,
            alpha: wgpu::BlendComponent::REPLACE,
        });
        let deferred_pipeline = rd
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Deferred Render Pipeline"),
                layout: Some(&rd.render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &deferred_shader,
                    entry_point: "vs_main",
                    buffers: &[ModelVertex::desc(), super::RawTranslationMatrix::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &deferred_shader,
                    entry_point: "fs_main",
                    targets: &[
                        // Albedo buffer
                        wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            blend,
                            write_mask: wgpu::ColorWrites::ALL,
                        },
                        // Normal buffer
                        wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba32Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        },
                        // Position buffer
                        wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba32Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        },
                    ],
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

        // this pipeline does nothing but copy over color data
        let copy_shader = rd
            .device
            .create_shader_module(&wgpu::include_wgsl!("../../shaders/gbuffer_copy.wgsl"));
        let copy_pipeline_layout =
            rd.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("copy pipeline"),
                    bind_group_layouts: &[&gbuffer.bind_group_layout],
                    push_constant_ranges: &[],
                });
        let copy_pipeline = rd
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Copy pipeline"),
                layout: Some(&copy_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &copy_shader,
                    entry_point: "vs_main",
                    buffers: &[ScreenQuad::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &copy_shader,
                    entry_point: "fs_main",
                    targets: &[wgpu::ColorTargetState {
                        format: rd.config.format,
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
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        let light_shader = rd
            .device
            .create_shader_module(&wgpu::include_wgsl!("../../shaders/deferred-light.wgsl"));
        let light_pipeline_layout =
            rd.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("lighting pipeline layout"),
                    bind_group_layouts: &[&gbuffer.bind_group_layout, &rd.camera_bind_group_layout],
                    push_constant_ranges: &[],
                });
        let light_pipeline = rd
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Light pipeline"),
                layout: Some(&light_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &light_shader,
                    entry_point: "vs_main",
                    buffers: &[LightSpheres::desc(), RawLight::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &light_shader,
                    entry_point: "fs_main",
                    targets: &[wgpu::ColorTargetState {
                        format: rd.config.format,
                        // we explicitly want to blend all the light generated
                        // also add in the dst as that will be the copy result
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Front),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                // the lights are to be drawn regardless, and so don't need
                // a depth buffer
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        World {
            line_render_pipeline,
            line_vertex_buffer,
            n_lines,
            render_pipeline,
            model_storage,
            camera_bind_group,
            camera_uniform,
            camera_buffer,
            forward_light_bind_group,
            forward_light_buffer,
            depth_texture,
            depth_bind_group_layout,
            depth_bind_group,
            texture_bind_group_layout,
            gbuffer,
            copy_pipeline,
            deferred_pipeline,
            light_pipeline,
            light_spheres: LightSpheres::new(&rd.device),
            screen_quad: ScreenQuad::new(&rd.device),
            deferred: true,
            clear_color: rd.clear_color,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.depth_texture = texture::Texture::new_depth_texture(device, config, false);
        self.depth_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Deferred depth bind group"),
            layout: &self.depth_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.depth_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.depth_texture.sampler),
                },
            ],
        });
        self.gbuffer.resize(device, (config.width, config.height));
    }

    fn do_render_rpass<'a>(
        &'a self,
        lines_to_draw: u32,
        render_pass: &mut wgpu::RenderPass<'a>,
        use_deferred_pipeline: bool,
    ) {
        render_pass.set_pipeline(if use_deferred_pipeline {
            &self.deferred_pipeline
        } else {
            &self.render_pipeline
        });

        for (c, obj_model) in self.model_storage.models().iter().enumerate() {
            render_pass.set_vertex_buffer(1, self.model_storage.instance_buffers()[c].slice(..));

            render_pass.draw_model_instanced(
                obj_model,
                0..self.model_storage.instances()[c].len() as u32,
                &self.camera_bind_group,
                &self.forward_light_bind_group,
            );
        }

        if !use_deferred_pipeline {
            render_pass.set_pipeline(&self.line_render_pipeline);
            render_pass.set_vertex_buffer(0, self.line_vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.draw(0..lines_to_draw * 2, 0..1);
        }
    }

    fn upload_lines(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, lines: &[Line]) {
        // update the line buffer to accomodate potential extra lines
        if lines.len() > self.n_lines as usize {
            self.n_lines = lines.len() as u32;
            self.line_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
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
        queue.write_buffer(
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
    }

    pub fn set_lights(&mut self, lights: Vec<(Light, Vec3)>) {
        self.light_spheres.lights = lights;
    }

    fn upload_lights(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let lights = &self.light_spheres.lights;
        let raw_lights: Vec<_> = lights.iter().map(RawLight::from).collect();
        update_buffer(
            device,
            queue,
            &mut self.light_spheres.transforms,
            &raw_lights,
            &mut self.light_spheres.n_lights,
        );
        if let Some(light) = raw_lights.get(0) {
            queue.write_buffer(
                &self.forward_light_buffer,
                0,
                bytemuck::cast_slice(&[LightUniform::from(light)]),
            );
        }
    }

    pub fn render_rpass<'a>(
        &'a mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lines: &[Line],
        render_pass: &mut wgpu::RenderPass<'a>,
    ) -> Result<(), RenderingError> {
        self.upload_lines(device, queue, lines);
        self.do_render_rpass(lines.len() as u32, render_pass, false);

        Ok(())
    }

    /// Load an obj file and all its associate files.
    pub fn load_model<P: AsRef<std::path::Path>>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
    ) -> Result<ModelIndex, RenderingError> {
        let model = Model::load(device, queue, &self.texture_bind_group_layout, path)?;

        let idx = self.model_storage.add_model(device, model, 16);

        Ok(idx)
    }

    pub fn update_camera(&mut self, queue: &wgpu::Queue, camera: &Camera, aspect: f32) {
        self.camera_uniform.update_view_proj(camera, aspect);

        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn update_instances<'a>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: impl Iterator<Item = (ModelIndex, &'a [Transform])>,
    ) {
        for (idx, transforms) in instances {
            self.model_storage
                .set_transforms(device, queue, idx, transforms.to_vec());
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        lines: &[super::Line],
        render_target: &wgpu::TextureView,
        depth_attachment: Option<&texture::Texture>,
        queue: &wgpu::Queue,
    ) -> Result<(), RenderingError> {
        if self.deferred {
            self.render_deferred(device, lines, render_target, queue)
        } else {
            self.render_forward(device, lines, render_target, depth_attachment, queue)
        }
    }

    fn render_forward(
        &mut self,
        device: &wgpu::Device,
        lines: &[super::Line],
        render_target: &wgpu::TextureView,
        depth_attachment: Option<&texture::Texture>,
        queue: &wgpu::Queue,
    ) -> Result<(), RenderingError> {
        self.upload_lines(device, queue, lines);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let [r, g, b] = self.clear_color;
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_attachment.unwrap_or(&self.depth_texture).view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            self.do_render_rpass(lines.len() as u32, &mut render_pass, false);
        }
        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    fn render_deferred(
        &mut self,
        device: &wgpu::Device,
        lines: &[super::Line],
        render_target: &wgpu::TextureView,
        queue: &wgpu::Queue,
    ) -> Result<(), RenderingError> {
        self.upload_lines(device, queue, lines);
        self.upload_lights(device, queue);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Deferred render GBuffer pass"),
                color_attachments: &self.gbuffer.color_attachments(),
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.push_debug_group("deferred render");
            render_pass.set_pipeline(&self.deferred_pipeline);
            for (c, obj_model) in self.model_storage.models().iter().enumerate() {
                render_pass
                    .set_vertex_buffer(1, self.model_storage.instance_buffers()[c].slice(..));
                render_pass.draw_model_instanced(
                    obj_model,
                    0..self.model_storage.instances()[c].len() as u32,
                    &self.camera_bind_group,
                    &self.forward_light_bind_group,
                );
            }
            render_pass.pop_debug_group();
        }
        // LIGHTING PASS
        // we start a separate render pass as our attachments and depth
        // buffers are different
        {
            let mut light_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Deferred render lighting pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            light_render_pass.set_bind_group(0, &self.gbuffer.bind_group, &[]);

            // COPY PASS
            light_render_pass.set_vertex_buffer(0, self.screen_quad.vtx.slice(..));
            light_render_pass
                .set_index_buffer(self.screen_quad.idx.slice(..), wgpu::IndexFormat::Uint16);
            light_render_pass.set_pipeline(&self.copy_pipeline);
            light_render_pass.draw_indexed(0..6, 0, 0..1);

            // LIGHT PASS
            light_render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            light_render_pass.set_bind_group(2, &self.depth_bind_group, &[]);
            light_render_pass.set_pipeline(&self.light_pipeline);
            light_render_pass.set_vertex_buffer(0, self.light_spheres.vtx.slice(..));
            light_render_pass
                .set_index_buffer(self.light_spheres.idx.slice(..), wgpu::IndexFormat::Uint32);
            light_render_pass.set_vertex_buffer(1, self.light_spheres.transforms.slice(..));
            light_render_pass.draw_indexed(
                0..self.light_spheres.n_elems,
                0,
                0..self.light_spheres.lights.len() as u32,
            );
        }
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Line Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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

            render_pass.set_pipeline(&self.line_render_pipeline);
            render_pass.set_vertex_buffer(0, self.line_vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.draw(0..lines.len() as u32 * 2, 0..1);
        }
        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

pub struct ScreenQuad {
    vtx: wgpu::Buffer,
    idx: wgpu::Buffer,
}

pub struct LightSpheres {
    vtx: wgpu::Buffer,
    idx: wgpu::Buffer,
    transforms: wgpu::Buffer, // just a float as we only need a scale value
    n_elems: u32,
    n_lights: u64,
    lights: Vec<(Light, Vec3)>,
}

#[repr(C)]
#[derive(bytemuck::Zeroable, bytemuck::Pod, Debug, Copy, Clone)]
pub struct RawLight {
    world_pos: [f32; 3],
    color: [f32; 3],
    radius: f32,
    k_c: f32,
    k_l: f32,
    k_q: f32,
}

#[repr(C)]
#[derive(bytemuck::Zeroable, bytemuck::Pod, Debug, Copy, Clone)]
struct LightUniform {
    world_pos: [f32; 3],
    radius: f32,
    color: [f32; 4], // alpha isn't used, just padding
    ks: [f32; 4],    // k_{c,l,q} and an extra for padding
}

impl From<&RawLight> for LightUniform {
    fn from(l: &RawLight) -> Self {
        let RawLight {
            world_pos,
            color,
            radius,
            k_c,
            k_l,
            k_q,
        } = *l;
        Self {
            world_pos,
            radius,
            color: [color[0], color[1], color[2], 1.0],
            ks: [k_c, k_l, k_q, 0.0],
        }
    }
}

impl From<&(Light, Vec3)> for RawLight {
    fn from((light, pos): &(Light, Vec3)) -> Self {
        RawLight {
            world_pos: pos.into_array(),
            color: light.color,
            radius: light.calc_radius(),
            k_c: light.k_constant,
            k_l: light.k_linear,
            k_q: light.k_quadratic,
        }
    }
}

fn update_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut wgpu::Buffer,
    new_data: &[T],
    buf_size: &mut u64,
) {
    if new_data.len() > *buf_size as usize {
        *buf_size = new_data.len() as u64;
        *buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
            size: *buf_size * mem::size_of::<T>() as wgpu::BufferAddress,
        });
    }
    queue.write_buffer(&*buffer, 0, bytemuck::cast_slice(new_data));
}

impl LightSpheres {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        }
    }

    fn new(device: &wgpu::Device) -> Self {
        use std::io::BufReader;
        static LIGHT_SPHERE_OBJ: &[u8] = include_bytes!("lightsphere.obj");
        let mut buf = BufReader::new(LIGHT_SPHERE_OBJ);
        let sphere_obj = tobj::load_obj_buf(&mut buf, &tobj::GPU_LOAD_OPTIONS, |_| {
            Err(tobj::LoadError::GenericFailure)
        }).expect("failed importing the light sphere, this is a statically linked value and should never fail");
        let mut verts: Vec<[f32; 3]> = Vec::new();
        let mesh = (sphere_obj.0)[0].mesh.clone();
        for i in 0..mesh.positions.len() / 3 {
            verts.push([
                mesh.positions[i * 3],
                mesh.positions[i * 3 + 1],
                mesh.positions[i * 3 + 2],
            ])
        }

        let vtx = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light sphere vertex buffer"),
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&verts),
        });
        let idx = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light sphere index buffer"),
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(&mesh.indices),
        });

        let n_lights = 16;
        let transforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light sphere instance buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: n_lights * mem::size_of::<RawLight>() as wgpu::BufferAddress,
            mapped_at_creation: false,
        });

        Self {
            vtx,
            idx,
            n_elems: mesh.indices.len() as u32,
            n_lights,
            transforms,
            lights: vec![],
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Light {
    pub color: [f32; 3],
    pub k_quadratic: f32,
    pub k_linear: f32,
    pub k_constant: f32,
}

impl Light {
    pub fn calc_radius(&self) -> f32 {
        let light_max = Vec3::from(self.color).reduce_partial_max();
        let &Self {
            k_quadratic: quadratic,
            k_linear: linear,
            k_constant: constant,
            ..
        } = self;
        let almost_black = 1.0 / (5.0 / 256.0 / 12.92); // convert sRGB to linear

        (-linear
            + f32::sqrt(linear.powi(2) - 4.0 * quadratic * (constant - almost_black * light_max)))
            / (2.0 * quadratic)
    }
}

impl ScreenQuad {
    pub fn new(device: &wgpu::Device) -> Self {
        const LOWER_LEFT: [f32; 2] = [-1.0, -1.0];
        const UPPER_RIGHT: [f32; 2] = [1.0, 1.0];
        const QUAD_VERTS: [[f32; 2]; 4] = [
            LOWER_LEFT,
            // upper left
            [LOWER_LEFT[0], UPPER_RIGHT[1]],
            UPPER_RIGHT,
            // lower right
            [UPPER_RIGHT[0], LOWER_LEFT[1]],
        ];
        // lower left-upper left-upper right
        const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

        Self {
            vtx: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("screen quad vertex buffer"),
                usage: wgpu::BufferUsages::VERTEX,
                contents: bytemuck::cast_slice(&QUAD_VERTS),
            }),
            idx: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("screen quad index buffer"),
                usage: wgpu::BufferUsages::INDEX,
                contents: bytemuck::cast_slice(&QUAD_INDICES),
            }),
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        }
    }
}

impl Vertex for RawLight {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<RawLight>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 2 * mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: (2 * mem::size_of::<[f32; 3]>() + mem::size_of::<f32>())
                        as wgpu::BufferAddress,
                    shader_location: 4,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: (2 * mem::size_of::<[f32; 3]>() + 2 * mem::size_of::<f32>())
                        as wgpu::BufferAddress,
                    shader_location: 5,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: (2 * mem::size_of::<[f32; 3]>() + 3 * mem::size_of::<f32>())
                        as wgpu::BufferAddress,
                    shader_location: 6,
                },
            ],
        }
    }
}
