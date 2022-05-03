use wgpu::util::DeviceExt;

use super::{Line, RawLine};
use crate::camera::{Camera, CameraUniform};
use crate::error::RenderingError;
use crate::model::{DrawModel, Model, ModelIndex, ModelManager, ModelStorage, ModelVertex, Vertex};
use crate::renderer::Renderer;
use crate::texture;

use std::mem;

// A world of "things" that can be rendered to an area of the screen
pub struct World {
    line_render_pipeline: wgpu::RenderPipeline,
    line_vertex_buffer: wgpu::Buffer,
    n_lines: u32,
    render_pipeline: wgpu::RenderPipeline,
    model_storage: ModelStorage,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    camera_bind_group: wgpu::BindGroup,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    pub camera: Camera,
    pub(crate) depth_texture: texture::Texture,
    clear_color: [f64; 3],
}

impl World {
    pub fn new(rd: &Renderer, camera: Camera) -> Self {
        let camera_uniform = CameraUniform::new(&camera);

        let camera_buffer = rd
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let camera_bind_group = rd.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &rd.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
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

        let depth_texture = texture::Texture::new_depth_texture(&rd.device, &rd.config);

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

        World {
            camera,
            line_render_pipeline,
            line_vertex_buffer,
            n_lines,
            render_pipeline,
            model_storage,
            camera_bind_group,
            camera_uniform,
            camera_buffer,
            depth_texture,
            texture_bind_group_layout,
            clear_color: rd.clear_color,
        }
    }

    fn do_render_rpass<'a>(
        &'a self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lines_to_draw: u32,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        for (c, obj_model) in self.model_storage.models().iter().enumerate() {
            render_pass.set_vertex_buffer(1, self.model_storage.instance_buffers()[c].slice(..));
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw_model_instanced(
                obj_model,
                0..self.model_storage.instances()[c].len() as u32,
                &self.camera_bind_group,
            );
        }

        render_pass.set_pipeline(&self.line_render_pipeline);
        render_pass.set_vertex_buffer(0, self.line_vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.draw(0..lines_to_draw, 0..1);
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

    pub fn render_rpass<'a>(
        &'a mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lines: &[Line],
        render_pass: &mut wgpu::RenderPass<'a>,
    ) -> Result<(), RenderingError> {
        self.upload_lines(device, queue, lines);
        self.do_render_rpass(device, queue, lines.len() as u32, render_pass);

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

    pub(crate) fn get_models_mut<'a>(
        &'a mut self,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
    ) -> ModelManager<'a> {
        self.model_storage.get_manager(device, queue)
    }

    pub fn update_camera(&mut self, queue: &wgpu::Queue) {
        self.camera_uniform.update_view_proj(&self.camera);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    //#[deprecated = "use the model manager for this functionality instead"]
    pub fn update_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[(ModelIndex, &[super::Transform])],
    ) {
        for (idx, data) in instances {
            self.model_storage
                .set_transforms(device, queue, *idx, data.to_vec());
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
        self.upload_lines(device, queue, lines);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let [r, g, b] = self.clear_color;
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &render_target,
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

            self.do_render_rpass(device, queue, lines.len() as u32, &mut render_pass);
        }
        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}
