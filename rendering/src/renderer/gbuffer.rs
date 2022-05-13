use crate::texture;

// macro to make writing out all the bind group entries less tedious
macro_rules! entries {
    [$($location:literal, $tex:ident);+] => {
        &[$(wgpu::BindGroupEntry {
            binding: 2 * $location,
            resource: wgpu::BindingResource::TextureView(&$tex.view),
        },
        wgpu::BindGroupEntry {
            binding: (2 * $location) + 1,
            resource: wgpu::BindingResource::Sampler(&$tex.sampler),
        }),+]
    };
}

pub struct GBuffer {
    pub(crate) diffuse: texture::Texture,
    pub(crate) normal: texture::Texture,
    pub(crate) position: texture::Texture,
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) bind_group: wgpu::BindGroup,
}

impl GBuffer {
    pub fn new(device: &wgpu::Device, resolution: (u32, u32)) -> Self {
        let diffuse = texture::Texture::new_render_target(
            "diffuse texture",
            device,
            resolution,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let normal = texture::Texture::new_render_target(
            "normal texture",
            device,
            resolution,
            wgpu::TextureFormat::Rgba32Float,
        );
        let position = texture::Texture::new_render_target(
            "position texture",
            device,
            resolution,
            wgpu::TextureFormat::Rgba32Float,
        );

        let bind_group_layout =
            device.create_bind_group_layout(&Self::bind_group_layout_descriptor());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("G-Buffer bind group"),
            layout: &bind_group_layout,
            entries: entries![
                0, diffuse;
                1, normal;
                2, position
            ],
        });

        Self {
            diffuse,
            normal,
            position,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, resolution: (u32, u32)) {
        let diffuse = texture::Texture::new_render_target(
            "G-Buffer diffuse texture",
            device,
            resolution,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let normal = texture::Texture::new_render_target(
            "G-Buffer normal texture",
            device,
            resolution,
            wgpu::TextureFormat::Rgba32Float,
        );
        let position = texture::Texture::new_render_target(
            "G-Buffer position texture",
            device,
            resolution,
            wgpu::TextureFormat::Rgba32Float,
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("G-Buffer bind group"),
            layout: &self.bind_group_layout,
            entries: entries![
                0, diffuse;
                1, normal;
                2, position
            ],
        });

        self.diffuse = diffuse;
        self.normal = normal;
        self.position = position;
        self.bind_group = bind_group;
    }

    pub fn bind_group_layout_descriptor<'a>() -> wgpu::BindGroupLayoutDescriptor<'a> {
        use wgpu::BindGroupLayoutEntry as Entry;

        const DEFAULT_TEX: Entry = Entry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        const DEFAULT_SAMPLER: Entry = Entry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        };

        wgpu::BindGroupLayoutDescriptor {
            label: Some("G-Buffer bind group layout"),
            entries: &[
                Entry {
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    ..DEFAULT_TEX
                }, // color texture
                Entry {
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    ..DEFAULT_SAMPLER
                },
                // normal texture
                Entry {
                    binding: 2,
                    ..DEFAULT_TEX
                },
                Entry {
                    binding: 3,
                    ..DEFAULT_SAMPLER
                },
                // position texture
                Entry {
                    binding: 4,
                    ..DEFAULT_TEX
                },
                Entry {
                    binding: 5,
                    ..DEFAULT_SAMPLER
                },
            ],
        }
    }

    pub fn color_attachments<'a>(&'a self) -> [wgpu::RenderPassColorAttachment<'a>; 3] {
        let diffuse = wgpu::RenderPassColorAttachment {
            view: &self.diffuse.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: true,
            },
        };

        let normal = wgpu::RenderPassColorAttachment {
            view: &self.normal.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: true,
            },
        };

        let position = wgpu::RenderPassColorAttachment {
            view: &self.position.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                store: true,
            },
        };

        [diffuse, normal, position]
    }
}
