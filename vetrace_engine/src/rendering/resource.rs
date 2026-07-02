use gl::types::*;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

use crate::scene::object::GpuTriangle;

pub fn compile_shader(src: &str, kind: GLenum) -> GLuint {
    let shader = unsafe { gl::CreateShader(kind) };
    let c_str = CString::new(src.as_bytes()).unwrap();

    unsafe {
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        let mut success = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buffer = vec![0u8; len as usize];
            gl::GetShaderInfoLog(
                shader,
                len,
                ptr::null_mut(),
                buffer.as_mut_ptr() as *mut GLchar,
            );
            let error = String::from_utf8_lossy(&buffer).to_string();
            panic!("Shader compile error: {}", error);
        }
    }

    shader
}

pub fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    let program = unsafe { gl::CreateProgram() };

    if vs != 0 {
        unsafe { gl::AttachShader(program, vs) };
    }
    if fs != 0 {
        unsafe { gl::AttachShader(program, fs) };
    }

    unsafe {
        gl::LinkProgram(program);

        let mut success = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            let mut len = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buffer = vec![0u8; len as usize];
            gl::GetProgramInfoLog(
                program,
                len,
                ptr::null_mut(),
                buffer.as_mut_ptr() as *mut GLchar,
            );
            let error = String::from_utf8_lossy(&buffer).to_string();
            panic!("Program link error: {}", error);
        }
    }

    program
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub v1: Vec3,
    pub v2: Vec3,
    pub v3: Vec3,
    pub n1: Vec3,
    pub n2: Vec3,
    pub n3: Vec3,
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    pub uv3: [f32; 2],
    pub material_index: u32,
}

impl Triangle {
    pub fn into_gpu(&self) -> GpuTriangle {
        let v0 = [self.v1.x as f32, self.v1.y as f32, self.v1.z as f32];
        let v1 = [self.v2.x as f32, self.v2.y as f32, self.v2.z as f32];
        let v2 = [self.v3.x as f32, self.v3.y as f32, self.v3.z as f32];
        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
        let duv1 = [self.uv2[0] - self.uv1[0], self.uv2[1] - self.uv1[1]];
        let duv2 = [self.uv3[0] - self.uv1[0], self.uv3[1] - self.uv1[1]];
        GpuTriangle {
            v0,
            _pad0: 0.0,
            e1,
            _pad1: 0.0,
            e2,
            _pad2: 0.0,
            n0: [self.n1.x as f32, self.n1.y as f32, self.n1.z as f32],
            _pad3: 0.0,
            n1: [self.n2.x as f32, self.n2.y as f32, self.n2.z as f32],
            _pad4: 0.0,
            n2: [self.n3.x as f32, self.n3.y as f32, self.n3.z as f32],
            _pad5: 0.0,
            uv0: self.uv1,
            duv1,
            duv2,
            material_index: self.material_index,
            _pad6: 0,
        }
    }
}


#[derive(Clone, Debug)]
pub struct ObjMaterialDesc {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub opacity: f32,
    pub base_color_texture: Option<std::path::PathBuf>,
}

impl Default for ObjMaterialDesc {
    fn default() -> Self {
        Self {
            name: "default_obj_material".to_string(),
            base_color: [0.65, 0.65, 0.65, 1.0],
            metallic: 0.0,
            roughness: 0.85,
            emissive: [0.0; 3],
            opacity: 1.0,
            base_color_texture: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjSubmesh {
    pub name: String,
    pub material: ObjMaterialDesc,
    pub triangles: Vec<GpuTriangle>,
}

#[derive(Clone, Copy, Debug, Default)]
struct ObjIndex {
    v: isize,
    vt: Option<isize>,
    vn: Option<isize>,
}

fn parse_obj_index(token: &str) -> Option<ObjIndex> {
    let mut it = token.split('/');
    let v = it.next()?.parse::<isize>().ok()?;
    let vt = it
        .next()
        .and_then(|s| if s.is_empty() { None } else { s.parse::<isize>().ok() });
    let vn = it
        .next()
        .and_then(|s| if s.is_empty() { None } else { s.parse::<isize>().ok() });
    Some(ObjIndex { v, vt, vn })
}

fn resolve_obj_index<T: Copy>(items: &[T], idx: isize) -> Option<T> {
    if idx > 0 {
        items.get((idx - 1) as usize).copied()
    } else if idx < 0 {
        let resolved = items.len() as isize + idx;
        if resolved >= 0 { items.get(resolved as usize).copied() } else { None }
    } else {
        None
    }
}

fn parse_map_path(tokens: &[&str]) -> Option<PathBuf> {
    // MTL texture lines may contain options such as `map_Kd -s 1 1 1 file.png`.
    // A full option parser is overkill here; the useful path is normally the last
    // non-empty token. This handles Sponza-style MTL files well.
    tokens.iter().rev().find_map(|t| {
        let t = t.trim();
        if t.is_empty() || t.starts_with('-') { None } else { Some(PathBuf::from(t)) }
    })
}

fn load_mtl_file(path: &Path) -> HashMap<String, ObjMaterialDesc> {
    let mut out = HashMap::new();
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return out,
    };
    let base_dir = path.parent().unwrap_or_else(|| Path::new(""));
    let mut current: Option<ObjMaterialDesc> = None;
    for line in io::BufReader::new(file).lines().flatten() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }
        match parts[0] {
            "newmtl" if parts.len() >= 2 => {
                if let Some(mat) = current.take() {
                    out.insert(mat.name.clone(), mat);
                }
                let name = parts[1..].join(" ");
                current = Some(ObjMaterialDesc { name, ..Default::default() });
            }
            "Kd" if parts.len() >= 4 => {
                if let Some(mat) = current.as_mut() {
                    mat.base_color[0] = parts[1].parse::<f32>().unwrap_or(mat.base_color[0]);
                    mat.base_color[1] = parts[2].parse::<f32>().unwrap_or(mat.base_color[1]);
                    mat.base_color[2] = parts[3].parse::<f32>().unwrap_or(mat.base_color[2]);
                }
            }
            "d" if parts.len() >= 2 => {
                if let Some(mat) = current.as_mut() {
                    mat.opacity = parts[1].parse::<f32>().unwrap_or(mat.opacity).clamp(0.0, 1.0);
                    mat.base_color[3] = mat.opacity;
                }
            }
            "Tr" if parts.len() >= 2 => {
                if let Some(mat) = current.as_mut() {
                    let tr = parts[1].parse::<f32>().unwrap_or(0.0).clamp(0.0, 1.0);
                    mat.opacity = 1.0 - tr;
                    mat.base_color[3] = mat.opacity;
                }
            }
            "Ke" if parts.len() >= 4 => {
                if let Some(mat) = current.as_mut() {
                    mat.emissive[0] = parts[1].parse::<f32>().unwrap_or(0.0);
                    mat.emissive[1] = parts[2].parse::<f32>().unwrap_or(0.0);
                    mat.emissive[2] = parts[3].parse::<f32>().unwrap_or(0.0);
                }
            }
            "Ns" if parts.len() >= 2 => {
                if let Some(mat) = current.as_mut() {
                    let ns = parts[1].parse::<f32>().unwrap_or(16.0).max(1.0);
                    // Convert Phong shininess to an approximate PBR roughness.
                    mat.roughness = (2.0 / (ns + 2.0)).sqrt().clamp(0.08, 1.0);
                }
            }
            "Pm" if parts.len() >= 2 => {
                if let Some(mat) = current.as_mut() {
                    mat.metallic = parts[1].parse::<f32>().unwrap_or(0.0).clamp(0.0, 1.0);
                }
            }
            "Pr" if parts.len() >= 2 => {
                if let Some(mat) = current.as_mut() {
                    mat.roughness = parts[1].parse::<f32>().unwrap_or(mat.roughness).clamp(0.0, 1.0);
                }
            }
            "map_Kd" | "map_kd" if parts.len() >= 2 => {
                if let Some(mat) = current.as_mut() {
                    if let Some(rel) = parse_map_path(&parts[1..]) {
                        let tex = if rel.is_absolute() { rel } else { base_dir.join(rel) };
                        mat.base_color_texture = Some(tex);
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(mat) = current.take() {
        out.insert(mat.name.clone(), mat);
    }
    out
}

pub fn load_obj_file_with_materials<P: AsRef<Path>>(path: P) -> Result<Vec<ObjSubmesh>, String> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = io::BufReader::new(file);
    let base_dir = path.parent().unwrap_or_else(|| Path::new(""));

    let mut vertices: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut materials: HashMap<String, ObjMaterialDesc> = HashMap::new();
    let mut submeshes: HashMap<String, ObjSubmesh> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut current_material = "default_obj_material".to_string();

    let ensure_submesh = |name: &str, materials: &HashMap<String, ObjMaterialDesc>, submeshes: &mut HashMap<String, ObjSubmesh>, order: &mut Vec<String>| {
        if !submeshes.contains_key(name) {
            let material = materials.get(name).cloned().unwrap_or_else(|| ObjMaterialDesc { name: name.to_string(), ..Default::default() });
            submeshes.insert(name.to_string(), ObjSubmesh {
                name: name.to_string(),
                material,
                triangles: Vec::new(),
            });
            order.push(name.to_string());
        }
    };

    ensure_submesh(&current_material, &materials, &mut submeshes, &mut order);

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "mtllib" if parts.len() >= 2 => {
                for name in &parts[1..] {
                    let mtl_path = base_dir.join(name);
                    materials.extend(load_mtl_file(&mtl_path));
                }
                // Refresh default submesh material in case the first usemtl appears after mtllib.
                if let Some(sm) = submeshes.get_mut(&current_material) {
                    if let Some(mat) = materials.get(&current_material).cloned() {
                        sm.material = mat;
                    }
                }
            }
            "usemtl" if parts.len() >= 2 => {
                current_material = parts[1..].join(" ");
                ensure_submesh(&current_material, &materials, &mut submeshes, &mut order);
            }
            "v" if parts.len() >= 4 => {
                if let (Ok(x), Ok(y), Ok(z)) = (parts[1].parse::<f64>(), parts[2].parse::<f64>(), parts[3].parse::<f64>()) {
                    vertices.push(Vec3 { x, y, z });
                }
            }
            "vt" if parts.len() >= 3 => {
                let u = parts[1].parse::<f32>().unwrap_or(0.0);
                let v = parts[2].parse::<f32>().unwrap_or(0.0);
                uvs.push([u, 1.0 - v]);
            }
            "vn" if parts.len() >= 4 => {
                if let (Ok(x), Ok(y), Ok(z)) = (parts[1].parse::<f64>(), parts[2].parse::<f64>(), parts[3].parse::<f64>()) {
                    normals.push(Vec3 { x, y, z });
                }
            }
            "f" if parts.len() >= 4 => {
                let face: Vec<ObjIndex> = parts[1..].iter().filter_map(|p| parse_obj_index(p)).collect();
                if face.len() < 3 { continue; }
                for i in 2..face.len() {
                    let idx = [face[0], face[i - 1], face[i]];
                    let mut p = [Vec3::default(); 3];
                    let mut n = [Vec3::default(); 3];
                    let mut uv = [[0.0_f32, 0.0_f32]; 3];
                    let mut valid = true;
                    for j in 0..3 {
                        p[j] = match resolve_obj_index(&vertices, idx[j].v) {
                            Some(v) => v,
                            None => { valid = false; Vec3::default() }
                        };
                        if let Some(vt) = idx[j].vt.and_then(|id| resolve_obj_index(&uvs, id)) {
                            uv[j] = vt;
                        }
                        if let Some(vn) = idx[j].vn.and_then(|id| resolve_obj_index(&normals, id)) {
                            n[j] = vn;
                        }
                    }
                    if !valid { continue; }
                    let flat = compute_flat_normal(&p[0], &p[1], &p[2]);
                    for j in 0..3 {
                        if n[j] == Vec3::default() { n[j] = flat; }
                    }
                    let tri = Triangle {
                        v1: p[0], v2: p[1], v3: p[2],
                        n1: n[0], n2: n[1], n3: n[2],
                        uv1: uv[0], uv2: uv[1], uv3: uv[2],
                        material_index: u32::MAX,
                    }.into_gpu();
                    ensure_submesh(&current_material, &materials, &mut submeshes, &mut order);
                    if let Some(sm) = submeshes.get_mut(&current_material) {
                        sm.triangles.push(tri);
                    }
                }
            }
            _ => {}
        }
    }

    let mut out = Vec::new();
    for key in order {
        if let Some(sm) = submeshes.remove(&key) {
            if !sm.triangles.is_empty() {
                out.push(sm);
            }
        }
    }
    if out.is_empty() {
        // Keep compatibility with OBJ files that have no material commands.
        let tris = load_obj_file(path)?;
        if !tris.is_empty() {
            out.push(ObjSubmesh {
                name: "default_obj_material".to_string(),
                material: ObjMaterialDesc::default(),
                triangles: tris,
            });
        }
    }
    Ok(out)
}

pub fn load_obj_file<P: AsRef<Path>>(path: P) -> Result<Vec<GpuTriangle>, String> {
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = io::BufReader::new(file);

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut triangles = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "v" if parts.len() >= 4 => {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f64>(),
                    parts[2].parse::<f64>(),
                    parts[3].parse::<f64>(),
                ) {
                    vertices.push(Vec3 { x, y, z });
                }
            }
            "vn" if parts.len() >= 4 => {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f64>(),
                    parts[2].parse::<f64>(),
                    parts[3].parse::<f64>(),
                ) {
                    normals.push(Vec3 { x, y, z });
                }
            }
            "f" if parts.len() >= 4 => {
                let mut face_verts = Vec::new();
                let mut face_norms = Vec::new();

                for part in &parts[1..] {
                    let indices: Vec<&str> = part.split('/').collect();

                    if let Ok(vi) = indices[0].parse::<usize>() {
                        if vi > 0 && vi <= vertices.len() {
                            face_verts.push(vertices[vi - 1]);
                        }
                    }

                    let normal = if indices.len() >= 3 && !indices[2].is_empty() {
                        if let Ok(ni) = indices[2].parse::<usize>() {
                            if ni > 0 && ni <= normals.len() {
                                normals[ni - 1]
                            } else {
                                Vec3::default()
                            }
                        } else {
                            Vec3::default()
                        }
                    } else {
                        Vec3::default()
                    };

                    face_norms.push(normal);
                }

                if let Ok(tris) = generate_triangles_from_face(&face_verts, &face_norms) {
                    triangles.extend(tris);
                }
            }
            _ => {}
        }
    }

    Ok(triangles.into_iter().map(|t| t.into_gpu()).collect())
}

fn generate_triangles_from_face(verts: &[Vec3], norms: &[Vec3]) -> Result<Vec<Triangle>, String> {
    if verts.len() < 3 || verts.len() != norms.len() {
        return Err("Invalid face format".to_string());
    }

    let mut tris = Vec::new();
    for i in 2..verts.len() {
        let normal = compute_flat_normal(&verts[0], &verts[i - 1], &verts[i]);
        tris.push(Triangle {
            v1: verts[0],
            v2: verts[i - 1],
            v3: verts[i],
            n1: if norms[0] == Vec3::default() {
                normal
            } else {
                norms[0]
            },
            n2: if norms[i - 1] == Vec3::default() {
                normal
            } else {
                norms[i - 1]
            },
            n3: if norms[i] == Vec3::default() {
                normal
            } else {
                norms[i]
            },
            uv1: [0.0, 0.0],
            uv2: [0.0, 0.0],
            uv3: [0.0, 0.0],
            material_index: u32::MAX,
        });
    }
    Ok(tris)
}

fn compute_flat_normal(v1: &Vec3, v2: &Vec3, v3: &Vec3) -> Vec3 {
    let u = Vec3 {
        x: v2.x - v1.x,
        y: v2.y - v1.y,
        z: v2.z - v1.z,
    };
    let v = Vec3 {
        x: v3.x - v1.x,
        y: v3.y - v1.y,
        z: v3.z - v1.z,
    };
    let nx = u.y * v.z - u.z * v.y;
    let ny = u.z * v.x - u.x * v.z;
    let nz = u.x * v.y - u.y * v.x;
    let len = (nx * nx + ny * ny + nz * nz).sqrt();

    if len > 0.0 {
        Vec3 {
            x: nx / len,
            y: ny / len,
            z: nz / len,
        }
    } else {
        Vec3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    }
}

static SPHERE_CACHE: Lazy<Mutex<HashMap<u32, Vec<GpuTriangle>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn midpoint_index(
    cache: &mut HashMap<(usize, usize), usize>,
    verts: &mut Vec<Vec3>,
    a: usize,
    b: usize,
) -> usize {
    let key = if a < b { (a, b) } else { (b, a) };
    if let Some(&idx) = cache.get(&key) {
        return idx;
    }
    let v1 = verts[a];
    let v2 = verts[b];
    let mut v = Vec3 {
        x: (v1.x + v2.x) * 0.5,
        y: (v1.y + v2.y) * 0.5,
        z: (v1.z + v2.z) * 0.5,
    };
    let len = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    v.x /= len;
    v.y /= len;
    v.z /= len;
    let idx = verts.len();
    verts.push(v);
    cache.insert(key, idx);
    idx
}

fn generate_unit_icosphere(subdiv: u32) -> Vec<GpuTriangle> {
    let t = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let mut verts = vec![
        Vec3 {
            x: -1.0,
            y: t,
            z: 0.0,
        },
        Vec3 {
            x: 1.0,
            y: t,
            z: 0.0,
        },
        Vec3 {
            x: -1.0,
            y: -t,
            z: 0.0,
        },
        Vec3 {
            x: 1.0,
            y: -t,
            z: 0.0,
        },
        Vec3 {
            x: 0.0,
            y: -1.0,
            z: t,
        },
        Vec3 {
            x: 0.0,
            y: 1.0,
            z: t,
        },
        Vec3 {
            x: 0.0,
            y: -1.0,
            z: -t,
        },
        Vec3 {
            x: 0.0,
            y: 1.0,
            z: -t,
        },
        Vec3 {
            x: t,
            y: 0.0,
            z: -1.0,
        },
        Vec3 {
            x: t,
            y: 0.0,
            z: 1.0,
        },
        Vec3 {
            x: -t,
            y: 0.0,
            z: -1.0,
        },
        Vec3 {
            x: -t,
            y: 0.0,
            z: 1.0,
        },
    ];
    for v in &mut verts {
        let len = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
        v.x /= len;
        v.y /= len;
        v.z /= len;
    }

    let mut faces: Vec<(usize, usize, usize)> = vec![
        (0, 11, 5),
        (0, 5, 1),
        (0, 1, 7),
        (0, 7, 10),
        (0, 10, 11),
        (1, 5, 9),
        (5, 11, 4),
        (11, 10, 2),
        (10, 7, 6),
        (7, 1, 8),
        (3, 9, 4),
        (3, 4, 2),
        (3, 2, 6),
        (3, 6, 8),
        (3, 8, 9),
        (4, 9, 5),
        (2, 4, 11),
        (6, 2, 10),
        (8, 6, 7),
        (9, 8, 1),
    ];

    for _ in 0..subdiv {
        let mut new_faces = Vec::with_capacity(faces.len() * 4);
        let mut cache = HashMap::new();
        for (a, b, c) in faces {
            let ab = midpoint_index(&mut cache, &mut verts, a, b);
            let bc = midpoint_index(&mut cache, &mut verts, b, c);
            let ca = midpoint_index(&mut cache, &mut verts, c, a);
            new_faces.push((a, ab, ca));
            new_faces.push((b, bc, ab));
            new_faces.push((c, ca, bc));
            new_faces.push((ab, bc, ca));
        }
        faces = new_faces;
    }

    faces
        .into_iter()
        .map(|(a, b, c)| {
            let v1 = verts[a];
            let v2 = verts[b];
            let v3 = verts[c];
            let n = compute_flat_normal(&v1, &v2, &v3);
            Triangle {
                v1,
                v2,
                v3,
                n1: n,
                n2: n,
                n3: n,
                uv1: [0.0, 0.0],
                uv2: [0.0, 0.0],
                uv3: [0.0, 0.0],
                material_index: u32::MAX,
            }
            .into_gpu()
        })
        .collect()
}

fn scale_triangles(tris: &[GpuTriangle], radius: f32) -> Vec<GpuTriangle> {
    let mut out = Vec::with_capacity(tris.len());
    for t in tris {
        let mut tri = *t;
        for i in 0..3 {
            tri.v0[i] *= radius;
            tri.e1[i] *= radius;
            tri.e2[i] *= radius;
        }
        out.push(tri);
    }
    out
}

/// Generate a cube mesh centered at the origin.
pub fn generate_cube_triangles(size: [f32; 3]) -> Vec<GpuTriangle> {
    let hx = size[0] as f64 * 0.5;
    let hy = size[1] as f64 * 0.5;
    let hz = size[2] as f64 * 0.5;
    let verts = [
        Vec3 {
            x: -hx,
            y: -hy,
            z: -hz,
        },
        Vec3 {
            x: hx,
            y: -hy,
            z: -hz,
        },
        Vec3 {
            x: hx,
            y: hy,
            z: -hz,
        },
        Vec3 {
            x: -hx,
            y: hy,
            z: -hz,
        },
        Vec3 {
            x: -hx,
            y: -hy,
            z: hz,
        },
        Vec3 {
            x: hx,
            y: -hy,
            z: hz,
        },
        Vec3 {
            x: hx,
            y: hy,
            z: hz,
        },
        Vec3 {
            x: -hx,
            y: hy,
            z: hz,
        },
    ];

    let mut tris = Vec::new();
    macro_rules! face {
        ($a:expr,$b:expr,$c:expr,$d:expr,$n:expr) => {
            tris.push(
                Triangle {
                    v1: verts[$a],
                    v2: verts[$b],
                    v3: verts[$c],
                    n1: $n,
                    n2: $n,
                    n3: $n,
                    uv1: [0.0, 0.0],
                    uv2: [1.0, 0.0],
                    uv3: [1.0, 1.0],
                    material_index: u32::MAX,
                }
                .into_gpu(),
            );
            tris.push(
                Triangle {
                    v1: verts[$a],
                    v2: verts[$c],
                    v3: verts[$d],
                    n1: $n,
                    n2: $n,
                    n3: $n,
                    uv1: [0.0, 0.0],
                    uv2: [1.0, 1.0],
                    uv3: [0.0, 1.0],
                    material_index: u32::MAX,
                }
                .into_gpu(),
            );
        };
    }
    face!(
        0,
        1,
        2,
        3,
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: -1.0
        }
    );
    face!(
        5,
        4,
        7,
        6,
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0
        }
    );
    face!(
        4,
        0,
        3,
        7,
        Vec3 {
            x: -1.0,
            y: 0.0,
            z: 0.0
        }
    );
    face!(
        1,
        5,
        6,
        2,
        Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0
        }
    );
    face!(
        3,
        2,
        6,
        7,
        Vec3 {
            x: 0.0,
            y: 1.0,
            z: 0.0
        }
    );
    face!(
        4,
        5,
        1,
        0,
        Vec3 {
            x: 0.0,
            y: -1.0,
            z: 0.0
        }
    );
    tris
}

/// Generate a UV sphere mesh centered at the origin.
pub fn generate_sphere_triangles(radius: f32, smoothness: u32) -> Vec<GpuTriangle> {
    let mut cache = SPHERE_CACHE.lock().unwrap();
    let unit = cache
        .entry(smoothness)
        .or_insert_with(|| generate_unit_icosphere(smoothness));
    if (radius - 1.0).abs() < f32::EPSILON {
        unit.clone()
    } else {
        scale_triangles(unit, radius)
    }
}