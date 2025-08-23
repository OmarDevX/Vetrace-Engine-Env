use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use ahash::HashMap;
use anyhow::Context;
use parking_lot::RwLock;

use glam::{Mat4, Vec4};
use gltf::animation::util::ReadOutputs;

use crate::components::components::{Animation, MorphTargets, MorphWeights};
use crate::gpu::{GpuMesh, GpuTexture, MeshHandle, TextureHandle, Vertex};
use crate::materials::PbrMaterial;
use crate::scene::object::{GpuTriangle, Object};
use crate::Engine;

#[derive(Default)]
struct MeshAccum {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    triangles: Vec<GpuTriangle>,
    morph_targets: HashMap<String, MorphTargetSet>,
}

#[derive(Clone, Debug)]
pub enum AnimationChannel {
    Translation(Vec<(f32, [f32; 3])>),
    Rotation(Vec<(f32, [f32; 4])>), // quaternion [x, y, z, w]
    Scale(Vec<(f32, [f32; 3])>),
    MorphTargetWeights(Vec<(f32, Vec<f32>)>), // time, weights for each morph target
}

#[derive(Clone, Debug)]
pub struct MorphTarget {
    pub name: String,
    pub vertex_positions: Vec<[f32; 3]>, // Delta positions for each vertex
    pub vertex_normals: Option<Vec<[f32; 3]>>, // Optional delta normals
}

#[derive(Clone, Default)]
pub struct MorphTargetSet {
    pub targets: Vec<MorphTarget>,
    pub base_vertex_count: usize,
}

#[derive(Clone, Default)]
pub struct AnimationClip {
    pub channels: Vec<AnimationChannel>,
    pub duration: f32,
}

pub struct AssetManager {
    root: PathBuf,
    meshes: RwLock<HashMap<String, MeshHandle>>,
    materials: RwLock<HashMap<String, PbrMaterial>>,
    textures: RwLock<HashMap<String, TextureHandle>>,
    animations: RwLock<HashMap<String, AnimationClip>>,
    morph_targets: RwLock<HashMap<String, MorphTargetSet>>,
}

impl AssetManager {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            meshes: Default::default(),
            materials: Default::default(),
            textures: Default::default(),
            animations: Default::default(),
            morph_targets: Default::default(),
        }
    }
    #[cfg(feature = "wgpu")]
    pub fn load_gltf_pbr(
        &self,
        engine: &mut Engine,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<u32> {
        let abs = self.root.join(path.as_ref());
        let data = std::fs::read(&abs).with_context(|| format!("read {:?}", abs))?;
        let gltf = gltf::Gltf::from_slice(&data)?;
        let blob = gltf.blob.as_ref().map(|b| &**b);
        let base_dir = abs.parent().unwrap_or(&self.root);
        let materials = {
            let device = engine.renderer.device();
            let queue = engine.renderer.queue();

            let mut image_bytes: Vec<Vec<u8>> = Vec::new();
            for img in gltf.images() {
                let bytes = load_image_bytes(&img, &data, blob, base_dir)
                    .with_context(|| format!("image {:?}", img.index()))?;
                image_bytes.push(bytes);
            }

            let tex_from_image =
                |idx: usize, srgb: bool, label: &str| -> anyhow::Result<TextureHandle> {
                    let key = format!("{}#img{}", abs.display(), idx);
                    if let Some(h) = self.textures.read().get(&key) {
                        return Ok(h.clone());
                    }
                    let img = image::load_from_memory(&image_bytes[idx])?.to_rgba8();
                    let (w, h) = img.dimensions();
                    let tex = GpuTexture::from_rgba8(device, queue, &img, w, h, srgb, label)?;
                    let handle = TextureHandle(Arc::new(tex));
                    self.textures.write().insert(key, handle.clone());
                    Ok(handle)
                };

            let mut mats = Vec::new();
            for m in gltf.materials() {
                let name = m
                    .name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("material_{}", m.index().unwrap_or(0)));
                let pbr = m.pbr_metallic_roughness();

                let mut base_color = pbr.base_color_factor();
                for i in 0..3 {
                    base_color[i] = srgb_to_linear(base_color[i]);
                }
                let base_color_tex = pbr
                    .base_color_texture()
                    .map(|info| {
                        let t = info.texture();
                        let img_idx = t.source().index();
                        tex_from_image(img_idx, true, &format!("{name}:base"))
                    })
                    .transpose()?;

                let metallic = pbr.metallic_factor();
                let roughness = pbr.roughness_factor();
                let metallic_roughness_tex = pbr
                    .metallic_roughness_texture()
                    .map(|info| {
                        let t = info.texture();
                        let img_idx = t.source().index();
                        tex_from_image(img_idx, false, &format!("{name}:mr"))
                    })
                    .transpose()?;

                let normal_tex = m
                    .normal_texture()
                    .map(|n| {
                        tex_from_image(n.texture().source().index(), false, &format!("{name}:norm"))
                    })
                    .transpose()?;

                let occlusion_tex = m
                    .occlusion_texture()
                    .map(|ao| {
                        tex_from_image(ao.texture().source().index(), false, &format!("{name}:ao"))
                    })
                    .transpose()?;

                let mut emissive = m.emissive_factor();
                for i in 0..3 {
                    emissive[i] = srgb_to_linear(emissive[i]);
                }
                let emissive_tex = m
                    .emissive_texture()
                    .map(|e| {
                        tex_from_image(e.texture().source().index(), true, &format!("{name}:em"))
                    })
                    .transpose()?;

                let ior = m.ior().unwrap_or(1.5);
                let opacity = base_color[3];

                let mat = PbrMaterial {
                    name: name.clone(),
                    base_color,
                    metallic,
                    roughness,
                    emissive,
                    specular_f0: [0.0; 3],
                    ior,
                    opacity,
                    base_color_tex,
                    metallic_roughness_tex,
                    normal_tex,
                    occlusion_tex,
                    emissive_tex,
                };
                self.materials.write().insert(name.clone(), mat.clone());
                mats.push(mat);
            }
            mats
        };
        // TODO: Fix this when we integrate scene management with new core engine
        // engine.scene.materials.extend(materials.clone());

        let mut buffers_data: Vec<Vec<u8>> = Vec::new();
        for buf in gltf.buffers() {
            if let Some(bytes) = get_buffer_slice(&gltf, &data, blob, base_dir, buf.index()) {
                buffers_data.push(bytes);
            } else {
                buffers_data.push(Vec::new());
            }
        }

        let mut first_clip: Option<String> = None;
        for anim in gltf.animations() {
            let name = anim
                .name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}#anim{}", abs.display(), anim.index()));
            let mut clip = AnimationClip::default();

            for channel in anim.channels() {
                let reader = channel.reader(|b| buffers_data.get(b.index()).map(|v| v.as_slice()));
                if let (Some(inputs), Some(outputs)) = (reader.read_inputs(), reader.read_outputs())
                {
                    let times: Vec<f32> = inputs.collect();
                    clip.duration = times.iter().copied().fold(clip.duration, f32::max);

                    match channel.target().property() {
                        gltf::animation::Property::Translation => {
                            if let ReadOutputs::Translations(tr) = outputs {
                                let values: Vec<[f32; 3]> = tr.collect();
                                let keyframes = times.into_iter().zip(values.into_iter()).collect();
                                clip.channels.push(AnimationChannel::Translation(keyframes));
                            }
                        }
                        gltf::animation::Property::Rotation => {
                            if let ReadOutputs::Rotations(rot) = outputs {
                                let values: Vec<[f32; 4]> = rot.into_f32().collect();
                                let keyframes = times.into_iter().zip(values.into_iter()).collect();
                                clip.channels.push(AnimationChannel::Rotation(keyframes));
                            }
                        }
                        gltf::animation::Property::Scale => {
                            if let ReadOutputs::Scales(sc) = outputs {
                                let values: Vec<[f32; 3]> = sc.collect();
                                let keyframes = times.into_iter().zip(values.into_iter()).collect();
                                clip.channels.push(AnimationChannel::Scale(keyframes));
                            }
                        }
                        gltf::animation::Property::MorphTargetWeights => {
                            // TODO: Implement morph target weight animation loading
                            // For now, skip morph target weight animations
                            println!("Found morph target weight animation - skipping for now");
                        }
                    }
                }
            }

            if !clip.channels.is_empty() {
                if first_clip.is_none() {
                    first_clip = Some(name.clone());
                }
                self.animations.write().insert(name, clip);
            }
        }

        let file_path_str = format!("{}", abs.display());
        let mut first_id: Option<u32> = None;
        for scene in gltf.scenes() {
            for node in scene.nodes() {
                let ids = spawn_gltf_node(
                    self,
                    engine,
                    node,
                    Mat4::IDENTITY,
                    &buffers_data,
                    &materials,
                    first_clip.as_deref(),
                    &file_path_str,
                )?;
                if first_id.is_none() {
                    first_id = ids.first().copied();
                }
            }
        }

        Ok(first_id.unwrap_or(0))
    }
}

impl AssetManager {
    pub fn get_animation(&self, name: &str) -> Option<AnimationClip> {
        self.animations.read().get(name).cloned()
    }

    /// Returns a list of all loaded animation names.
    pub fn animation_names(&self) -> Vec<String> {
        self.animations.read().keys().cloned().collect()
    }

    pub fn get_morph_targets(&self, key: &str) -> Option<MorphTargetSet> {
        self.morph_targets.read().get(key).cloned()
    }

    /// Returns a list of all loaded morph target keys.
    pub fn morph_target_keys(&self) -> Vec<String> {
        self.morph_targets.read().keys().cloned().collect()
    }
}

fn spawn_gltf_node(
    assets: &AssetManager,
    engine: &mut Engine,
    node: gltf::Node,
    parent: Mat4,
    buffers_data: &Vec<Vec<u8>>,
    materials: &Vec<PbrMaterial>,
    first_clip: Option<&str>,
    file_path: &str,
) -> anyhow::Result<Vec<u32>> {
    let local = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world = parent * local;
    let (scale, rot, _trans) = world.to_scale_rotation_translation();
    let mut ids = Vec::new();

    if let Some(mesh) = node.mesh() {
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| buffers_data.get(b.index()).map(|v| v.as_slice()));

            let positions: Vec<[f32; 3]> = reader.read_positions().context("positions")?.collect();
            let normals: Vec<[f32; 3]> = if let Some(n) = reader.read_normals() {
                n.collect()
            } else {
                vec![[0.0, 1.0, 0.0]; positions.len()]
            };
            let texcoords: Vec<[f32; 2]> = if let Some(t) = reader.read_tex_coords(0) {
                t.into_f32().collect()
            } else {
                vec![[0.0, 0.0]; positions.len()]
            };
            let tangents: Vec<[f32; 4]> = if let Some(t) = reader.read_tangents() {
                t.map(|v| [v[0], v[1], v[2], v[3]]).collect()
            } else {
                vec![[1.0, 0.0, 0.0, 1.0]; positions.len()]
            };
            let indices: Vec<u32> = if let Some(ind) = reader.read_indices() {
                ind.into_u32().collect()
            } else {
                (0..positions.len() as u32).collect()
            };

            let mut min = [f32::MAX; 3];
            let mut max = [f32::MIN; 3];
            for p in &positions {
                for i in 0..3 {
                    if p[i] < min[i] {
                        min[i] = p[i];
                    }
                    if p[i] > max[i] {
                        max[i] = p[i];
                    }
                }
            }
            let center = [
                (min[0] + max[0]) * 0.5,
                (min[1] + max[1]) * 0.5,
                (min[2] + max[2]) * 0.5,
            ];

            let mut vertices = Vec::with_capacity(positions.len());
            for (i, p) in positions.iter().enumerate() {
                vertices.push(Vertex {
                    pos: [p[0] - center[0], p[1] - center[1], p[2] - center[2]],
                    nrm: normals[i],
                    tan: tangents[i],
                    uv: texcoords[i],
                });
            }

            let mut tris = Vec::new();
            for idx in indices.chunks_exact(3) {
                let i0 = idx[0] as usize;
                let i1 = idx[1] as usize;
                let i2 = idx[2] as usize;
                let v0 = &vertices[i0];
                let v1 = &vertices[i1];
                let v2 = &vertices[i2];
                let e1 = [
                    v1.pos[0] - v0.pos[0],
                    v1.pos[1] - v0.pos[1],
                    v1.pos[2] - v0.pos[2],
                ];
                let e2 = [
                    v2.pos[0] - v0.pos[0],
                    v2.pos[1] - v0.pos[1],
                    v2.pos[2] - v0.pos[2],
                ];
                let duv1 = [v1.uv[0] - v0.uv[0], v1.uv[1] - v0.uv[1]];
                let duv2 = [v2.uv[0] - v0.uv[0], v2.uv[1] - v0.uv[1]];
                tris.push(GpuTriangle {
                    v0: v0.pos,
                    _pad0: 0.0,
                    e1,
                    _pad1: 0.0,
                    e2,
                    _pad2: 0.0,
                    n0: v0.nrm,
                    _pad3: 0.0,
                    n1: v1.nrm,
                    _pad4: 0.0,
                    n2: v2.nrm,
                    _pad5: 0.0,
                    uv0: v0.uv,
                    duv1,
                    duv2,
                    material_index: u32::MAX,
                    _pad6: 0,
                });
            }

            let name = format!("{}#node{}#prim{}", file_path, node.index(), prim.index());
            let gm = GpuMesh::from_cpu_with_morph_targets(
                engine.renderer.device(),
                &name,
                &vertices,
                &indices,
                None,
            )?;
            let handle = MeshHandle(Arc::new(gm));
            assets.meshes.write().insert(name, handle.clone());

            let mut obj = Object::default();
            obj.is_cube = false;
            let wc = world * Vec4::new(center[0], center[1], center[2], 1.0);
            obj.position = [wc.x, wc.y, wc.z];
            obj.orientation = [rot.x, rot.y, rot.z, rot.w];
            obj.scale = [scale.x, scale.y, scale.z];
            engine.spawn_with_triangles(obj, tris.clone());
            let id = (engine.scene.objects.len() - 1) as u32;
            if let Some(entity) = engine.core.find_entity_by_object_id(id) {
                engine.world.insert(entity, handle);
                if let Some(mi) = prim.material().index().and_then(|i| materials.get(i)) {
                    engine.world.insert(entity, mi.clone());
                }
                if let Some(anim_name) = first_clip {
                    engine.world.insert(
                        entity,
                        Animation {
                            clip: anim_name.to_string(),
                            ..Default::default()
                        },
                    );
                }
            }
            ids.push(id);
        }
    }

    for child in node.children() {
        ids.extend(spawn_gltf_node(
            assets,
            engine,
            child,
            world,
            buffers_data,
            materials,
            first_clip,
            file_path,
        )?);
    }

    Ok(ids)
}

fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

fn load_image_bytes(
    img: &gltf::image::Image,
    file_bytes: &[u8],
    blob: Option<&[u8]>,
    base_dir: &Path,
) -> anyhow::Result<Vec<u8>> {
    use gltf::image::Source;
    Ok(match img.source() {
        Source::Uri { uri, .. } => {
            if let Some(bin) = uri.strip_prefix("data:") {
                let comma = bin.find(',').context("bad data uri")?;
                let (_header, data) = bin.split_at(comma + 1);
                base64::decode(data)?
            } else {
                std::fs::read(base_dir.join(uri))?
            }
        }
        Source::View { view, mime_type: _ } => {
            let start = view.offset();
            let end = start + view.length();
            let buffer = view.buffer();
            match buffer.source() {
                gltf::buffer::Source::Bin => {
                    let b = blob.context("no GLB blob")?;
                    b[start..end].to_vec()
                }
                gltf::buffer::Source::Uri(uri) => {
                    let bytes = if uri.starts_with("data:") {
                        let comma = uri.find(',').context("bad buffer data uri")?;
                        base64::decode(&uri[comma + 1..])?
                    } else {
                        std::fs::read(base_dir.join(uri))?
                    };
                    bytes[start..end].to_vec()
                }
            }
        }
    })
}

fn get_buffer_slice(
    gltf: &gltf::Gltf,
    file_bytes: &[u8],
    blob: Option<&[u8]>,
    base_dir: &Path,
    index: usize,
) -> Option<Vec<u8>> {
    let buf = gltf.buffers().nth(index)?;
    match buf.source() {
        gltf::buffer::Source::Bin => blob.map(|b| b.to_vec()),
        gltf::buffer::Source::Uri(uri) => {
            if uri.starts_with("data:") {
                let comma = uri.find(',')?;
                base64::decode(&uri[comma + 1..]).ok()
            } else {
                std::fs::read(base_dir.join(uri)).ok()
            }
        }
    }
}
