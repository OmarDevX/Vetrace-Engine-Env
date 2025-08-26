use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use ahash::HashMap;
use anyhow::Context;
use parking_lot::RwLock;

use glam::{Mat4, Vec3, Vec4};
use gltf::animation::util::ReadOutputs;

use crate::components::components::{Animation, MorphTargets, MorphWeights, Skin};
use crate::gpu::{GpuMesh, GpuTexture, MeshHandle, TextureHandle, Vertex};
use crate::materials::PbrMaterial;
use crate::scene::object::{GpuTriangle, Object};
use crate::Engine;

struct MeshAccum {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    triangles: Vec<GpuTriangle>,
    morph_targets: Option<MorphTargetSet>,
    prim_ranges: Vec<PrimitiveRange>,
    min: [f32; 3],
    max: [f32; 3],
}

impl Default for MeshAccum {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            triangles: Vec::new(),
            morph_targets: None,
            prim_ranges: Vec::new(),
            min: [f32::MAX; 3],
            max: [f32::MIN; 3],
        }
    }
}

#[derive(Default, Clone, Copy)]
struct PrimitiveRange {
    start: usize,
    count: usize,
    material_index: u32,
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
        // Register all materials in the engine scene so triangles can reference
        // them directly by index when glTF nodes are spawned.
        let material_base = engine.scene.materials.len() as u32;
        engine.scene.materials.extend(materials.clone());

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
                            if let ReadOutputs::MorphTargetWeights(weights) = outputs {
                                let frame_count = times.len().max(1);
                                let total_weights = weights.clone().into_f32().count();
                                let targets_per_frame = total_weights / frame_count;
                                let mut values = Vec::with_capacity(frame_count);
                                let mut iter = weights.into_f32();
                                for _ in 0..frame_count {
                                    let mut frame = Vec::with_capacity(targets_per_frame);
                                    for _ in 0..targets_per_frame {
                                        if let Some(w) = iter.next() {
                                            frame.push(w);
                                        }
                                    }
                                    values.push(frame);
                                }
                                let keyframes = times.into_iter().zip(values.into_iter()).collect();
                                clip.channels
                                    .push(AnimationChannel::MorphTargetWeights(keyframes));
                            }
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
        let mut accum = MeshAccum::default();
        for scene in gltf.scenes() {
            for node in scene.nodes() {
                accumulate_gltf_node(
                    &mut accum,
                    node,
                    Mat4::IDENTITY,
                    &buffers_data,
                    material_base,
                )?;
            }
        }

        let center = [
            (accum.min[0] + accum.max[0]) * 0.5,
            (accum.min[1] + accum.max[1]) * 0.5,
            (accum.min[2] + accum.max[2]) * 0.5,
        ];
        for v in &mut accum.vertices {
            v.pos[0] -= center[0];
            v.pos[1] -= center[1];
            v.pos[2] -= center[2];
        }

        for prim in &accum.prim_ranges {
            let slice = &accum.indices[prim.start..prim.start + prim.count];
            for idx in slice.chunks_exact(3) {
                let i0 = idx[0] as usize;
                let i1 = idx[1] as usize;
                let i2 = idx[2] as usize;
                let v0 = &accum.vertices[i0];
                let v1 = &accum.vertices[i1];
                let v2 = &accum.vertices[i2];
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
                accum.triangles.push(GpuTriangle {
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
                    material_index: prim.material_index,
                    _pad6: 0,
                });
            }
        }

        if let Some(ref mut set) = accum.morph_targets {
            set.base_vertex_count = accum.vertices.len();
        }
        let morph_key = accum
            .morph_targets
            .as_ref()
            .map(|_| format!("morph:{}", file_path_str));

        let gm = GpuMesh::from_cpu_with_morph_targets(
            engine.renderer.device(),
            &file_path_str,
            &accum.vertices,
            &accum.indices,
            accum.morph_targets.as_ref(),
        )?;
        let handle = MeshHandle(Arc::new(gm));
        self.meshes
            .write()
            .insert(file_path_str.clone(), handle.clone());
        if let (Some(key), Some(set)) = (morph_key.as_ref(), accum.morph_targets.clone()) {
            self.morph_targets.write().insert(key.clone(), set);
        }

        let mut obj = Object::default();
        obj.is_cube = false;
        obj.material_index = 0;
        obj.position = center;
        obj.orientation = [0.0, 0.0, 0.0, 1.0];
        obj.scale = [1.0, 1.0, 1.0];
        engine.spawn_with_triangles(obj, accum.triangles.clone());
        let id = (engine.scene.objects.len() - 1) as u32;
        if let Some(entity) = engine.core.find_entity_by_object_id(id) {
            engine.world.insert(entity, handle);
            if let Some(anim_name) = first_clip.as_deref() {
                engine.world.insert(
                    entity,
                    Animation {
                        clip: anim_name.to_string(),
                        ..Default::default()
                    },
                );
            }
            if let Some(key) = morph_key {
                let count = accum
                    .morph_targets
                    .as_ref()
                    .map(|set| set.targets.len())
                    .unwrap_or(0);
                engine.world.insert(entity, MorphTargets { morph_key: key });
                engine
                    .world
                    .insert(entity, MorphWeights { weights: vec![0.0; count] });
            }
            if let Some(skin) = gltf.skins().next() {
                let reader = skin.reader(|b| buffers_data.get(b.index()).map(|v| v.as_slice()));
                let inverse_bind_mats: Vec<[[f32; 4]; 4]> = reader
                    .read_inverse_bind_matrices()
                    .map(|iter| iter.map(|m| m.into()).collect())
                    .unwrap_or_default();
                let joints = skin.joints().map(|_| crate::ecs::Entity(0)).collect();
                engine
                    .world
                    .insert(entity, Skin { inverse_bind_mats, joints });
            }
        }

        Ok(id)
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

fn accumulate_gltf_node(
    accum: &mut MeshAccum,
    node: gltf::Node,
    parent: Mat4,
    buffers_data: &Vec<Vec<u8>>,
    material_base: u32,
) -> anyhow::Result<()> {
    let local = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world = parent * local;
    let flip = world.determinant().is_sign_negative();

    if let Some(mesh) = node.mesh() {
        let normal_matrix = world.inverse().transpose();
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| buffers_data.get(b.index()).map(|v| v.as_slice()));

            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .context("positions")?
                .map(|p| {
                    let v = world.transform_point3(Vec3::new(p[0], p[1], p[2]));
                    [v.x, v.y, v.z]
                })
                .collect();
            for p in &positions {
                for i in 0..3 {
                    if p[i] < accum.min[i] {
                        accum.min[i] = p[i];
                    }
                    if p[i] > accum.max[i] {
                        accum.max[i] = p[i];
                    }
                }
            }
            let normals: Vec<[f32; 3]> = if let Some(n) = reader.read_normals() {
                n.map(|v| {
                    let vec = normal_matrix
                        .transform_vector3(Vec3::new(v[0], v[1], v[2]))
                        .normalize();
                    let mut arr = [vec.x, vec.y, vec.z];
                    if flip {
                        arr[0] = -arr[0];
                        arr[1] = -arr[1];
                        arr[2] = -arr[2];
                    }
                    arr
                })
                .collect()
            } else {
                vec![[0.0, 1.0, 0.0]; positions.len()]
            };
            let texcoords: Vec<[f32; 2]> = if let Some(t) = reader.read_tex_coords(0) {
                t.into_f32().collect()
            } else {
                vec![[0.0, 0.0]; positions.len()]
            };
            let tangents: Vec<[f32; 4]> = if let Some(t) = reader.read_tangents() {
                t.map(|v| {
                    let vec = normal_matrix
                        .transform_vector3(Vec3::new(v[0], v[1], v[2]))
                        .normalize();
                    let mut arr = [vec.x, vec.y, vec.z, v[3]];
                    if flip {
                        arr[0] = -arr[0];
                        arr[1] = -arr[1];
                        arr[2] = -arr[2];
                        arr[3] = -arr[3];
                    }
                    arr
                })
                .collect()
            } else {
                vec![[1.0, 0.0, 0.0, 1.0]; positions.len()]
            };
            let joints: Vec<[u16; 4]> = if let Some(j) = reader.read_joints(0) {
                j.into_u16().collect()
            } else {
                vec![[0; 4]; positions.len()]
            };
            let weights: Vec<[f32; 4]> = if let Some(w) = reader.read_weights(0) {
                w.into_f32().collect()
            } else {
                vec![[0.0; 4]; positions.len()]
            };

            let mut vertices = Vec::with_capacity(positions.len());
            for i in 0..positions.len() {
                vertices.push(Vertex {
                    pos: positions[i],
                    nrm: normals[i],
                    tan: tangents[i],
                    uv: texcoords[i],
                    joints: joints[i],
                    weights: weights[i],
                });
            }

            let base = accum.vertices.len() as usize;
            accum.vertices.extend(vertices);

            // Handle morph targets for this primitive
            let target_count = prim.morph_targets().count();
            if target_count > 0 {
                let set = accum
                    .morph_targets
                    .get_or_insert_with(|| MorphTargetSet { targets: Vec::new(), base_vertex_count: 0 });

                // Ensure we have enough targets
                if set.targets.len() < target_count {
                    for i in set.targets.len()..target_count {
                        set.targets.push(MorphTarget {
                            name: format!("morph{i}"),
                            vertex_positions: vec![[0.0; 3]; accum.vertices.len()],
                            vertex_normals: None,
                        });
                    }
                } else {
                    for tgt in &mut set.targets {
                        tgt.vertex_positions.resize(accum.vertices.len(), [0.0; 3]);
                        if let Some(n) = &mut tgt.vertex_normals {
                            n.resize(accum.vertices.len(), [0.0; 3]);
                        }
                    }
                }

                for (i, target) in reader.read_morph_targets().enumerate() {
                    let (positions, normals, _tangents) = target;
                    if let Some(iter) = positions {
                        for (j, p) in iter.enumerate() {
                            let vec = world.transform_vector3(Vec3::new(p[0], p[1], p[2]));
                            set.targets[i].vertex_positions[base + j] = [vec.x, vec.y, vec.z];
                        }
                    }
                    if let Some(iter) = normals {
                        if set.targets[i].vertex_normals.is_none() {
                            set.targets[i].vertex_normals = Some(vec![[0.0; 3]; accum.vertices.len()]);
                        }
                        let normals_vec = set.targets[i].vertex_normals.as_mut().unwrap();
                        for (j, n) in iter.enumerate() {
                            let vec = normal_matrix.transform_vector3(Vec3::new(n[0], n[1], n[2]));
                            let mut arr = [vec.x, vec.y, vec.z];
                            if flip {
                                arr[0] = -arr[0];
                                arr[1] = -arr[1];
                                arr[2] = -arr[2];
                            }
                            normals_vec[base + j] = arr;
                        }
                    }
                }
            }

            let base = base as u32;
            let mut indices: Vec<u32> = if let Some(ind) = reader.read_indices() {
                ind.into_u32().map(|i| i + base).collect()
            } else {
                (0..positions.len() as u32).map(|i| i + base).collect()
            };
            if flip {
                for idx in indices.chunks_mut(3) {
                    idx.swap(1, 2);
                }
            }
            let start = accum.indices.len();
            let count = indices.len();
            accum.indices.extend(indices);
            let mat_idx = prim
                .material()
                .index()
                .map(|i| material_base + i as u32)
                .unwrap_or(0);
            accum.prim_ranges.push(PrimitiveRange {
                start,
                count,
                material_index: mat_idx,
            });
        }
    }

    for child in node.children() {
        accumulate_gltf_node(accum, child, world, buffers_data, material_base)?;
    }

    Ok(())
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
