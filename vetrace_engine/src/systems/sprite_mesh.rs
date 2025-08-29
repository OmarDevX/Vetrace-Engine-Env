use crate::components::components::{Sprite3D, Transform};
use crate::materials::PbrMaterial;
use crate::ecs::Behaviour;
use crate::engine::engine::Engine;
use crate::gpu::{GpuMesh, MeshHandle, TextureHandle as GpuTextureHandle, GpuTexture, Vertex};
use std::sync::Arc;
use wgpu::SamplerDescriptor;

/// System that ensures sprites use regular mesh rendering
#[derive(Default)]
pub struct SpriteMeshSystem {
    quad: Option<MeshHandle>,
}

impl SpriteMeshSystem {
    fn ensure_quad(&mut self, engine: &Engine) -> MeshHandle {
        if let Some(m) = &self.quad { return m.clone(); }
        let device = engine.renderer.device();
        let verts = [
            Vertex { pos: [-0.5, -0.5, 0.0], nrm: [0.0, 0.0, 1.0], tan: [1.0, 0.0, 0.0, 1.0], uv: [0.0, 0.0], joints: [0;4], weights:[0.0;4] },
            Vertex { pos: [ 0.5, -0.5, 0.0], nrm: [0.0, 0.0, 1.0], tan: [1.0, 0.0, 0.0, 1.0], uv: [1.0, 0.0], joints: [0;4], weights:[0.0;4] },
            Vertex { pos: [-0.5,  0.5, 0.0], nrm: [0.0, 0.0, 1.0], tan: [1.0, 0.0, 0.0, 1.0], uv: [0.0, 1.0], joints: [0;4], weights:[0.0;4] },
            Vertex { pos: [ 0.5,  0.5, 0.0], nrm: [0.0, 0.0, 1.0], tan: [1.0, 0.0, 0.0, 1.0], uv: [1.0, 1.0], joints: [0;4], weights:[0.0;4] },
        ];
        let indices = [0u32,1,2,2,1,3];
        let mesh = GpuMesh::from_cpu(device, "sprite_quad", &verts, &indices).expect("quad");
        let handle = MeshHandle(Arc::new(mesh));
        self.quad = Some(handle.clone());
        handle
    }
}

impl Behaviour for SpriteMeshSystem {
    fn update(&mut self, engine: &mut Engine, _dt: f32) {
        let quad = self.ensure_quad(engine);
        let mut needs_mesh = Vec::new();
        let mut needs_material = Vec::new();

        {
            // Gather entities that require mesh/material insertion while we have a
            // query borrow, but postpone the actual mutations until after the
            // borrow ends to satisfy the Rust borrow checker.
            let mut query = engine
                .world
                .query::<(&mut Transform, &Sprite3D, Option<&MeshHandle>, Option<&PbrMaterial>)>();
            for (e, (transform, sprite, mesh, material)) in query.iter() {
                transform.size[0] = sprite.size[0];
                transform.size[1] = sprite.size[1];
                if mesh.is_none() {
                    needs_mesh.push(e);
                }
                if material.is_none() {
                    needs_material.push((e, sprite.texture.clone()));
                }
            }
        }

        for e in needs_mesh {
            let _ = engine.world.insert(e, quad.clone());
        }

        if !needs_material.is_empty() {
            let device = engine.renderer.device();
            for (e, tex) in needs_material {
                let view = tex.texture.create_view(&wgpu::TextureViewDescriptor::default());
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
}
