use crate::components::components::{Sprite3D, Transform};
use crate::materials::PbrMaterial;
use crate::ecs::Behaviour;
use crate::engine::engine::Engine;
use crate::gpu::{GpuMesh, MeshHandle, Vertex};
use std::sync::Arc;

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
        for (e, transform, sprite) in engine.world.query2_mut::<Transform, Sprite3D>() {
            if engine.world.get::<MeshHandle>(e).is_none() {
                engine.world.insert(e, quad.clone());
            }
            if engine.world.get::<PbrMaterial>(e).is_none() {
                engine.world.insert(e, PbrMaterial {
                    name: "sprite".into(),
                    base_color: [1.0, 1.0, 1.0, 1.0],
                    metallic: 0.0,
                    roughness: 1.0,
                    emissive: [0.0, 0.0, 0.0],
                    specular_f0: [0.0, 0.0, 0.0],
                    ior: 1.5,
                    opacity: 1.0,
                    base_color_tex: Some(sprite.texture.clone()),
                    metallic_roughness_tex: None,
                    normal_tex: None,
                    occlusion_tex: None,
                    emissive_tex: None,
                });
            }
            transform.size[0] = sprite.size[0];
            transform.size[1] = sprite.size[1];
        }
    }
}
