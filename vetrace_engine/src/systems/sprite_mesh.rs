use crate::components::components::{Sprite3D, Transform};
use crate::ecs::{entity::Entity, Behaviour};
use crate::engine::engine::Engine;
use crate::gpu::{GpuMesh, GpuTexture, MeshHandle, TextureHandle as GpuTextureHandle, Vertex};
use crate::materials::PbrMaterial;
use crate::rendering::texture::TextureHandle as CpuTextureHandle;
use std::sync::Arc;
use wgpu::SamplerDescriptor;

/// System that ensures sprites use regular mesh rendering
#[derive(Default)]
pub struct SpriteMeshSystem {
    quad: Option<MeshHandle>,
}

impl SpriteMeshSystem {
    fn ensure_quad(&mut self, engine: &Engine) -> MeshHandle {
        if let Some(m) = &self.quad {
            return m.clone();
        }
        let device = engine.renderer.device();
        let verts = [
            Vertex {
                pos: [-0.5, -0.5, 0.0],
                nrm: [0.0, 0.0, 1.0],
                tan: [1.0, 0.0, 0.0, 1.0],
                uv: [0.0, 0.0],
                joints: [0; 4],
                weights: [0.0; 4],
            },
            Vertex {
                pos: [0.5, -0.5, 0.0],
                nrm: [0.0, 0.0, 1.0],
                tan: [1.0, 0.0, 0.0, 1.0],
                uv: [1.0, 0.0],
                joints: [0; 4],
                weights: [0.0; 4],
            },
            Vertex {
                pos: [-0.5, 0.5, 0.0],
                nrm: [0.0, 0.0, 1.0],
                tan: [1.0, 0.0, 0.0, 1.0],
                uv: [0.0, 1.0],
                joints: [0; 4],
                weights: [0.0; 4],
            },
            Vertex {
                pos: [0.5, 0.5, 0.0],
                nrm: [0.0, 0.0, 1.0],
                tan: [1.0, 0.0, 0.0, 1.0],
                uv: [1.0, 1.0],
                joints: [0; 4],
                weights: [0.0; 4],
            },
        ];
        let indices = [0u32, 1, 2, 2, 1, 3];
        let mesh = GpuMesh::from_cpu(device, "sprite_quad", &verts, &indices).expect("quad");
        let handle = MeshHandle(Arc::new(mesh));
        self.quad = Some(handle.clone());
        handle
    }
}

impl Behaviour for SpriteMeshSystem {
    fn update(&mut self, engine: &mut Engine, _dt: f32) {
        let quad = self.ensure_quad(engine);

        // Clone data we need while holding the mutable query borrow, then
        // release the borrow before mutating the world again.
        let sprite_data: Vec<(Entity, CpuTextureHandle)> = {
            let mut data = Vec::new();
            for (e, transform, sprite) in engine.world.query2_mut::<Transform, Sprite3D>() {
                transform.size[0] = sprite.size[0];
                transform.size[1] = sprite.size[1];
                data.push((e, sprite.texture.clone()));
            }
            data
        };

        let device = engine.renderer.device();
        for (e, tex) in sprite_data {
            // Always replace the mesh with the quad to guarantee sprites render
            let _ = engine.world.insert(e, quad.clone());

            // Build a basic PBR material with the sprite texture every update
            let view = tex
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = device.create_sampler(&SamplerDescriptor {
                label: Some("sprite_sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });
            let size = tex.texture.size();
            let format = tex.texture.format();
            let tex_handle = GpuTextureHandle(Arc::new(GpuTexture {
                view,
                sampler,
                format,
                size,
                is_srgb: true,
            }));
            let _ = engine.world.insert(
                e,
                PbrMaterial {
                    name: "sprite".into(),
                    base_color: [1.0, 1.0, 1.0, 1.0],
                    metallic: 0.0,
                    roughness: 1.0,
                    emissive: [0.0, 0.0, 0.0],
                    specular_f0: [0.0, 0.0, 0.0],
                    ior: 1.5,
                    opacity: 1.0,
                    base_color_tex: Some(tex_handle),
                    metallic_roughness_tex: None,
                    normal_tex: None,
                    occlusion_tex: None,
                    emissive_tex: None,
                },
            );
        }
    }
}
