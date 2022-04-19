use crate::texture;
use ahash::AHashMap;
use egui::{
    epaint::{ImageDelta, Mesh, Vertex},
    TextureId,
};
use wgpu;

use std::mem;

// not sure if it's a better idea to use texture::Texture
// TODO: look into it
struct Texture {
    tex: wgpu::Texture,
    _sampler: wgpu::Sampler,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

// in glsl:
// ```
// layout(location = 0) in vec2 pos;
// layout(location = 1) in vec2 uv_coords;
// layout(location = 2) in uint color;
// ```
// in rust,
// see: https://docs.rs/epaint/0.17.0/epaint/struct.Vertex.html
fn egui_vertex_desc<'a>() -> wgpu::VertexBufferLayout<'a> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<f32>() as wgpu::BufferAddress * 2,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<f32>() as wgpu::BufferAddress * 4,
                shader_location: 2,
                format: wgpu::VertexFormat::Uint32,
            },
        ],
    }
}

pub struct Painter {
    textures: AHashMap<egui::TextureId, Texture>,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_buf_size: wgpu::BufferAddress,
    index_buffer: wgpu::Buffer,
    index_buf_size: wgpu::BufferAddress,
    local_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl Painter {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) -> Painter {
        // we keep the bind group layout around
        // as each time we make a new texture, we need to create its bindgroup
        // but these bindgroups are the same every time.
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
                label: Some("egui texture bind group layout"),
            });

        let local_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("egui local buffer"),
            mapped_at_creation: false,
            size: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // write the current config sizes into the local buffer
        // I have no clue what happens when the window is resized.
        // TODO: add (proper) resizing support elsewhere
        queue.write_buffer(
            &local_buffer,
            0,
            bytemuck::cast_slice(&[config.width as f32, config.height as f32]),
        );

        let local_bind_group_layout =
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
                label: Some("egui local bind group layout"),
            });
        // as opposed to the texture bind groups, where we keep the layout around,
        // the local bind group will stay the same forever, so we just make it
        // and discard the layout
        let local_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("egui local bind group"),
            layout: &local_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: local_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("egui_shader.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../egui_shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&local_bind_group_layout, &texture_bind_group_layout],
            label: Some("egui pipeline layout"),
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("egui render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[egui_vertex_desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: config.format,
                    // we don't want any transparent UI to replace
                    // other elements of the scene.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::OVER,
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // cull mode is None as egui has no guarantees for which
                // face will be "forwards"
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            // use the depth stencil that the renderpass uses
            // cause otherwise it crashes
            // TODO: add stencil support so we don't draw the scene where there's UI
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: false,
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
        // "reasonable" default sizes, I have no clue honestly, but it doesn't
        // really matter as on-demand reallocation is supported anyway
        let vertex_buf_size = 32;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("egui vertex buffer"),
            size: vertex_buf_size * mem::size_of::<Vertex>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buf_size = 32;
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("egui index buffer"),
            size: index_buf_size * mem::size_of::<u32>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Painter {
            textures: AHashMap::new(),
            pipeline,
            vertex_buffer,
            vertex_buf_size,
            index_buffer,
            index_buf_size,
            local_bind_group,
            texture_bind_group_layout,
        }
    }

    // weird lifetimes, self must outlive renderpass
    // as renderpass borrows some of the buffers in self.
    pub fn paint<'a, 'b: 'a>(
        &'b mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'a>,
        meshes: Vec<egui::ClippedMesh>,
        scale_factor: f32,
        physical_height: u32,
        physical_width: u32,
    ) {
        // find the highest number of vertices and whatnot beforehand
        // this is annoying to do in the loop later due to renderpass
        // borrowing the vertex/index buffer, and the overhead
        // of looping through it an extra time is minimal compared to
        // the rest of what we're doing
        // in case of performance issues: verify this claim
        let max_verts = meshes
            .iter()
            .map(|egui::ClippedMesh(_, Mesh { vertices, .. })| vertices.len())
            .max()
            .unwrap_or(0);
        let max_indices = meshes
            .iter()
            .map(|egui::ClippedMesh(_, Mesh { indices, .. })| indices.len())
            .max()
            .unwrap_or(0);

        if max_verts as wgpu::BufferAddress > self.vertex_buf_size {
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("egui vertex buffer"),
                size: (max_verts * mem::size_of::<Vertex>()) as wgpu::BufferAddress,
                mapped_at_creation: false,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.vertex_buf_size = max_verts as wgpu::BufferAddress;
        }
        if max_indices as wgpu::BufferAddress > self.index_buf_size {
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("egui index buffer"),
                size: (max_indices * mem::size_of::<Vertex>()) as wgpu::BufferAddress,
                mapped_at_creation: false,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
            self.index_buf_size = max_indices as wgpu::BufferAddress;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.local_bind_group, &[]);

        for egui::ClippedMesh(clip_rect, mesh) in meshes {
            let texture = if let Some(texture) = self.textures.get(&mesh.texture_id) {
                texture
            } else {
                panic!(
                    "Couldn't find the texture id specified ({:?})",
                    mesh.texture_id
                );
            };

            // code from https://github.com/hasenbanck/egui_wgpu_backend
            // I don't really know how to properly translate the rects
            // in light of logical pixel sizes etc., so I can't verify
            // correctness of this.
            // this is also "important" for not rendering UI that
            // shouldn't be rendered I believe.
            // Transform clip rect to physical pixels.
            let clip_min_x = scale_factor * clip_rect.min.x;
            let clip_min_y = scale_factor * clip_rect.min.y;
            let clip_max_x = scale_factor * clip_rect.max.x;
            let clip_max_y = scale_factor * clip_rect.max.y;

            // Make sure clip rect can fit within an `u32`.
            let clip_min_x = clip_min_x.clamp(0.0, physical_width as f32);
            let clip_min_y = clip_min_y.clamp(0.0, physical_height as f32);
            let clip_max_x = clip_max_x.clamp(clip_min_x, physical_width as f32);
            let clip_max_y = clip_max_y.clamp(clip_min_y, physical_height as f32);

            let clip_min_x = clip_min_x.round() as u32;
            let clip_min_y = clip_min_y.round() as u32;
            let clip_max_x = clip_max_x.round() as u32;
            let clip_max_y = clip_max_y.round() as u32;

            let width = (clip_max_x - clip_min_x).max(1);
            let height = (clip_max_y - clip_min_y).max(1);

            {
                // Clip scissor rectangle to target size.
                let x = clip_min_x.min(physical_width);
                let y = clip_min_y.min(physical_height);
                let width = width.min(physical_width - x);
                let height = height.min(physical_height - y);

                // Skip rendering with zero-sized clip areas.
                if width == 0 || height == 0 {
                    continue;
                }

                pass.set_scissor_rect(x, y, width, height);
            }
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&mesh.indices));
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&mesh.vertices));

            pass.set_bind_group(1, &texture.bind_group, &[]);
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        }
        // TODO: see if we need to reset the scissor to it's previous values
    }

    // create a new texture if it didn't exist earlier
    // this might be better served by using texture::Texture, but I'm not sure
    fn make_tex(
        &mut self,
        id: egui::TextureId,
        device: &wgpu::Device,
        size: wgpu::Extent3d,
    ) -> Texture {
        let tex_name = format!("egui texture {:?}", id);
        let bind_group_name = format!("egui texture {:?} bind group", id);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(&tex_name),
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&bind_group_name),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Texture {
            tex: texture,
            _view: view,
            _sampler: sampler,
            bind_group,
        }
    }

    /// Update all the textures egui is requesting, call this before paint
    pub fn update_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        set: AHashMap<TextureId, ImageDelta>,
    ) {
        for (id, delta) in set {
            let (size, data) = match delta.image {
                egui::epaint::image::ImageData::Color(img) => (img.size, img.pixels),
                // this has worked for now, but I think there might be a better
                // solution than just making a normal RGBA image with all white
                // pixels
                egui::epaint::image::ImageData::Alpha(img) => {
                    let data = img.pixels;
                    let new_data: Vec<_> = data
                        .into_iter()
                        .map(egui::Color32::from_white_alpha)
                        .collect();
                    (img.size, new_data)
                }
            };

            let texture_size = wgpu::Extent3d {
                width: size[0] as u32,
                height: size[1] as u32,
                depth_or_array_layers: 1,
            };

            // there were some annoying lifetime issues and whatnot here
            // this looks really ugly, so should be refactored
            // but I don't really know how exactly. Ahash should support
            // the entry API, possibly we could move away from ahash altogether.
            let tex_exists = self.textures.get(&id).is_some();

            let texture;

            if tex_exists {
                texture = &self.textures.get(&id).unwrap().tex;
            } else {
                let tex = self.make_tex(id, device, texture_size);
                self.textures.insert(id, tex);
                texture = &self.textures.get(&id).unwrap().tex;
            }

            // I think offset is just an index into the image buffer?
            // if it isn't then this is **completely** wrong,
            // and this entire part needs to be rewritten
            let offset = if let Some(pos) = delta.pos {
                pos[0] * pos[1] * 4
            } else {
                0
            } as u64;

            // still assuming offset is just into the flat buffer...
            // TODO: Check that this is actually correct and won't blow up in our faces
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(
                    &data
                        .into_iter()
                        .flat_map(|x| x.to_array().into_iter())
                        .collect::<Vec<_>>(),
                ),
                wgpu::ImageDataLayout {
                    offset,
                    bytes_per_row: std::num::NonZeroU32::new(4 * size[0] as u32),
                    rows_per_image: std::num::NonZeroU32::new(size[1] as u32),
                },
                texture_size,
            );
        }
    }

    /// Free all the textures egui is asking us to free,
    /// call this after paint
    pub fn free_textures(&mut self, free: Vec<TextureId>) {
        for id in free {
            // thankfully WGPU frees these for us
            self.textures.remove(&id);
        }
    }
}
