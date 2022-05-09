use std::collections::HashMap;
use std::mem;
use std::ops::{Range, RangeBounds};
use std::path::Path;

use tobj::LoadOptions;
use wgpu::util::DeviceExt;

use super::texture;
use crate::error::LoadError;
use crate::range::range as slice_range;
use crate::renderer::RawTranslationMatrix;

use common::Transform;

pub type ModelIndex = usize;

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct Material {
    pub name: String,
    // TODO: load normals
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct ModelBatch {
    pub models: Vec<IndirectModel>,
    pub materials: Vec<IndirectMaterial>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub diffuse_textures: Vec<texture::Texture>,
    pub normal_textures: Vec<texture::Texture>,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct IndirectModel {
    pub meshes: Vec<IndirectMesh>,
}

#[derive(Debug)]
pub struct IndirectMaterial {
    pub name: String,
    pub diffuse_texture_index: usize,
    pub normal_texture_index: usize,
}

#[derive(Debug)]
pub struct IndirectMesh {
    pub name: String,
    pub vtx_start: wgpu::BufferAddress,
    pub idx_start: wgpu::BufferAddress,
    pub material_index: usize,
    pub num_elements: u32,
    pub num_vertices: u32,
}

impl ModelBatch {
    pub fn from_model_files<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        paths: Vec<P>,
    ) -> Result<Self, LoadError> {
        let mut materials = Vec::new();

        let mut diffuse_textures = Vec::new();
        let mut normal_textures = Vec::new();

        let mut model_buffers = Vec::new();

        for path in paths {
            let (obj_models, obj_materials) = tobj::load_obj(
                path.as_ref(),
                &LoadOptions {
                    triangulate: true,
                    single_index: true,
                    ..Default::default()
                },
            )?;
            let containing_folder = path.as_ref().parent().ok_or(LoadError::Missing)?;

            let obj_materials = obj_materials?;

            let m_start = materials.len();
            for mat in obj_materials {
                let name = mat.name.clone();
                let (diffuse_texture, normal_texture) =
                    load_material(device, queue, mat, containing_folder)?;

                let diffuse_texture_index = diffuse_textures.len();
                diffuse_textures.push(diffuse_texture);
                let normal_texture_index = normal_textures.len();
                normal_textures.push(normal_texture);

                materials.push(IndirectMaterial {
                    name,
                    diffuse_texture_index,
                    normal_texture_index,
                });
            }

            let mut vertex_buf = Vec::new();
            let mut index_buf = Vec::new();
            let mut meshes = Vec::new();

            for mut m in obj_models {
                let mut vertices = load_vertices(&m);

                let vtx_start = vertex_buf.len() as wgpu::BufferAddress;
                let idx_start = index_buf.len() as wgpu::BufferAddress;
                let num_elements = m.mesh.indices.len() as u32;
                let num_vertices = vertices.len() as u32;

                vertex_buf.append(&mut vertices);
                index_buf.append(&mut m.mesh.indices);

                meshes.push(IndirectMesh {
                    name: m.name,
                    vtx_start,
                    idx_start,
                    material_index: m.mesh.material_id.map(|id| id + m_start).unwrap_or(0),
                    num_elements,
                    num_vertices,
                });
            }

            model_buffers.push((meshes, vertex_buf, index_buf))
        }

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Model batch bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: Some(make_nzu32(diffuse_textures.len() as u32)),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: Some(make_nzu32(diffuse_textures.len() as u32)),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: Some(make_nzu32(normal_textures.len() as u32)),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: Some(make_nzu32(normal_textures.len() as u32)),
                },
            ],
        });
        let diffuse_array = diffuse_textures
            .iter()
            .map(|tex| &tex.view)
            .collect::<Vec<_>>();
        let sampler_array = diffuse_textures
            .iter()
            .map(|tex| &tex.sampler)
            .collect::<Vec<_>>();

        let normal_array = normal_textures
            .iter()
            .map(|tex| &tex.view)
            .collect::<Vec<_>>();
        let normal_sampler_array = normal_textures
            .iter()
            .map(|tex| &tex.sampler)
            .collect::<Vec<_>>();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model batch bind group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&diffuse_array),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::SamplerArray(&sampler_array),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureViewArray(&normal_array),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::SamplerArray(&normal_sampler_array),
                },
            ],
        });

        let mut full_vtx_buffer = Vec::new();
        let mut full_idx_buffer = Vec::new();
        let mut models = Vec::new();
        let mut vtx_offset = 0;
        let mut idx_offset = 0;
        for (mut meshes, mut vertices, mut indices) in model_buffers {
            for indirect_mesh in &mut meshes {
                indirect_mesh.vtx_start += vtx_offset;
                indirect_mesh.idx_start += idx_offset;
            }
            models.push(IndirectModel { meshes });
            full_vtx_buffer.append(&mut vertices);
            full_idx_buffer.append(&mut indices);
            vtx_offset = full_vtx_buffer.len() as u64;
            idx_offset = full_idx_buffer.len() as u64;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model batch vertex buffer"),
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&full_vtx_buffer),
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("model batch index buffer"),
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(&full_idx_buffer),
        });

        Ok(ModelBatch {
            models,
            materials,
            diffuse_textures,
            normal_textures,
            vertex_buffer,
            index_buffer,
            bind_group,
        })
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        path: P,
    ) -> Result<Self, LoadError> {
        let (obj_models, obj_materials) = tobj::load_obj(
            path.as_ref(),
            &LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;

        let obj_materials = obj_materials?;

        // We're assuming that the texture files are stored with the obj file
        let containing_folder = path.as_ref().parent().ok_or(LoadError::Missing)?;

        let mut materials = Vec::new();
        for mat in obj_materials {
            let name = mat.name.clone();
            let (diffuse_texture, normal_texture) =
                load_material(device, queue, mat, containing_folder)?;

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                    },
                ],
                label: None,
            });

            materials.push(Material {
                name,
                diffuse_texture,
                normal_texture,
                bind_group,
            });
        }

        let mut meshes = Vec::with_capacity(obj_models.len());
        for m in obj_models {
            let vertices = load_vertices(&m);

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", path.as_ref())),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", path.as_ref())),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            meshes.push(Mesh {
                name: m.name,
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            });
        }

        Ok(Self { meshes, materials })
    }
}

fn load_material(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mat: tobj::Material,
    containing_folder: &Path,
) -> Result<(texture::Texture, texture::Texture), LoadError> {
    let diffuse_path = mat.diffuse_texture;
    let diffuse_texture =
        texture::Texture::load(device, queue, containing_folder.join(diffuse_path))?;

    let normal_path = mat.normal_texture;
    let normal_texture = if normal_path == "" {
        let img = image::Rgb32FImage::from_pixel(1, 1, image::Rgb([0.0, 0.0, 1.0]));
        texture::Texture::from_image(
            device,
            queue,
            &image::DynamicImage::ImageRgb32F(img),
            Some("normal texture"),
        )
    } else {
        texture::Texture::load(device, queue, containing_folder.join(normal_path))?
    };

    Ok((diffuse_texture, normal_texture))
}

fn load_vertices(m: &tobj::Model) -> Vec<ModelVertex> {
    let mut vertices = Vec::new();
    for i in 0..m.mesh.positions.len() / 3 {
        vertices.push(ModelVertex {
            position: [
                m.mesh.positions[i * 3],
                m.mesh.positions[i * 3 + 1],
                m.mesh.positions[i * 3 + 2],
            ],
            tex_coords: [
                *m.mesh.texcoords.get(i * 2).unwrap_or(&0.0f32),
                *m.mesh.texcoords.get(i * 2 + 1).unwrap_or(&0.0f32),
            ],
            normal: [
                m.mesh.normals[i * 3],
                m.mesh.normals[i * 3 + 1],
                m.mesh.normals[i * 3 + 2],
            ],
        });
    }
    vertices
}

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }
}

#[derive(Debug, Default)]
pub struct ModelStorage {
    models: Vec<Model>,
    instances: Vec<Vec<Transform>>,
    instance_buffers: Vec<wgpu::Buffer>,
}

#[derive(Debug)]
pub struct ModelManager<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    storage: &'a mut ModelStorage,
    pending_writes: HashMap<ModelIndex, Vec<Transform>>,
}

impl<'a> ModelManager<'a> {
    pub fn get_transforms_mut<R: RangeBounds<usize>>(
        &mut self,
        model: ModelIndex,
        range: R,
    ) -> &mut [Transform] {
        self.pending_writes
            .entry(model)
            .or_insert_with(|| self.storage.get_transforms(model, range).to_vec())
    }

    pub fn set_transforms(&mut self, model: ModelIndex, transforms: Vec<Transform>) {
        self.pending_writes.insert(model, transforms);
    }
}

impl<'a> Drop for ModelManager<'a> {
    fn drop(&mut self) {
        for (model, data) in &mut self.pending_writes.drain() {
            self.storage
                .set_transforms(self.device, self.queue, model, data);
        }
    }
}

impl ModelStorage {
    pub fn new() -> Self {
        Self {
            models: vec![],
            instances: vec![],
            instance_buffers: vec![],
        }
    }

    pub fn get_manager<'a>(
        &'a mut self,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
    ) -> ModelManager<'a> {
        ModelManager {
            device,
            queue,
            storage: self,
            pending_writes: HashMap::new(),
        }
    }

    pub fn add_model(
        &mut self,
        device: &wgpu::Device,
        model: Model,
        n_instances: u64,
    ) -> ModelIndex {
        let idx = self.models.len();
        self.models.push(model);
        self.instances.push(vec![]);
        self.instance_buffers
            .push(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Instance buffer {}", self.models.len())),
                // size of a 4x4 matrix of f32s
                size: n_instances * 4 * 4 * mem::size_of::<f32>() as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            }));
        idx
    }

    pub fn get_transforms<R: RangeBounds<usize>>(
        &self,
        model: ModelIndex,
        range: R,
    ) -> &[Transform] {
        &self.instances[model][slice_range(range, ..self.instances[model].len())]
    }

    pub fn modify_transforms_with<F, R: RangeBounds<usize>>(
        &mut self,
        model: ModelIndex,
        range: R,
        f: F,
        queue: &wgpu::Queue,
    ) where
        F: FnOnce(&mut [Transform]),
    {
        let Range { start, end } = slice_range(range, ..self.instances[model].len());
        f(&mut self.instances[model][start..end]);
        let raw: Vec<_> = self.instances[model][start..end]
            .iter()
            .copied()
            .map(RawTranslationMatrix::new)
            .collect();
        queue.write_buffer(
            &self.instance_buffers[model],
            start as u64 * mem::size_of::<RawTranslationMatrix>() as u64,
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
        let raw: Vec<_> = new_transforms
            .iter()
            .copied()
            .map(RawTranslationMatrix::new)
            .collect();
        if old_len < raw.len() {
            let new_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Instance buffer for model {}", model)),
                contents: bytemuck::cast_slice(&raw),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.instance_buffers[model] = new_buffer;
        } else {
            queue.write_buffer(&self.instance_buffers[model], 0, bytemuck::cast_slice(&raw));
        }
        self.instances[model] = new_transforms;
    }

    pub fn models(&self) -> &[Model] {
        self.models.as_ref()
    }

    pub fn instances(&self) -> &[Vec<Transform>] {
        self.instances.as_ref()
    }

    pub fn instance_buffers(&self) -> &[wgpu::Buffer] {
        self.instance_buffers.as_ref()
    }
}

fn make_nzu32(n: u32) -> std::num::NonZeroU32 {
    std::num::NonZeroU32::new(n).unwrap()
}
