use crate::components::components::{Material, ObjectRef, Renderable, Shape, Sprite3D, Transform};
use crate::ecs::{entity::Entity, Behaviour};
use crate::engine::engine::Engine;
use crate::gpu::{GpuMesh, GpuTexture, MeshHandle, TextureHandle as GpuTextureHandle, Vertex};
use crate::materials::PbrMaterial;
use crate::rendering::texture::TextureHandle as CpuTextureHandle;
use crate::scene::object::{GpuTriangle, Object};
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

        // Gather sprite entities while holding the query borrow, then release it
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
            if engine.world.get::<ObjectRef>(e).is_none() {
                if let Some(t) = engine.world.get::<Transform>(e).cloned() {
                    let radius = 0.5 * t.size[0].max(t.size[1]);
                    let tris = vec![
                        GpuTriangle {
                            v0: [-0.5, -0.5, 0.0],
                            _pad0: 0.0,
                            e1: [1.0, 0.0, 0.0],
                            _pad1: 0.0,
                            e2: [0.0, 1.0, 0.0],
                            _pad2: 0.0,
                            n0: [0.0, 0.0, 1.0],
                            _pad3: 0.0,
                            n1: [0.0, 0.0, 1.0],
                            _pad4: 0.0,
                            n2: [0.0, 0.0, 1.0],
                            _pad5: 0.0,
                            uv0: [0.0, 0.0],
                            duv1: [1.0, 0.0],
                            duv2: [0.0, 1.0],
                            material_index: 0,
                            _pad6: 0,
                        },
                        GpuTriangle {
                            v0: [-0.5, 0.5, 0.0],
                            _pad0: 0.0,
                            e1: [1.0, -1.0, 0.0],
                            _pad1: 0.0,
                            e2: [1.0, 0.0, 0.0],
                            _pad2: 0.0,
                            n0: [0.0, 0.0, 1.0],
                            _pad3: 0.0,
                            n1: [0.0, 0.0, 1.0],
                            _pad4: 0.0,
                            n2: [0.0, 0.0, 1.0],
                            _pad5: 0.0,
                            uv0: [0.0, 1.0],
                            duv1: [1.0, -1.0],
                            duv2: [1.0, 0.0],
                            material_index: 0,
                            _pad6: 0,
                        },
                    ];

                    let start = engine.scene.triangles.len();
                    engine.scene.add_triangles(tris.clone());
                    let mut nodes = crate::scene::tri_bvh::build_bvh(&tris);
                    let b_start = engine.scene.tri_bvh_nodes.len();
                    crate::scene::tri_bvh::offset_nodes(&mut nodes, b_start as i32);
                    let b_count = nodes.len();
                    engine.scene.add_tri_bvh_nodes(nodes);

                    let mut obj = Object::new(t.position, radius, [1.0, 1.0, 1.0], 1.0, 0.0, false);
                    obj.is_cube = false;
                    obj.is_mesh = true;
                    obj.triangle_start_idx = start;
                    obj.triangle_count = tris.len();
                    obj.tri_bvh_start = b_start;
                    obj.tri_bvh_count = b_count;
                    obj.size = [1.0, 1.0, 0.0];
                    obj.orientation = t.orientation;
                    engine.scene.add_object(obj);
                    let obj_id = (engine.scene.objects.len() - 1) as u32;
                    engine.world.insert(e, ObjectRef { id: obj_id });
                    engine.world.insert(
                        e,
                        Renderable {
                            color: [1.0, 1.0, 1.0],
                            roughness: 1.0,
                            emission: 0.0,
                            is_mesh: true,
                            triangle_start_idx: start as u32,
                            triangle_count: tris.len() as u32,
                        },
                    );
                    engine.world.insert(e, Material::default());
                    engine.world.insert(
                        e,
                        Shape {
                            is_cube: false,
                            radius,
                        },
                    );
                }
            }

            let _ = engine.world.insert(e, quad.clone());

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
