fn boot_log(stage: &str) {
    eprintln!("[VETRACE BOOT] {}", stage);
    let _ = std::io::Write::flush(&mut std::io::stderr());
}

fn render_log(stage: &str) {
    eprintln!("[VETRACE RENDER] {}", stage);
    let _ = std::io::Write::flush(&mut std::io::stderr());
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MaterialUniforms {
    base_color: [f32; 4],
    metallic: f32,
    roughness: f32,
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MorphWeightUniforms {
    weights: [f32; 8], // Support up to 8 morph targets
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurParams {
    resolution: [f32; 2],
    _pad0: [f32; 2],
    region: [f32; 4],
    feather: f32,
    _pad1: [f32; 7],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PrimitiveVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PrimitiveInstance {
    object_index: u32,
    _pad: [u32; 3],
}


#[derive(Clone, Copy, Debug)]
struct SsrQuality {
    max_steps: u32,
    stride: f32,
    thickness: f32,
    roughness_cutoff: f32,
    confidence_threshold: f32,
    temporal_blend: f32,
}

impl SsrQuality {
    fn for_profile(
        profile: crate::rendering::renderer::RendererProfile,
        adaptive_quality: f32,
    ) -> Self {
        let quality = adaptive_quality.clamp(0.5, 1.0);
        let mut ssr = match profile {
            crate::rendering::renderer::RendererProfile::Cinematic => Self {
                max_steps: 48, stride: 0.65, thickness: 0.24, roughness_cutoff: 0.72, confidence_threshold: 0.12, temporal_blend: 0.30,
            },
            crate::rendering::renderer::RendererProfile::Ultra => Self {
                max_steps: 40, stride: 0.75, thickness: 0.27, roughness_cutoff: 0.68, confidence_threshold: 0.14, temporal_blend: 0.28,
            },
            crate::rendering::renderer::RendererProfile::High => Self {
                max_steps: 34, stride: 0.90, thickness: 0.30, roughness_cutoff: 0.62, confidence_threshold: 0.16, temporal_blend: 0.24,
            },
            crate::rendering::renderer::RendererProfile::Balanced => Self {
                max_steps: 28, stride: 1.00, thickness: 0.35, roughness_cutoff: 0.56, confidence_threshold: 0.18, temporal_blend: 0.20,
            },
            crate::rendering::renderer::RendererProfile::Indoor60FPS => Self {
                max_steps: 22, stride: 1.20, thickness: 0.42, roughness_cutoff: 0.50, confidence_threshold: 0.22, temporal_blend: 0.16,
            },
            crate::rendering::renderer::RendererProfile::Low => Self {
                max_steps: 18, stride: 1.45, thickness: 0.48, roughness_cutoff: 0.44, confidence_threshold: 0.26, temporal_blend: 0.12,
            },
        };
        if quality < 0.9 {
            let scale = (quality / 0.9).clamp(0.55, 1.0);
            ssr.max_steps = ((ssr.max_steps as f32 * scale).round() as u32).max(12);
            ssr.stride /= scale;
            ssr.thickness += (1.0 - scale) * 0.12;
            ssr.roughness_cutoff = (ssr.roughness_cutoff - (1.0 - scale) * 0.12).max(0.35);
            ssr.confidence_threshold = (ssr.confidence_threshold + (1.0 - scale) * 0.12).min(0.4);
            ssr.temporal_blend = (ssr.temporal_blend * scale).clamp(0.08, 0.32);
        }
        ssr
    }
}

const MAX_BLUR_REGIONS: usize = 16;

fn primitive_vertices_from_triangles(tris: &[GpuTriangle]) -> (Vec<PrimitiveVertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(tris.len() * 3);
    let mut indices = Vec::with_capacity(tris.len() * 3);
    for tri in tris {
        let v0 = tri.v0;
        let v1 = [
            tri.v0[0] + tri.e1[0],
            tri.v0[1] + tri.e1[1],
            tri.v0[2] + tri.e1[2],
        ];
        let v2 = [
            tri.v0[0] + tri.e2[0],
            tri.v0[1] + tri.e2[1],
            tri.v0[2] + tri.e2[2],
        ];
        let base = vertices.len() as u32;
        vertices.push(PrimitiveVertex {
            position: v0,
            normal: tri.n0,
        });
        vertices.push(PrimitiveVertex {
            position: v1,
            normal: tri.n1,
        });
        vertices.push(PrimitiveVertex {
            position: v2,
            normal: tri.n2,
        });
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    (vertices, indices)
}

fn blue_noise_rgba8_tile() -> Vec<u8> {
    const BLUE_NOISE_TILE_SIZE: u32 = 16;
    let mut rgba = Vec::with_capacity((BLUE_NOISE_TILE_SIZE * BLUE_NOISE_TILE_SIZE * 4) as usize);
    for y in 0..BLUE_NOISE_TILE_SIZE {
        for x in 0..BLUE_NOISE_TILE_SIZE {
            // Small deterministic high-frequency tile used as a fallback/default.
            // Projects that want an authored blue-noise texture can replace this
            // with a locally loaded texture from their asset pipeline.
            let v = ((x * 73 + y * 151 + (x ^ y) * 37 + (x * y * 17)) & 0xff) as u8;
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    }
    rgba
}

fn hash_noise_u8(mut x: u32) -> u8 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x & 0xff) as u8
}

fn create_cloud_noise_texture_3d(
    device: &Device,
    queue: &Queue,
    size: u32,
    seed: u32,
    label: &str,
) -> (Texture, TextureView) {
    let mut rgba = Vec::with_capacity((size * size * size * 4) as usize);
    for z in 0..size {
        for y in 0..size {
            for x in 0..size {
                let h = hash_noise_u8(
                    x.wrapping_mul(73_856_093)
                        ^ y.wrapping_mul(19_349_663)
                        ^ z.wrapping_mul(83_492_791)
                        ^ seed,
                );
                let h2 = hash_noise_u8(u32::from(h) ^ seed.rotate_left(13));
                let h3 = hash_noise_u8(u32::from(h2) ^ x.wrapping_mul(31) ^ z.wrapping_mul(17));
                rgba.extend_from_slice(&[h, h2, h3, 255]);
            }
        }
    }
    let extent = Extent3d {
        width: size,
        height: size,
        depth_or_array_layers: size,
    };
    let texture = device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D3,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        &rgba,
        ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * size),
            rows_per_image: Some(size),
        },
        extent,
    );
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

fn generated_weather_rgba8(size: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 / size as f32;
            let fy = y as f32 / size as f32;
            let n = hash_noise_u8(x.wrapping_mul(928_371) ^ y.wrapping_mul(364_479)) as f32 / 255.0;
            let wave = ((fx * 19.0).sin() * (fy * 17.0).cos() * 0.5 + 0.5) * 0.45 + n * 0.55;
            rgba.extend_from_slice(&[
                (wave * 255.0) as u8,
                ((0.55 + wave * 0.45) * 255.0) as u8,
                ((0.25 + fy * 0.55) * 255.0) as u8,
                ((0.35 + n * 0.65) * 255.0) as u8,
            ]);
        }
    }
    rgba
}

fn atmosphere_lut_bind_group_entries() -> [BindGroupLayoutEntry; 6] {
    [
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::Rgba16Float,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                multisampled: false,
                view_dimension: TextureViewDimension::D2,
                sample_type: TextureSampleType::Float { filterable: true },
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                multisampled: false,
                view_dimension: TextureViewDimension::D2,
                sample_type: TextureSampleType::Float { filterable: true },
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 5,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                multisampled: false,
                view_dimension: TextureViewDimension::D2,
                sample_type: TextureSampleType::Float { filterable: true },
            },
            count: None,
        },
    ]
}

fn create_cloud_temporal_texture(
    device: &Device,
    width: u32,
    height: u32,
    format: TextureFormat,
    label: &str,
) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

fn write_r16float_texture(queue: &Queue, texture: &Texture, width: u32, height: u32, value: u16) {
    let texels = vec![value; (width * height) as usize];
    queue.write_texture(
        ImageCopyTexture { texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
        bytemuck::cast_slice(&texels),
        ImageDataLayout { offset: 0, bytes_per_row: Some(2 * width), rows_per_image: Some(height) },
        Extent3d { width, height, depth_or_array_layers: 1 },
    );
}

fn write_rgba16float_texture(queue: &Queue, texture: &Texture, width: u32, height: u32, value: [u16; 4]) {
    let texels = vec![value; (width * height) as usize];
    queue.write_texture(
        ImageCopyTexture { texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
        bytemuck::cast_slice(&texels),
        ImageDataLayout { offset: 0, bytes_per_row: Some(8 * width), rows_per_image: Some(height) },
        Extent3d { width, height, depth_or_array_layers: 1 },
    );
}

fn initialize_resolve_fallback_textures(
    queue: &Queue,
    width: u32,
    height: u32,
    ambient_occlusion_texture: &Texture,
    ambient_occlusion_history_texture: &Texture,
    ssr_color_texture: &Texture,
    ssr_history_texture: &Texture,
    hybrid_rt_gi_texture: &Texture,
    gi_history_texture: &Texture,
    gi_buffer_texture: &Texture,
) {
    const F16_ZERO: u16 = 0x0000;
    const F16_ONE: u16 = 0x3c00;
    write_r16float_texture(queue, ambient_occlusion_texture, width, height, F16_ONE);
    write_r16float_texture(queue, ambient_occlusion_history_texture, width, height, F16_ONE);
    write_rgba16float_texture(queue, ssr_color_texture, width, height, [F16_ZERO; 4]);
    write_rgba16float_texture(queue, ssr_history_texture, width, height, [F16_ZERO; 4]);
    write_rgba16float_texture(queue, hybrid_rt_gi_texture, width, height, [F16_ZERO; 4]);
    write_rgba16float_texture(queue, gi_history_texture, width, height, [F16_ZERO; 4]);
    write_rgba16float_texture(queue, gi_buffer_texture, width, height, [F16_ZERO; 4]);
}

impl WgpuRenderer {
    fn ambient_occlusion_method_constant(
        method: crate::rendering::renderer::AmbientOcclusionMethod,
    ) -> u32 {
        match method {
            crate::rendering::renderer::AmbientOcclusionMethod::Off => AO_METHOD_OFF,
            crate::rendering::renderer::AmbientOcclusionMethod::SSAO => AO_METHOD_SSAO,
            crate::rendering::renderer::AmbientOcclusionMethod::GTAO => AO_METHOD_GTAO,
            crate::rendering::renderer::AmbientOcclusionMethod::RTAO => AO_METHOD_RTAO,
        }
    }

    fn create_sdfgi_mip_bind_groups(
        device: &Device,
        texture: &Texture,
        layout: &BindGroupLayout,
    ) -> Vec<BindGroup> {
        let mip_count = (GI_SDF_RES as f32).log2().floor() as u32 + 1;
        (1..mip_count)
            .map(|level| {
                let src_view = texture.create_view(&TextureViewDescriptor {
                    label: Some("sdfgi_mip_src_cached"),
                    format: None,
                    dimension: Some(TextureViewDimension::D3),
                    aspect: TextureAspect::All,
                    base_mip_level: level - 1,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
                });
                let dst_view = texture.create_view(&TextureViewDescriptor {
                    label: Some("sdfgi_mip_dst_cached"),
                    format: None,
                    dimension: Some(TextureViewDimension::D3),
                    aspect: TextureAspect::All,
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
                });
                device.create_bind_group(&BindGroupDescriptor {
                    label: Some("sdfgi_mip_bg_cached"),
                    layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&src_view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(&dst_view),
                        },
                    ],
                })
            })
            .collect()
    }

    fn hash_bytes(hasher: &mut DefaultHasher, bytes: &[u8]) {
        bytes.hash(hasher);
    }

    fn bake_settings_hash(params: &RenderParams) -> u64 {
        let mut hasher = DefaultHasher::new();
        params.gi_quality.hash(&mut hasher);
        params.gi_debug_mode.hash(&mut hasher);
        params.gi_mode.hash(&mut hasher);
        hasher.write_u32(params.max_bounces as u32);
        hasher.finish()
    }

    fn update_static_gi_hash(
        &mut self,
        objects: &[GpuObject],
        triangles: &[GpuTriangle],
        bvh: &[GpuBvhNode],
        tri_bvh: &[GpuTriBvhNode],
        materials: &[crate::scene::object::GpuMaterial],
    ) {
        let static_objects: Vec<GpuObject> = objects
            .iter()
            .copied()
            .filter(|object| {
                object.scene_flags & crate::scene::object::SCENE_FLAG_DYNAMIC_GEOMETRY == 0
            })
            .collect();

        let mut scene_hasher = DefaultHasher::new();
        Self::hash_bytes(&mut scene_hasher, bytemuck::cast_slice(&static_objects));
        Self::hash_bytes(&mut scene_hasher, bytemuck::cast_slice(triangles));
        Self::hash_bytes(&mut scene_hasher, bytemuck::cast_slice(bvh));
        Self::hash_bytes(&mut scene_hasher, bytemuck::cast_slice(tri_bvh));
        Self::hash_bytes(&mut scene_hasher, bytemuck::cast_slice(materials));
        let scene_hash = scene_hasher.finish();

        let mut geometry_hasher = DefaultHasher::new();
        Self::hash_bytes(&mut geometry_hasher, bytemuck::cast_slice(&static_objects));
        Self::hash_bytes(&mut geometry_hasher, bytemuck::cast_slice(triangles));
        Self::hash_bytes(&mut geometry_hasher, bytemuck::cast_slice(bvh));
        Self::hash_bytes(&mut geometry_hasher, bytemuck::cast_slice(tri_bvh));
        let geometry_hash = geometry_hasher.finish();

        let mut material_light_hasher = DefaultHasher::new();
        Self::hash_bytes(&mut material_light_hasher, bytemuck::cast_slice(materials));
        let material_light_hash = material_light_hasher.finish();

        if self.gi_cache.static_scene_hash != scene_hash {
            self.gi_cache.scene_hash = scene_hash;
            self.gi_cache.geometry_hash = geometry_hash;
            self.gi_cache.material_light_hash = material_light_hash;
            self.gi_cache.static_scene_hash = scene_hash;
            self.gi_cache.mark_dirty();
        }
    }

    pub fn mark_gi_dirty(&mut self) {
        self.gi_cache.mark_dirty();
    }

    pub fn upload_light_probe_gi_data(
        &mut self,
        probes: &[GpuLightProbeData],
        coefficients: &[GpuLightProbeSh],
    ) -> Result<(), String> {
        if probes.is_empty() {
            self.gi_probe_count = 0;
            self.gi_cache.has_probe_data = false;
            return Ok(());
        }
        if coefficients.len() < probes.len() {
            return Err(format!(
                "light probe GI upload requires at least one SH/irradiance coefficient block per probe: probes={}, coefficients={}",
                probes.len(),
                coefficients.len()
            ));
        }
        self.gi_probe_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("gi_probe_data"),
            contents: bytemuck::cast_slice(probes),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        self.gi_probe_sh_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("gi_probe_sh_coefficients"),
            contents: bytemuck::cast_slice(&coefficients[..probes.len()]),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        self.gi_probe_count = probes.len().min(256) as u32;
        self.gi_cache.has_probe_data = true;
        self.recreate_gi_resolve_bind_group();
        Ok(())
    }

    fn recreate_gi_resolve_bind_group(&mut self) {
        self.gi_resolve_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("gi_resolve_bg"),
            layout: &self.gi_resolve_bind_group_layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&self.depth_view) },
                BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&self.gbuf_albedo_view) },
                BindGroupEntry { binding: 2, resource: BindingResource::TextureView(&self.gbuf_normal_view) },
                BindGroupEntry { binding: 3, resource: BindingResource::TextureView(&self.lightmap_view) },
                BindGroupEntry { binding: 4, resource: BindingResource::TextureView(&self.gi_radiance_view) },
                BindGroupEntry { binding: 5, resource: BindingResource::TextureView(&self.hybrid_rt_gi_view) },
                BindGroupEntry { binding: 6, resource: BindingResource::TextureView(&self.gi_history_view) },
                BindGroupEntry { binding: 7, resource: BindingResource::TextureView(&self.gi_buffer_view) },
                BindGroupEntry { binding: 8, resource: self.gi_resolve_params_buffer.as_entire_binding() },
                BindGroupEntry { binding: 9, resource: self.gi_probe_buffer.as_entire_binding() },
                BindGroupEntry { binding: 10, resource: self.gi_probe_sh_buffer.as_entire_binding() },
                BindGroupEntry { binding: 11, resource: BindingResource::TextureView(&self.gbuf_lightmap_uv_view) },
            ],
        });
    }

    pub fn mark_lightmap_gi_ready(&mut self, has_atlas: bool, has_uvs: bool) {
        self.gi_cache.has_lightmap_atlas = has_atlas;
        self.gi_cache.has_lightmap_uvs = has_uvs;
    }

    pub fn bake_lighting<P: AsRef<std::path::Path>>(
        &mut self,
        artifact_path: P,
        params: &RenderParams,
    ) -> std::io::Result<()> {
        let settings_hash = Self::bake_settings_hash(params);
        let contents = format!(
            "vetrace_lighting_bake_v1\nscene_hash={}\ngeometry_hash={}\nmaterial_light_hash={}\nbake_settings_hash={}\nbackend=scalable_gi\nartifacts=lightmap,probes,optional_sdfgi\ndynamic_policy=sample_probes_or_sdfgi_receive_raster_or_rt_shadows\npath_traced_policy=editor_ground_truth_cinematic_screenshot_bake_validation\n",
            self.gi_cache.scene_hash,
            self.gi_cache.geometry_hash,
            self.gi_cache.material_light_hash,
            settings_hash
        );
        std::fs::write(&artifact_path, contents)?;
        self.gi_cache.last_baked_scene_hash = self.gi_cache.static_scene_hash;
        self.gi_cache.bake_settings_hash = settings_hash;
        self.gi_cache.artifact_path = Some(artifact_path.as_ref().to_path_buf());
        self.gi_cache.probe_metadata = Some("dynamic objects sample baked probes/lightmaps; only dynamic shadows/reflections stay real-time".to_string());
        self.gi_cache.has_lightmap_atlas = true;
        self.gi_cache.has_lightmap_uvs = true;
        self.gi_cache.has_probe_data = true;
        self.gi_cache.dirty = false;
        Ok(())
    }

    pub fn load_lighting_bake<P: AsRef<std::path::Path>>(
        &mut self,
        artifact_path: P,
        params: &RenderParams,
    ) -> std::io::Result<bool> {
        let contents = std::fs::read_to_string(&artifact_path)?;
        let scene_hash = format!("scene_hash={}", self.gi_cache.scene_hash);
        let settings_hash = format!("bake_settings_hash={}", Self::bake_settings_hash(params));
        let matches = contents.contains(&scene_hash) && contents.contains(&settings_hash);
        if matches {
            self.gi_cache.last_baked_scene_hash = self.gi_cache.static_scene_hash;
            self.gi_cache.bake_settings_hash = Self::bake_settings_hash(params);
            self.gi_cache.artifact_path = Some(artifact_path.as_ref().to_path_buf());
            self.gi_cache.has_lightmap_atlas = true;
            self.gi_cache.has_lightmap_uvs = true;
            self.gi_cache.has_probe_data = true;
            self.gi_cache.dirty = false;
        }
        Ok(matches)
    }

    fn texture_array_limit(device: &Device) -> u32 {
        const RESERVED_TEXTURE_SLOTS: u32 = 7;
        const HARD_CAP: u32 = 256;
        device
            .limits()
            .max_sampled_textures_per_shader_stage
            .min(HARD_CAP)
            .saturating_sub(RESERVED_TEXTURE_SLOTS)
            .max(1)
    }

    pub fn new(window: &Window, width: i32, height: i32, is_2d: bool) -> Self {
        boot_log("WgpuRenderer::new: start");
        boot_log("WgpuRenderer::new: before init_wgpu");
        let (device, queue, surface, config) =
            pollster::block_on(init_wgpu(window, width as u32, height as u32));
        boot_log("WgpuRenderer::new: after init_wgpu");
        let device = std::sync::Arc::new(device);
        let queue = std::sync::Arc::new(queue);
        set_wgpu_device_queue(device.clone(), queue.clone());
        let surface_width = width as u32;
        let surface_height = height as u32;
        let render_width = surface_width;
        let render_height = surface_height;
        let (
            st,
            screen_view,
            screen_history_texture,
            screen_history_view,
            dt,
            dv,
            dst,
            dsv,
            nt,
            normal_view,
            ct,
            color_view,
            ga_t,
            gbuf_albedo_view,
            gn_t,
            gbuf_normal_view,
            gm_t,
            gbuf_material_view,
            gi_sdf_texture,
            gi_sdf_view,
            gi_sdf_storage_view,
            gi_radiance_texture,
            gi_radiance_view,
            gi_radiance_storage_view,
            gi_history_texture,
            gi_history_view,
            gi_noisy_texture,
            gi_noisy_view,
            gi_buffer_texture,
            gi_buffer_view,
            motion_texture,
            motion_view,
            variance_texture,
            variance_view,
            lightmap_texture,
            lightmap_view,
            depth_history_texture,
            depth_history_view,
            normal_history_texture,
            normal_history_view,
            occluder_texture,
            occluder_view,
            sampler,
            linear_sampler,
        ) = create_textures(&device, config.format, render_width, render_height);
        boot_log("WgpuRenderer::new: after create_textures");
        const RASTER_SHADOW_MAP_SIZE: u32 = 2048;
        let raster_shadow_texture = device.create_texture(&TextureDescriptor {
            label: Some("raster_shadow_map"),
            size: Extent3d {
                width: RASTER_SHADOW_MAP_SIZE,
                height: RASTER_SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let raster_shadow_view =
            raster_shadow_texture.create_view(&TextureViewDescriptor::default());
        let raster_shadow_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("raster_shadow_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            compare: Some(CompareFunction::LessEqual),
            ..Default::default()
        });
        let raster_shadow_view_proj_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("raster_shadow_view_proj"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (
            transmittance_lut_texture,
            transmittance_lut_view,
            transmittance_lut_storage_view,
            sky_view_lut_texture,
            sky_view_lut_view,
            sky_view_lut_storage_view,
            multi_scattering_lut_texture,
            multi_scattering_lut_view,
            multi_scattering_lut_storage_view,
            aerial_perspective_lut_texture,
            aerial_perspective_lut_view,
            aerial_perspective_lut_storage_view,
        ) = create_atmosphere_lut_textures(&device);
        boot_log("WgpuRenderer::new: after create_atmosphere_lut_textures");
        let blur_src_texture = device.create_texture(&TextureDescriptor {
            label: Some("blur_src"),
            size: Extent3d {
                width: render_width,
                height: render_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: config.format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let blur_src_view = blur_src_texture.create_view(&TextureViewDescriptor::default());
        let (cloud_radiance_texture, cloud_radiance_view) = create_cloud_temporal_texture(
            &device,
            render_width,
            render_height,
            TextureFormat::Rgba16Float,
            "cloud_radiance_current",
        );
        let (cloud_radiance_history_texture, cloud_radiance_history_view) =
            create_cloud_temporal_texture(
                &device,
                render_width,
                render_height,
                TextureFormat::Rgba16Float,
                "cloud_radiance_history",
            );
        let (cloud_transmittance_texture, cloud_transmittance_view) = create_cloud_temporal_texture(
            &device,
            render_width,
            render_height,
            TextureFormat::R16Float,
            "cloud_transmittance_current",
        );
        let (cloud_transmittance_history_texture, cloud_transmittance_history_view) =
            create_cloud_temporal_texture(
                &device,
                render_width,
                render_height,
                TextureFormat::R16Float,
                "cloud_transmittance_history",
            );
        let (cloud_shadow_texture, cloud_shadow_view) = create_cloud_temporal_texture(
            &device,
            512,
            512,
            TextureFormat::R16Float,
            "cloud_directional_shadow_optical_depth",
        );
        let (cloud_shadow_history_texture, cloud_shadow_history_view) =
            create_cloud_temporal_texture(
                &device,
                512,
                512,
                TextureFormat::R16Float,
                "cloud_directional_shadow_history",
            );

        // Create placeholder buffers large enough to satisfy the minimum
        // binding size required by the shaders. Even with an empty scene the
        // renderer expects space for at least 64 objects and materials.
        const MIN_SCENE_CAPACITY: u64 = 64;
        let object_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("objects"),
            size: std::mem::size_of::<GpuObject>() as u64 * MIN_SCENE_CAPACITY,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cloud_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("volumetric_clouds"),
            size: std::mem::size_of::<crate::scene::object::GpuVolumetricCloud>() as u64
                * crate::scene::object::MAX_VOLUMETRIC_CLOUDS as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let material_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("materials"),
            size: std::mem::size_of::<crate::scene::object::GpuMaterial>() as u64
                * MIN_SCENE_CAPACITY,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let default_custom = crate::scene::object::GpuCustomMaterial::default();
        let custom_material_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("custom_materials"),
            contents: bytemuck::bytes_of(&default_custom),
            usage: BufferUsages::STORAGE,
        });
        let light_header_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("light_header"),
            size: std::mem::size_of::<LightListHeader>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let light_index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("light_indices"),
            size: std::mem::size_of::<u32>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("params"),
            size: std::mem::size_of::<ShaderParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let blit_params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("blit_params"),
            size: std::mem::size_of::<BlitParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let gi_params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gi_params"),
            size: std::mem::size_of::<GiParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let gi_resolve_params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gi_resolve_params"),
            size: std::mem::size_of::<GiResolveParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let gi_probe_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("gi_probe_data"),
            contents: &[0u8; 32],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let gi_probe_sh_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("gi_probe_sh_coefficients"),
            contents: &[0u8; 128],
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let postfx_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("postfx"),
            size: std::mem::size_of::<PostFxUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let light_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("light"),
            size: std::mem::size_of::<LightUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let white_texture = crate::gpu::TextureHandle(std::sync::Arc::new(
            crate::gpu::GpuTexture::from_rgba8(
                device.as_ref(),
                queue.as_ref(),
                &[255, 255, 255, 255],
                1,
                1,
                true,
                "default_white",
            )
            .expect("white texture"),
        ));
        let blue_noise_rgba = blue_noise_rgba8_tile();
        let blue_noise_texture = crate::gpu::TextureHandle(std::sync::Arc::new(
            crate::gpu::GpuTexture::from_rgba8(
                device.as_ref(),
                queue.as_ref(),
                &blue_noise_rgba,
                16,
                16,
                false,
                "blue_noise_texture",
            )
            .expect("blue noise texture"),
        ));

        let (cloud_shape_noise_texture, cloud_shape_noise_view) = create_cloud_noise_texture_3d(
            device.as_ref(),
            queue.as_ref(),
            32,
            0x1234_abcd,
            "cloud_shape_noise",
        );
        let (cloud_detail_noise_texture, cloud_detail_noise_view) = create_cloud_noise_texture_3d(
            device.as_ref(),
            queue.as_ref(),
            32,
            0x9675_31ef,
            "cloud_detail_noise",
        );
        let cloud_weather_rgba = generated_weather_rgba8(128);
        let cloud_weather_extent = Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        };
        let cloud_weather_texture = device.create_texture(&TextureDescriptor {
            label: Some("cloud_weather_map"),
            size: cloud_weather_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            ImageCopyTexture {
                texture: &cloud_weather_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &cloud_weather_rgba,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * 128),
                rows_per_image: Some(128),
            },
            cloud_weather_extent,
        );
        let cloud_weather_view =
            cloud_weather_texture.create_view(&TextureViewDescriptor::default());
        let (cloud_radiance_texture, cloud_radiance_view) = create_cloud_temporal_texture(
            &device,
            render_width,
            render_height,
            TextureFormat::Rgba16Float,
            "cloud_radiance_current",
        );
        let (cloud_radiance_history_texture, cloud_radiance_history_view) =
            create_cloud_temporal_texture(
                &device,
                render_width,
                render_height,
                TextureFormat::Rgba16Float,
                "cloud_radiance_history",
            );
        let (cloud_transmittance_texture, cloud_transmittance_view) = create_cloud_temporal_texture(
            &device,
            render_width,
            render_height,
            TextureFormat::R16Float,
            "cloud_transmittance_current",
        );
        let (cloud_transmittance_history_texture, cloud_transmittance_history_view) =
            create_cloud_temporal_texture(
                &device,
                render_width,
                render_height,
                TextureFormat::R16Float,
                "cloud_transmittance_history",
            );
        let (cloud_shadow_texture, cloud_shadow_view) = create_cloud_temporal_texture(
            &device,
            512,
            512,
            TextureFormat::R16Float,
            "cloud_directional_shadow_optical_depth",
        );
        let (cloud_shadow_history_texture, cloud_shadow_history_view) =
            create_cloud_temporal_texture(
                &device,
                512,
                512,
                TextureFormat::R16Float,
                "cloud_directional_shadow_history",
            );

        let create_hybrid_effect_texture = |label: &str| {
            create_cloud_temporal_texture(
                &device,
                render_width,
                render_height,
                TextureFormat::Rgba16Float,
                label,
            )
        };
        let (hybrid_rt_shadow_texture, hybrid_rt_shadow_view) =
            create_hybrid_effect_texture("hybrid_rt_shadow_mask");
        let (hybrid_rt_reflection_texture, hybrid_rt_reflection_view) =
            create_hybrid_effect_texture("hybrid_rt_reflection_radiance");
        let (hybrid_rt_reflection_history_texture, hybrid_rt_reflection_history_view) =
            create_hybrid_effect_texture("hybrid_rt_reflection_history");
        let (ssr_color_texture, ssr_color_view) =
            create_hybrid_effect_texture("ssr_reflection_radiance");
        let (ssr_history_texture, ssr_history_view) =
            create_hybrid_effect_texture("ssr_reflection_history");
        let (hybrid_rt_gi_texture, hybrid_rt_gi_view) =
            create_hybrid_effect_texture("hybrid_rt_gi_radiance");
        let (hybrid_rt_transparency_texture, hybrid_rt_transparency_view) =
            create_hybrid_effect_texture("hybrid_rt_transparency_radiance");
        let (ambient_occlusion_texture, ambient_occlusion_view) = create_cloud_temporal_texture(
            &device,
            render_width,
            render_height,
            TextureFormat::R16Float,
            "ambient_occlusion_current",
        );
        let (ambient_occlusion_history_texture, ambient_occlusion_history_view) =
            create_cloud_temporal_texture(
                &device,
                render_width,
                render_height,
                TextureFormat::R16Float,
                "ambient_occlusion_history",
            );
        let gbuf_lightmap_uv_texture = device.create_texture(&TextureDescriptor {
            label: Some("gbuf_lightmap_uv"),
            size: Extent3d {
                width: render_width,
                height: render_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let gbuf_lightmap_uv_view =
            gbuf_lightmap_uv_texture.create_view(&TextureViewDescriptor::default());
        initialize_resolve_fallback_textures(
            queue.as_ref(),
            render_width,
            render_height,
            &ambient_occlusion_texture,
            &ambient_occlusion_history_texture,
            &ssr_color_texture,
            &ssr_history_texture,
            &hybrid_rt_gi_texture,
            &gi_history_texture,
            &gi_buffer_texture,
        );
        let hybrid_rt_params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("hybrid_rt_params"),
            size: std::mem::size_of::<HybridRtEffectParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let hybrid_composite_params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("hybrid_composite_params"),
            size: std::mem::size_of::<HybridCompositeParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ssr_params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("ssr_params"),
            size: std::mem::size_of::<SsrParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ambient_occlusion_params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("ambient_occlusion_params"),
            size: std::mem::size_of::<AmbientOcclusionParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let triangle_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("triangles"),
            size: std::mem::size_of::<GpuTriangle>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bvh_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("bvh"),
            size: std::mem::size_of::<GpuBvhNode>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let tri_bvh_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("tri_bvh"),
            size: std::mem::size_of::<GpuTriBvhNode>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        boot_log("WgpuRenderer::new: before SDFGI shader modules");
        let sdfgi_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("sdfgi_prepass"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/hybrid/sdfgi_prepass.comp.wgsl",).into(),
            ),
        });
        let sdfgi_inject_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("sdfgi_inject"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/hybrid/sdfgi_inject.comp.wgsl",).into(),
            ),
        });
        let sdfgi_mip_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("sdfgi_mips"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/hybrid/sdfgi_mips.comp.wgsl",).into(),
            ),
        });

        boot_log("WgpuRenderer::new: after SDFGI shader modules");
        let texture_array_limit = Self::texture_array_limit(&device);
        let safe_shader_mode = std::env::var("VETRACE_SAFE_SHADER").ok().as_deref() == Some("1");
        boot_log("WgpuRenderer::new: before bootstrap compute shader module");
        if safe_shader_mode {
            boot_log(
                "WgpuRenderer::new: VETRACE_SAFE_SHADER=1, forcing bootstrap diagnostic shader",
            );
        }
        let mut shader_compiler = RaytraceShaderCompiler {
            device: device.clone(),
            base_shader_template: concat!(
                include_str!("../../../assets/shaders/wgpu/hybrid/pbr_lighting.wgsl"),
                "\n",
                include_str!("../../../assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl"),
            )
            .to_string(),
            material_registry: std::collections::HashMap::new(),
        };
        let compute_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("bootstrap_compute_shader"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/hybrid/bootstrap.comp.wgsl").into(),
            ),
        });
        boot_log("WgpuRenderer::new: after bootstrap compute shader module");
        boot_log("WgpuRenderer::new: skipping cinematic pathtrace pipeline at startup");
        boot_log("WgpuRenderer::new: before compute bind group layout");
        let compute_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("compute_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: TextureFormat::R32Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 9,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 10,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Uint,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 11,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 12,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D3,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 13,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 14,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 15,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 16,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D3,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 17,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // GI intermediates
                    BindGroupLayoutEntry {
                        binding: 18,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 19,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 20,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 21,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: std::num::NonZeroU32::new(texture_array_limit),
                    },
                    BindGroupLayoutEntry {
                        binding: 22,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 23,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 24,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 25,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D3,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 26,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 27,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 28,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 29,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 30,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D3,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 31,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D3,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 32,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 33,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 34,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 35,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 36,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 37,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 38,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 39,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 40,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 41,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 42,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::Comparison),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 43,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 44,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 45,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 46,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 47,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                ],
            });

        boot_log("WgpuRenderer::new: after compute bind group layout");
        let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("compute_pl"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });
        boot_log("WgpuRenderer::new: before bootstrap compute_pipeline");
        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("bootstrap_compute_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });
        boot_log(
            "WgpuRenderer::new: after bootstrap compute_pipeline; before bootstrap cloud_shadow_pipeline",
        );
        let cloud_shadow_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("bootstrap_cloud_directional_shadow_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "cloud_shadow_main",
            compilation_options: Default::default(),
        });
        let hybrid_compose_pipeline = if safe_shader_mode {
            None
        } else {
            render_log("compiling lightweight hybrid compose pipeline...");
            let hybrid_started = Instant::now();
            let hybrid_compose_shader = device.create_shader_module(ShaderModuleDescriptor {
                label: Some("hybrid_compose_shader"),
                source: ShaderSource::Wgsl(
                    concat!(
                        include_str!("../../../assets/shaders/wgpu/hybrid/pbr_lighting.wgsl"),
                        "\n",
                        include_str!(
                            "../../../assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl"
                        ),
                    )
                    .into(),
                ),
            });
            let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("hybrid_compose_pipeline"),
                layout: Some(&compute_pipeline_layout),
                module: &hybrid_compose_shader,
                entry_point: "main",
                compilation_options: Default::default(),
            });
            render_log(&format!(
                "lightweight hybrid compose pipeline compiled in {:.3}s",
                hybrid_started.elapsed().as_secs_f64()
            ));
            Some(pipeline)
        };
        let hybrid_compose_pipeline_error = None;

        let hybrid_rt_effect_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("hybrid_rt_effect_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Uint,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Uint,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Uint,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 9,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 10,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 11,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 12,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 13,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry { binding: 14, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                    BindGroupLayoutEntry { binding: 21, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: true } }, count: std::num::NonZeroU32::new(texture_array_limit) },
                    BindGroupLayoutEntry { binding: 22, visibility: ShaderStages::COMPUTE, ty: BindingType::Sampler(SamplerBindingType::Filtering), count: None },
                ],
            });
        let ssr_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ssr_bgl"),
            entries: &[
                BindGroupLayoutEntry { binding: 0, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 1, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 2, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 3, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 4, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 5, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Uint }, count: None },
                BindGroupLayoutEntry { binding: 6, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::WriteOnly, format: TextureFormat::Rgba16Float, view_dimension: TextureViewDimension::D2 }, count: None },
                BindGroupLayoutEntry { binding: 7, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        let rtao_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("rtao_bgl"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Uint,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Uint,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Uint,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::R16Float,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 7,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 8,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 9,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 10,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 11,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 12,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 13,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let hybrid_composite_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("hybrid_composite_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 9,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 10,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 11,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 12,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Uint,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 13,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry { binding: 14, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                    BindGroupLayoutEntry { binding: 43, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                    BindGroupLayoutEntry { binding: 44, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                    BindGroupLayoutEntry { binding: 45, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                    BindGroupLayoutEntry { binding: 46, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                    BindGroupLayoutEntry { binding: 47, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                ],
            });
        let gi_resolve_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("gi_resolve_bgl"),
            entries: &[
                BindGroupLayoutEntry { binding: 0, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 1, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 2, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 3, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 4, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D3, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 5, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 6, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
                BindGroupLayoutEntry { binding: 7, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::WriteOnly, format: TextureFormat::Rgba16Float, view_dimension: TextureViewDimension::D2 }, count: None },
                BindGroupLayoutEntry { binding: 8, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                BindGroupLayoutEntry { binding: 9, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                BindGroupLayoutEntry { binding: 10, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                BindGroupLayoutEntry { binding: 11, visibility: ShaderStages::COMPUTE, ty: BindingType::Texture { multisampled: false, view_dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Float { filterable: false } }, count: None },
            ],
        });
        let ambient_occlusion_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("ambient_occlusion_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let hybrid_rt_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("hybrid_rt_effect_pl"),
            bind_group_layouts: &[&hybrid_rt_effect_bind_group_layout],
            push_constant_ranges: &[],
        });
        let ssr_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("ssr_pl"),
            bind_group_layouts: &[&ssr_bind_group_layout],
            push_constant_ranges: &[],
        });
        let rtao_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("rtao_pl"),
            bind_group_layouts: &[&rtao_bind_group_layout],
            push_constant_ranges: &[],
        });
        let hybrid_composite_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("hybrid_composite_pl"),
                bind_group_layouts: &[&hybrid_composite_bind_group_layout],
                push_constant_ranges: &[],
            });
        let gi_resolve_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("gi_resolve_pl"),
            bind_group_layouts: &[&gi_resolve_bind_group_layout],
            push_constant_ranges: &[],
        });
        let ambient_occlusion_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("ambient_occlusion_pl"),
                bind_group_layouts: &[&ambient_occlusion_bind_group_layout],
                push_constant_ranges: &[],
            });
        let make_hybrid_pipeline =
            |label: &str, source: &str, layout: &PipelineLayout| -> Option<ComputePipeline> {
                if safe_shader_mode {
                    return None;
                }
                let module = device.create_shader_module(ShaderModuleDescriptor {
                    label: Some(label),
                    source: ShaderSource::Wgsl(source.into()),
                });
                Some(device.create_compute_pipeline(&ComputePipelineDescriptor {
                    label: Some(label),
                    layout: Some(layout),
                    module: &module,
                    entry_point: "main",
                    compilation_options: Default::default(),
                }))
            };
        let gi_resolve_pipeline = make_hybrid_pipeline(
            "gi_resolve_pipeline",
            include_str!("../../../assets/shaders/wgpu/hybrid/gi_resolve.comp.wgsl"),
            &gi_resolve_pipeline_layout,
        );
        let hybrid_rt_shadow_pipeline = make_hybrid_pipeline(
            "hybrid_rt_shadows_pipeline",
            include_str!(
                "../../../assets/shaders/wgpu/hybrid/rt_shadows.comp.wgsl"
            ),
            &hybrid_rt_pipeline_layout,
        );
        let ssr_pipeline = make_hybrid_pipeline(
            "ssr_pipeline",
            include_str!("../../../assets/shaders/wgpu/hybrid/ssr.comp.wgsl"),
            &ssr_pipeline_layout,
        );
        let hybrid_rt_reflection_pipeline = make_hybrid_pipeline(
            "hybrid_rt_reflections_pipeline",
            concat!(
                include_str!("../../../assets/shaders/wgpu/hybrid/bvh_traversal.wgsl"),
                "\n",
                include_str!("../../../assets/shaders/wgpu/hybrid/rt_reflections.comp.wgsl")
            ),
            &hybrid_rt_pipeline_layout,
        );
        let hybrid_rt_gi_pipeline = make_hybrid_pipeline(
            "hybrid_rt_gi_pipeline",
            concat!(
                include_str!("../../../assets/shaders/wgpu/hybrid/bvh_traversal.wgsl"),
                "\n",
                include_str!("../../../assets/shaders/wgpu/hybrid/rt_gi.comp.wgsl")
            ),
            &hybrid_rt_pipeline_layout,
        );
        let hybrid_rt_transparency_pipeline = make_hybrid_pipeline("hybrid_rt_transparency_pipeline", include_str!("../../../assets/shaders/wgpu/hybrid/rt_transparency.comp.wgsl"), &hybrid_rt_pipeline_layout);
        let ambient_occlusion_pipeline = make_hybrid_pipeline(
            "ambient_occlusion_pipeline",
            include_str!("../../../assets/shaders/wgpu/hybrid/ambient_occlusion.comp.wgsl"),
            &ambient_occlusion_pipeline_layout,
        );
        let rtao_pipeline = make_hybrid_pipeline(
            "rtao_pipeline",
            concat!(
                include_str!("../../../assets/shaders/wgpu/hybrid/bvh_traversal.wgsl"),
                "\n",
                include_str!("../../../assets/shaders/wgpu/hybrid/rt_ao.comp.wgsl")
            ),
            &rtao_pipeline_layout,
        );
        let hybrid_composite_pipeline = make_hybrid_pipeline(
            "hybrid_composite_pipeline",
            include_str!(
                "../../../assets/shaders/wgpu/hybrid/hybrid_effects_composite.comp.wgsl"
            ),
            &hybrid_composite_pipeline_layout,
        );

        boot_log(
            "WgpuRenderer::new: after bootstrap cloud_shadow_pipeline; before atmosphere shader modules",
        );
        let transmittance_lut_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("transmittance_lut"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/atmosphere/transmittance_lut.comp.wgsl")
                    .into(),
            ),
        });
        let sky_view_lut_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("sky_view_lut"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/atmosphere/sky_view_lut.comp.wgsl")
                    .into(),
            ),
        });
        let multi_scattering_lut_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("multi_scattering_lut"),
            source: ShaderSource::Wgsl(
                include_str!(
                    "../../../assets/shaders/wgpu/atmosphere/multi_scattering_lut.comp.wgsl"
                )
                .into(),
            ),
        });
        let aerial_perspective_lut_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("aerial_perspective_lut"),
            source: ShaderSource::Wgsl(
                include_str!(
                    "../../../assets/shaders/wgpu/atmosphere/aerial_perspective_lut.comp.wgsl"
                )
                .into(),
            ),
        });
        let atmosphere_lut_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("atmosphere_lut_bgl"),
                entries: &atmosphere_lut_bind_group_entries(),
            });
        let aerial_perspective_lut_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("aerial_perspective_lut_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                ],
            });
        let atmosphere_lut_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("atmosphere_lut_pl"),
                bind_group_layouts: &[&atmosphere_lut_bind_group_layout],
                push_constant_ranges: &[],
            });
        let aerial_perspective_lut_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("aerial_perspective_lut_pl"),
                bind_group_layouts: &[&aerial_perspective_lut_bind_group_layout],
                push_constant_ranges: &[],
            });
        boot_log("WgpuRenderer::new: before atmosphere LUT pipelines");
        let transmittance_lut_pipeline =
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("transmittance_lut_pipeline"),
                layout: Some(&atmosphere_lut_pipeline_layout),
                module: &transmittance_lut_shader,
                entry_point: "main",
                compilation_options: Default::default(),
            });
        let sky_view_lut_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("sky_view_lut_pipeline"),
            layout: Some(&atmosphere_lut_pipeline_layout),
            module: &sky_view_lut_shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });
        let multi_scattering_lut_pipeline =
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("multi_scattering_lut_pipeline"),
                layout: Some(&atmosphere_lut_pipeline_layout),
                module: &multi_scattering_lut_shader,
                entry_point: "main",
                compilation_options: Default::default(),
            });
        let aerial_perspective_lut_pipeline =
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("aerial_perspective_lut_pipeline"),
                layout: Some(&aerial_perspective_lut_pipeline_layout),
                module: &aerial_perspective_lut_shader,
                entry_point: "main",
                compilation_options: Default::default(),
            });
        let transmittance_lut_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("transmittance_lut_bg"),
            layout: &atmosphere_lut_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&transmittance_lut_storage_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&sky_view_lut_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&blue_noise_texture.0.view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&blue_noise_texture.0.sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&sky_view_lut_view),
                },
            ],
        });
        let sky_view_lut_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("sky_view_lut_bg"),
            layout: &atmosphere_lut_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&sky_view_lut_storage_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&multi_scattering_lut_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&blue_noise_texture.0.view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&blue_noise_texture.0.sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&transmittance_lut_view),
                },
            ],
        });
        let multi_scattering_lut_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("multi_scattering_lut_bg"),
            layout: &atmosphere_lut_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&multi_scattering_lut_storage_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&sky_view_lut_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&blue_noise_texture.0.view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&blue_noise_texture.0.sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&transmittance_lut_view),
                },
            ],
        });
        let aerial_perspective_lut_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("aerial_perspective_lut_bg"),
            layout: &aerial_perspective_lut_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&aerial_perspective_lut_storage_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&multi_scattering_lut_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&blue_noise_texture.0.view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&blue_noise_texture.0.sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&transmittance_lut_view),
                },
            ],
        });

        let rt_denoise_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("rt_denoise"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/hybrid/rt_denoise.comp.wgsl",).into(),
            ),
        });
        let rt_denoise_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("rt_denoise_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rg16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: TextureFormat::R32Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 9,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 10,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 11,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        boot_log("WgpuRenderer::new: after atmosphere LUT pipelines; before rt denoise");
        let rt_denoise_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("rt_denoise_pl"),
            bind_group_layouts: &[&rt_denoise_bind_group_layout],
            push_constant_ranges: &[],
        });
        let rt_denoise_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("rt_denoise_pipe"),
            layout: Some(&rt_denoise_pipeline_layout),
            module: &rt_denoise_shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        let denoise_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("denoise"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/hybrid/denoise.comp.wgsl",).into(),
            ),
        });
        let denoise_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("denoise_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 10,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 13,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D3,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 14,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 15,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 16,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 17,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 18,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        boot_log("WgpuRenderer::new: after rt denoise; before denoise");
        let denoise_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("denoise_pl"),
            bind_group_layouts: &[&denoise_bind_group_layout],
            push_constant_ranges: &[],
        });
        let denoise_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("denoise_pipeline"),
            layout: Some(&denoise_pipeline_layout),
            module: &denoise_shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        let material_textures = vec![white_texture.clone(); texture_array_limit as usize];
        let tex_views: Vec<&TextureView> = material_textures.iter().map(|t| &t.0.view).collect();
        let compute_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("compute_bg"),
            layout: &compute_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: triangle_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: tri_bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&color_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&dv),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(&normal_view),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(&gbuf_albedo_view),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(&gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: gi_params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::TextureView(&gi_sdf_view),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 14,
                    resource: BindingResource::TextureView(&gi_history_view),
                },
                BindGroupEntry {
                    binding: 15,
                    resource: BindingResource::TextureView(&gi_noisy_view),
                },
                BindGroupEntry {
                    binding: 16,
                    resource: BindingResource::TextureView(&gi_radiance_view),
                },
                BindGroupEntry {
                    binding: 17,
                    resource: material_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 18,
                    resource: BindingResource::TextureView(&lightmap_view),
                },
                BindGroupEntry {
                    binding: 19,
                    resource: light_header_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 20,
                    resource: light_index_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 21,
                    resource: BindingResource::TextureViewArray(&tex_views),
                },
                BindGroupEntry {
                    binding: 22,
                    resource: BindingResource::Sampler(&linear_sampler),
                },
                BindGroupEntry {
                    binding: 23,
                    resource: custom_material_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 24,
                    resource: BindingResource::TextureView(&sky_view_lut_view),
                },
                BindGroupEntry {
                    binding: 25,
                    resource: BindingResource::TextureView(&aerial_perspective_lut_view),
                },
                BindGroupEntry {
                    binding: 26,
                    resource: cloud_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 27,
                    resource: BindingResource::TextureView(&blue_noise_texture.0.view),
                },
                BindGroupEntry {
                    binding: 28,
                    resource: BindingResource::Sampler(&blue_noise_texture.0.sampler),
                },
                BindGroupEntry {
                    binding: 29,
                    resource: BindingResource::TextureView(&transmittance_lut_view),
                },
                BindGroupEntry {
                    binding: 30,
                    resource: BindingResource::TextureView(&cloud_shape_noise_view),
                },
                BindGroupEntry {
                    binding: 31,
                    resource: BindingResource::TextureView(&cloud_detail_noise_view),
                },
                BindGroupEntry {
                    binding: 32,
                    resource: BindingResource::TextureView(&cloud_weather_view),
                },
                BindGroupEntry {
                    binding: 33,
                    resource: BindingResource::TextureView(&cloud_radiance_view),
                },
                BindGroupEntry {
                    binding: 34,
                    resource: BindingResource::TextureView(&cloud_radiance_history_view),
                },
                BindGroupEntry {
                    binding: 35,
                    resource: BindingResource::TextureView(&cloud_transmittance_view),
                },
                BindGroupEntry {
                    binding: 36,
                    resource: BindingResource::TextureView(&cloud_transmittance_history_view),
                },
                BindGroupEntry {
                    binding: 37,
                    resource: BindingResource::TextureView(&cloud_shadow_view),
                },
                BindGroupEntry {
                    binding: 38,
                    resource: BindingResource::TextureView(&cloud_shadow_history_view),
                },
                BindGroupEntry {
                    binding: 39,
                    resource: BindingResource::TextureView(&screen_history_view),
                },
                BindGroupEntry {
                    binding: 40,
                    resource: raster_shadow_view_proj_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 41,
                    resource: BindingResource::TextureView(&raster_shadow_view),
                },
                BindGroupEntry {
                    binding: 42,
                    resource: BindingResource::Sampler(&raster_shadow_sampler),
                },
                BindGroupEntry {
                    binding: 43,
                    resource: BindingResource::TextureView(&gi_buffer_view),
                },
                BindGroupEntry {
                    binding: 44,
                    resource: BindingResource::TextureView(&ambient_occlusion_view),
                },
                BindGroupEntry {
                    binding: 45,
                    resource: BindingResource::TextureView(&ssr_color_view),
                },
                BindGroupEntry {
                    binding: 46,
                    resource: BindingResource::TextureView(&hybrid_rt_reflection_view),
                },
                BindGroupEntry {
                    binding: 47,
                    resource: BindingResource::TextureView(&gbuf_lightmap_uv_view),
                },
            ],
        });

        let rt_denoise_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("rt_denoise_bg"),
            layout: &rt_denoise_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&color_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&screen_history_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&screen_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&dv),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&normal_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&motion_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&variance_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(&depth_history_view),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(&normal_history_view),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: postfx_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: object_buffer.as_entire_binding(),
                },
            ],
        });

        let denoise_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("denoise_bg"),
            layout: &denoise_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&dv),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: BindingResource::TextureView(&gi_sdf_view),
                },
                BindGroupEntry {
                    binding: 14,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 15,
                    resource: BindingResource::TextureView(&gi_history_view),
                },
                BindGroupEntry {
                    binding: 16,
                    resource: BindingResource::TextureView(&gi_noisy_view),
                },
                BindGroupEntry {
                    binding: 17,
                    resource: BindingResource::TextureView(&gi_buffer_view),
                },
                BindGroupEntry {
                    binding: 18,
                    resource: postfx_buffer.as_entire_binding(),
                },
            ],
        });

        let sdfgi_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("sdfgi_bgl"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::R32Float,
                        view_dimension: TextureViewDimension::D3,
                    },
                    count: None,
                },
            ],
        });
        boot_log("WgpuRenderer::new: after denoise; before SDFGI pipelines");
        let sdfgi_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("sdfgi_pl"),
            bind_group_layouts: &[&sdfgi_bind_group_layout],
            push_constant_ranges: &[],
        });
        let sdfgi_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("sdfgi_pipeline"),
            layout: Some(&sdfgi_pipeline_layout),
            module: &sdfgi_shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });
        let sdfgi_inject_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("sdfgi_inject_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D3,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let sdfgi_inject_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("sdfgi_inject_pl"),
                bind_group_layouts: &[&sdfgi_inject_bind_group_layout],
                push_constant_ranges: &[],
            });
        let sdfgi_inject_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("sdfgi_inject_pipeline"),
            layout: Some(&sdfgi_inject_pipeline_layout),
            module: &sdfgi_inject_shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });
        let sdfgi_mip_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("sdfgi_mip_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D3,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R32Float,
                            view_dimension: TextureViewDimension::D3,
                        },
                        count: None,
                    },
                ],
            });
        let sdfgi_mip_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("sdfgi_mip_pl"),
            bind_group_layouts: &[&sdfgi_mip_bind_group_layout],
            push_constant_ranges: &[],
        });
        let sdfgi_mip_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("sdfgi_mip_pipeline"),
            layout: Some(&sdfgi_mip_pipeline_layout),
            module: &sdfgi_mip_shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });
        let sdfgi_mip_bind_groups = Self::create_sdfgi_mip_bind_groups(
            &device,
            &gi_sdf_texture,
            &sdfgi_mip_bind_group_layout,
        );
        let sdfgi_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("sdfgi_bg"),
            layout: &sdfgi_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&gi_sdf_storage_view),
                },
            ],
        });
        let sdfgi_inject_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("sdfgi_inject_bg"),
            layout: &sdfgi_inject_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&gi_sdf_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&gi_radiance_storage_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let render_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("render_shader"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/postprocess/blit.wgsl",).into(),
            ),
        });
        let render_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("render_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 9,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 10,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 11,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 12,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 13,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        boot_log("WgpuRenderer::new: after SDFGI pipelines; before blit/render pipeline");
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("render_pl"),
            bind_group_layouts: &[&render_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &render_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &render_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let render_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("render_bg"),
            layout: &render_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Sampler(&linear_sampler),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&screen_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&normal_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: postfx_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&dv),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&occluder_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: light_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(&screen_history_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&depth_history_view),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureView(&normal_history_view),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::Sampler(&linear_sampler),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: blit_params_buffer.as_entire_binding(),
                },
            ],
        });

        let blur_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("ui_blur"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/ui/blur.wgsl").into(),
            ),
        });
        let blur_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("blur_bgl"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(NonZeroU64::new(64).unwrap()),
                    },
                    count: None,
                },
            ],
        });
        let blur_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("blur_pl"),
            bind_group_layouts: &[&blur_bind_group_layout],
            push_constant_ranges: &[],
        });
        let blur_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("blur_pipeline"),
            layout: Some(&blur_pipeline_layout),
            vertex: VertexState {
                module: &blur_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &blur_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });
        let blur_params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("blur_params"),
            size: (MAX_BLUR_REGIONS as u64) * 256,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let blur_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("blur_bg"),
            layout: &blur_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Sampler(&linear_sampler),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&blur_src_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &blur_params_buffer,
                        offset: 0,
                        size: Some(NonZeroU64::new(64).unwrap()),
                    }),
                },
            ],
        });

        // --- Sprite rendering pipeline ---
        let sprite_vert_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("sprite_vert"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/sprite/sprite_vert.wgsl").into(),
            ),
        });
        let sprite_frag_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("sprite_frag"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/sprite/sprite_frag.wgsl").into(),
            ),
        });
        let occluder_frag_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("occluder_frag"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/sprite/occluder_frag.wgsl").into(),
            ),
        });
        let sprite_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("sprite_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                ],
            });
        let sprite_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("sprite_pl"),
            bind_group_layouts: &[&sprite_bind_group_layout],
            push_constant_ranges: &[],
        });
        let sprite_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&sprite_pipeline_layout),
            vertex: VertexState {
                module: &sprite_vert_shader,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 5]>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x3,
                        },
                        VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: VertexFormat::Float32x2,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &sprite_frag_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
        });
        let occluder_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("occluder_pipeline"),
            layout: Some(&sprite_pipeline_layout),
            vertex: VertexState {
                module: &sprite_vert_shader,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 5]>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x3,
                        },
                        VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: VertexFormat::Float32x2,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &occluder_frag_shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::R8Unorm,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let pbr_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("pbr_shader"),
            source: ShaderSource::Wgsl(include_str!("../../../shaders/simple_pbr.wgsl").into()),
        });
        boot_log("WgpuRenderer::new: after sprite/occluder pipelines; before PBR pipeline");
        let pbr_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("pbr_bgl"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pbr_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pbr_pl"),
            bind_group_layouts: &[&pbr_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pbr_vertex_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<crate::gpu::Vertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: 40,
                    shader_location: 3,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: 48,
                    shader_location: 4,
                    format: VertexFormat::Uint16x4,
                },
                VertexAttribute {
                    offset: 56,
                    shader_location: 5,
                    format: VertexFormat::Float32x4,
                },
            ],
        };
        let pbr_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pbr_pipeline"),
            layout: Some(&pbr_pipeline_layout),
            vertex: VertexState {
                module: &pbr_shader,
                entry_point: "vs_main",
                buffers: &[pbr_vertex_layout.clone()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &pbr_shader,
                entry_point: "fs_main",
                targets: &[
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba8Uint,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::R32Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    }),
                ],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
        });
        let primitive_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("primitive_gbuffer_shader"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/hybrid/primitive_gbuffer.wgsl").into(),
            ),
        });
        let primitive_gbuffer_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("primitive_gbuffer_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let primitive_gbuffer_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("primitive_gbuffer_pl"),
                bind_group_layouts: &[&primitive_gbuffer_bind_group_layout],
                push_constant_ranges: &[],
            });
        let primitive_vertex_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<PrimitiveVertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
            ],
        };
        let primitive_instance_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<PrimitiveInstance>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: &[VertexAttribute {
                offset: 0,
                shader_location: 2,
                format: VertexFormat::Uint32,
            }],
        };
        let primitive_gbuffer_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("primitive_gbuffer_pipeline"),
            layout: Some(&primitive_gbuffer_pipeline_layout),
            vertex: VertexState {
                module: &primitive_shader,
                entry_point: "vs_main",
                buffers: &[
                    primitive_vertex_layout.clone(),
                    primitive_instance_layout.clone(),
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &primitive_shader,
                entry_point: "fs_main",
                targets: &[
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba8Uint,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::R32Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    }),
                ],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let shadow_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("raster_shadow_shader"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/hybrid/raster_shadow.wgsl").into(),
            ),
        });
        let pbr_shadow_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("pbr_shadow_shader"),
            source: ShaderSource::Wgsl(
                include_str!("../../../assets/shaders/wgpu/hybrid/pbr_shadow.wgsl").into(),
            ),
        });
        let primitive_shadow_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("primitive_shadow_pipeline"),
            layout: Some(&primitive_gbuffer_pipeline_layout),
            vertex: VertexState {
                module: &shadow_shader,
                entry_point: "primitive_vs_main",
                buffers: &[
                    primitive_vertex_layout.clone(),
                    primitive_instance_layout.clone(),
                ],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState::default(),
            multiview: None,
        });
        let pbr_shadow_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("pbr_shadow_bgl"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let pbr_shadow_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pbr_shadow_pl"),
            bind_group_layouts: &[&pbr_shadow_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pbr_shadow_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pbr_shadow_pipeline"),
            layout: Some(&pbr_shadow_pipeline_layout),
            vertex: VertexState {
                module: &pbr_shadow_shader,
                entry_point: "pbr_vs_main",
                buffers: &[pbr_vertex_layout],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState::default(),
            multiview: None,
        });
        let (cube_vertices, cube_indices) = primitive_vertices_from_triangles(
            &crate::rendering::resource::generate_cube_triangles([1.0, 1.0, 1.0]),
        );
        let (sphere_vertices, sphere_indices) = primitive_vertices_from_triangles(
            &crate::rendering::resource::generate_sphere_triangles(1.0, 1),
        );
        let primitive_cube_vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("primitive_cube_vertices"),
            contents: bytemuck::cast_slice(&cube_vertices),
            usage: BufferUsages::VERTEX,
        });
        let primitive_cube_index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("primitive_cube_indices"),
            contents: bytemuck::cast_slice(&cube_indices),
            usage: BufferUsages::INDEX,
        });
        let primitive_sphere_vertex_buffer =
            device.create_buffer_init(&util::BufferInitDescriptor {
                label: Some("primitive_sphere_vertices"),
                contents: bytemuck::cast_slice(&sphere_vertices),
                usage: BufferUsages::VERTEX,
            });
        let primitive_sphere_index_buffer =
            device.create_buffer_init(&util::BufferInitDescriptor {
                label: Some("primitive_sphere_indices"),
                contents: bytemuck::cast_slice(&sphere_indices),
                usage: BufferUsages::INDEX,
            });
        let sprite_view_proj_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("sprite_vp"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sprite_vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("sprite_vbo"),
            size: (6 * std::mem::size_of::<[f32; 5]>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let primitive_gbuffer_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("primitive_gbuffer_bg"),
            layout: &primitive_gbuffer_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: sprite_view_proj_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: material_buffer.as_entire_binding(),
                },
            ],
        });
        let make_hybrid_rt_bind_group = |label: &str, out_view: &TextureView| {
            device.create_bind_group(&BindGroupDescriptor {
                label: Some(label),
                layout: &hybrid_rt_effect_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&dv),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&gbuf_normal_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&gbuf_material_view),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&gbuf_albedo_view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(&gbuf_material_view),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::TextureView(&gbuf_material_view),
                    },
                    BindGroupEntry {
                        binding: 6,
                        resource: BindingResource::TextureView(out_view),
                    },
                    BindGroupEntry {
                        binding: 7,
                        resource: hybrid_rt_params_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 8,
                        resource: params_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 9,
                        resource: object_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 10,
                        resource: triangle_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 11,
                        resource: bvh_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 12,
                        resource: tri_bvh_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 13,
                        resource: material_buffer.as_entire_binding(),
                    },
                    BindGroupEntry { binding: 14, resource: BindingResource::TextureView(&ssr_color_view) },
                    BindGroupEntry { binding: 21, resource: BindingResource::TextureViewArray(&tex_views) },
                    BindGroupEntry { binding: 22, resource: BindingResource::Sampler(&linear_sampler) },
                ],
            })
        };
        let rtao_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("rtao_bg"),
            layout: &rtao_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&dv),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&gbuf_albedo_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&ambient_occlusion_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: hybrid_rt_params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: triangle_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: tri_bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: material_buffer.as_entire_binding(),
                },
            ],
        });
        let hybrid_rt_shadow_bind_group =
            make_hybrid_rt_bind_group("hybrid_rt_shadow_bg", &hybrid_rt_shadow_view);
        let hybrid_rt_reflection_bind_group =
            make_hybrid_rt_bind_group("hybrid_rt_reflection_bg", &hybrid_rt_reflection_view);
        let ssr_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("ssr_bg"),
            layout: &ssr_bind_group_layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&dv) },
                BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&gbuf_normal_view) },
                BindGroupEntry { binding: 2, resource: BindingResource::TextureView(&gbuf_albedo_view) },
                BindGroupEntry { binding: 3, resource: BindingResource::TextureView(&color_view) },
                BindGroupEntry { binding: 4, resource: BindingResource::TextureView(&ssr_history_view) },
                BindGroupEntry { binding: 5, resource: BindingResource::TextureView(&gbuf_material_view) },
                BindGroupEntry { binding: 6, resource: BindingResource::TextureView(&ssr_color_view) },
                BindGroupEntry { binding: 7, resource: ssr_params_buffer.as_entire_binding() },
            ],
        });
        let hybrid_rt_gi_bind_group =
            make_hybrid_rt_bind_group("hybrid_rt_gi_bg", &hybrid_rt_gi_view);
        let hybrid_rt_transparency_bind_group =
            make_hybrid_rt_bind_group("hybrid_rt_transparency_bg", &hybrid_rt_transparency_view);

        let gi_resolve_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("gi_resolve_bg"),
            layout: &gi_resolve_bind_group_layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&dv) },
                BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&gbuf_albedo_view) },
                BindGroupEntry { binding: 2, resource: BindingResource::TextureView(&gbuf_normal_view) },
                BindGroupEntry { binding: 3, resource: BindingResource::TextureView(&lightmap_view) },
                BindGroupEntry { binding: 4, resource: BindingResource::TextureView(&gi_radiance_view) },
                BindGroupEntry { binding: 5, resource: BindingResource::TextureView(&hybrid_rt_gi_view) },
                BindGroupEntry { binding: 6, resource: BindingResource::TextureView(&gi_history_view) },
                BindGroupEntry { binding: 7, resource: BindingResource::TextureView(&gi_buffer_view) },
                BindGroupEntry { binding: 8, resource: gi_resolve_params_buffer.as_entire_binding() },
                BindGroupEntry { binding: 9, resource: gi_probe_buffer.as_entire_binding() },
                BindGroupEntry { binding: 10, resource: gi_probe_sh_buffer.as_entire_binding() },
                BindGroupEntry { binding: 11, resource: BindingResource::TextureView(&gbuf_lightmap_uv_view) },
            ],
        });
        let hybrid_composite_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("hybrid_composite_bg"),
            layout: &hybrid_composite_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&color_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&screen_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&gi_buffer_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: hybrid_composite_params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&hybrid_rt_shadow_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&hybrid_rt_reflection_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(&hybrid_rt_gi_view),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(&hybrid_rt_transparency_view),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(&color_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&cloud_radiance_view),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureView(&cloud_transmittance_view),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::TextureView(&gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: BindingResource::TextureView(&ambient_occlusion_view),
                },
                BindGroupEntry { binding: 14, resource: BindingResource::TextureView(&ssr_color_view) },
                BindGroupEntry { binding: 43, resource: BindingResource::TextureView(&gi_buffer_view) },
                BindGroupEntry { binding: 44, resource: BindingResource::TextureView(&ambient_occlusion_view) },
                BindGroupEntry { binding: 45, resource: BindingResource::TextureView(&ssr_color_view) },
                BindGroupEntry { binding: 46, resource: BindingResource::TextureView(&hybrid_rt_reflection_view) },
                BindGroupEntry { binding: 47, resource: BindingResource::TextureView(&gbuf_lightmap_uv_view) },
            ],
        });
        let ambient_occlusion_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("ambient_occlusion_bg"),
            layout: &ambient_occlusion_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&dv),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&ambient_occlusion_history_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&ambient_occlusion_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: ambient_occlusion_params_buffer.as_entire_binding(),
                },
            ],
        });

        boot_log("WgpuRenderer::new: all pipelines created; constructing renderer");
        Self {
            surface,
            device: device.clone(),
            queue: queue.clone(),
            config,
            surface_width,
            surface_height,
            width: render_width,
            height: render_height,
            render_scale: 1.0,
            sharpness: 0.0,
            is_2d: is_2d,
            object_buffer,
            object_buffer_capacity: MIN_SCENE_CAPACITY as usize,
            cloud_buffer,
            triangle_buffer,
            bvh_buffer,
            tri_bvh_buffer,
            material_buffer,
            custom_material_buffer,
            light_header_buffer,
            light_index_buffer,
            triangle_count: 0,
            bvh_node_count: 0,
            tri_bvh_node_count: 0,
            material_count: 0,
            custom_material_count: 0,
            params_buffer,
            blit_params_buffer,
            screen_texture: st,
            screen_view,
            screen_history_texture,
            screen_history_view,
            depth_texture: dt,
            depth_view: dv,
            depth_stencil_texture: dst,
            depth_stencil_view: dsv,
            normal_texture: nt,
            normal_view,
            color_texture: ct,
            color_view,
            gbuf_albedo_texture: ga_t,
            gbuf_albedo_view,
            gbuf_normal_texture: gn_t,
            gbuf_normal_view,
            gbuf_material_texture: gm_t,
            gbuf_material_view,
            gbuf_lightmap_uv_texture,
            gbuf_lightmap_uv_view,
            gi_probe_buffer,
            gi_probe_sh_buffer,
            gi_probe_count: 0,
            gi_sdf_texture,
            gi_sdf_view,
            gi_sdf_storage_view,
            gi_radiance_texture,
            gi_radiance_view,
            gi_radiance_storage_view,
            gi_cache: GiCacheState {
                dirty: true,
                ..Default::default()
            },
            gi_history_texture,
            gi_history_view,
            gi_noisy_texture,
            gi_noisy_view,
            gi_buffer_texture,
            gi_buffer_view,
            motion_texture,
            motion_view,
            variance_texture,
            variance_view,
            lightmap_texture,
            lightmap_view,
            transmittance_lut_texture,
            transmittance_lut_view,
            transmittance_lut_storage_view,
            sky_view_lut_texture,
            sky_view_lut_view,
            sky_view_lut_storage_view,
            multi_scattering_lut_texture,
            multi_scattering_lut_view,
            multi_scattering_lut_storage_view,
            aerial_perspective_lut_texture,
            aerial_perspective_lut_view,
            aerial_perspective_lut_storage_view,
            depth_history_texture,
            depth_history_view,
            normal_history_texture,
            normal_history_view,
            occluder_texture,
            occluder_view,
            blur_src_texture,
            blur_src_view,
            white_texture,
            blue_noise_texture,
            _cloud_shape_noise_texture: cloud_shape_noise_texture,
            cloud_shape_noise_view,
            _cloud_detail_noise_texture: cloud_detail_noise_texture,
            cloud_detail_noise_view,
            _cloud_weather_texture: cloud_weather_texture,
            cloud_weather_view,
            cloud_radiance_texture,
            cloud_radiance_view,
            cloud_radiance_history_texture,
            cloud_radiance_history_view,
            cloud_transmittance_texture,
            cloud_transmittance_view,
            cloud_transmittance_history_texture,
            cloud_transmittance_history_view,
            cloud_shadow_texture,
            cloud_shadow_view,
            cloud_shadow_history_texture,
            cloud_shadow_history_view,
            raster_shadow_texture,
            raster_shadow_view,
            raster_shadow_sampler,
            raster_shadow_view_proj_buffer,
            material_textures,
            gi_params_buffer,
            gi_resolve_params_buffer,
            sdfgi_bind_group_layout,
            sdfgi_bind_group,
            sdfgi_pipeline,
            sdfgi_inject_bind_group_layout,
            sdfgi_inject_bind_group,
            sdfgi_inject_pipeline,
            sdfgi_mip_bind_group_layout,
            sdfgi_mip_bind_groups,
            sdfgi_mip_pipeline,
            gi_resolve_bind_group_layout,
            gi_resolve_bind_group,
            gi_resolve_pipeline,
            atmosphere_lut_bind_group_layout,
            transmittance_lut_bind_group,
            transmittance_lut_pipeline,
            sky_view_lut_bind_group,
            sky_view_lut_pipeline,
            multi_scattering_lut_bind_group,
            multi_scattering_lut_pipeline,
            aerial_perspective_lut_bind_group,
            aerial_perspective_lut_pipeline,
            sampler,
            linear_sampler,
            shader_compiler,
            compute_bind_group_layout,
            compute_bind_group,
            hybrid_rt_shadow_texture,
            hybrid_rt_shadow_view,
            hybrid_rt_reflection_texture,
            hybrid_rt_reflection_view,
            hybrid_rt_reflection_history_texture,
            hybrid_rt_reflection_history_view,
            ssr_color_texture,
            ssr_color_view,
            ssr_history_texture,
            ssr_history_view,
            hybrid_rt_gi_texture,
            hybrid_rt_gi_view,
            hybrid_rt_transparency_texture,
            hybrid_rt_transparency_view,
            ambient_occlusion_texture,
            ambient_occlusion_view,
            ambient_occlusion_history_texture,
            ambient_occlusion_history_view,
            hybrid_rt_params_buffer,
            hybrid_composite_params_buffer,
            ssr_params_buffer,
            ambient_occlusion_params_buffer,
            hybrid_rt_effect_bind_group_layout,
            hybrid_rt_shadow_bind_group,
            hybrid_rt_reflection_bind_group,
            ssr_bind_group_layout,
            ssr_bind_group,
            hybrid_rt_gi_bind_group,
            hybrid_rt_transparency_bind_group,
            hybrid_composite_bind_group_layout,
            hybrid_composite_bind_group,
            ambient_occlusion_bind_group_layout,
            ambient_occlusion_bind_group,
            rtao_bind_group_layout,
            rtao_bind_group,
            hybrid_rt_shadow_pipeline,
            hybrid_rt_reflection_pipeline,
            ssr_pipeline,
            hybrid_rt_gi_pipeline,
            hybrid_rt_transparency_pipeline,
            hybrid_compose_pipeline,
            hybrid_composite_pipeline,
            ambient_occlusion_pipeline,
            rtao_pipeline,
            hybrid_compose_pipeline_error,
            cinematic_compute_pipeline: None,
            cinematic_cloud_shadow_pipeline: None,
            cinematic_pipeline_status: LazyPipelineStatus::NotCompiled,
            cinematic_pipeline_error: None,
            active_compute_pipeline_kind: MainComputePipelineKind::Bootstrap,
            safe_shader_mode,
            pending_material_names: Vec::new(),
            pending_shader_defs: Vec::new(),
            denoise_bind_group_layout,
            denoise_bind_group,
            compute_pipeline,
            cloud_shadow_pipeline,
            denoise_pipeline,
            rt_denoise_bind_group_layout,
            rt_denoise_bind_group,
            rt_denoise_pipeline,
            render_bind_group_layout,
            render_bind_group,
            render_pipeline,
            blur_bind_group_layout,
            blur_bind_group,
            blur_pipeline,
            blur_params_buffer,
            pending_blur_regions: Vec::new(),
            blur_feather: 0.0,
            sprite_bind_group_layout,
            sprite_pipeline,
            occluder_pipeline,
            sprite_vertex_buffer,
            sprite_view_proj_buffer,
            pbr_bind_group_layout,
            pbr_shadow_bind_group_layout,
            pbr_pipeline,
            primitive_gbuffer_bind_group_layout,
            primitive_gbuffer_bind_group,
            primitive_gbuffer_pipeline,
            primitive_shadow_pipeline,
            pbr_shadow_pipeline,
            primitive_cube_vertex_buffer,
            primitive_cube_index_buffer,
            primitive_cube_index_count: cube_indices.len() as u32,
            primitive_sphere_vertex_buffer,
            primitive_sphere_index_buffer,
            primitive_sphere_index_count: sphere_indices.len() as u32,
            prev_raster_primitive_count: None,
            postfx_buffer,
            light_buffer,
            post_fx_uniforms: PostFxUniforms::default(),
            prev_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            prev_taa_jitter: [0.0, 0.0],
            frame_number: 0,
            prev_cam_pos: [0.0; 3],
            prev_cam_front: [0.0; 3],
            prev_cam_up: [0.0; 3],
            prev_cam_right: [0.0; 3],
            prev_num_objects: 0,
            prev_shader_params: None,
            prev_gi_params: None,
            prev_gi_resolve_params: None,
            prev_blit_params: None,
            prev_post_fx_uniforms: None,
            prev_sprite_view_proj: None,
            prev_light_data: None,
            sprite_vertices_cache: Vec::new(),
            prev_material_names: Vec::new(),
            prev_material_fallback_tags: 0,
            prev_shader_defs: Vec::new(),
            prev_objects: Vec::new(),
            prev_triangles: vec![GpuTriangle::zeroed()],
            profiler_stats: crate::rendering::renderer::RendererProfilerStats::default(),
            adaptive_quality: 1.0,
            slow_frame_streak: 0,
            fast_frame_streak: 0,
            prev_bvh_nodes: Vec::new(),
            prev_tri_bvh_nodes: Vec::new(),
            profiler_query_set: if device.features().contains(Features::TIMESTAMP_QUERY) {
                Some(device.create_query_set(&QuerySetDescriptor {
                    label: Some("renderer_profiler_timestamps"),
                    ty: QueryType::Timestamp,
                    count: 12,
                }))
            } else {
                None
            },
            profiler_query_buffer: if device.features().contains(Features::TIMESTAMP_QUERY) {
                Some(device.create_buffer(&BufferDescriptor {
                    label: Some("renderer_profiler_timestamp_resolve"),
                    size: 12 * std::mem::size_of::<u64>() as u64,
                    usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }))
            } else {
                None
            },
            profiler_readback_buffer: if device.features().contains(Features::TIMESTAMP_QUERY) {
                Some(device.create_buffer(&BufferDescriptor {
                    label: Some("renderer_profiler_timestamp_readback"),
                    size: 12 * std::mem::size_of::<u64>() as u64,
                    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }))
            } else {
                None
            },
            profiler_timestamp_period: queue.get_timestamp_period(),
        }
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        self.surface_width = width as u32;
        self.surface_height = height as u32;
        self.config.width = self.surface_width;
        self.config.height = self.surface_height;
        self.surface.configure(&self.device, &self.config);
        self.width = (self.surface_width as f32 * self.render_scale) as u32;
        self.height = (self.surface_height as f32 * self.render_scale) as u32;
        let (
            st,
            screen_view,
            screen_history_texture,
            screen_history_view,
            dt,
            dv,
            dst,
            dsv,
            nt,
            normal_view,
            ct,
            color_view,
            ga_t,
            gbuf_albedo_view,
            gn_t,
            gbuf_normal_view,
            gm_t,
            gbuf_material_view,
            gi_sdf_texture,
            gi_sdf_view,
            gi_sdf_storage_view,
            gi_radiance_texture,
            gi_radiance_view,
            gi_radiance_storage_view,
            gi_history_texture,
            gi_history_view,
            gi_noisy_texture,
            gi_noisy_view,
            gi_buffer_texture,
            gi_buffer_view,
            motion_texture,
            motion_view,
            variance_texture,
            variance_view,
            lightmap_texture,
            lightmap_view,
            depth_history_texture,
            depth_history_view,
            normal_history_texture,
            normal_history_view,
            occluder_texture,
            occluder_view,
            sampler,
            linear_sampler,
        ) = create_textures(&self.device, self.config.format, self.width, self.height);
        let (
            transmittance_lut_texture,
            transmittance_lut_view,
            transmittance_lut_storage_view,
            sky_view_lut_texture,
            sky_view_lut_view,
            sky_view_lut_storage_view,
            multi_scattering_lut_texture,
            multi_scattering_lut_view,
            multi_scattering_lut_storage_view,
            aerial_perspective_lut_texture,
            aerial_perspective_lut_view,
            aerial_perspective_lut_storage_view,
        ) = create_atmosphere_lut_textures(&self.device);
        let blur_src_texture = self.device.create_texture(&TextureDescriptor {
            label: Some("blur_src"),
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.config.format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let blur_src_view = blur_src_texture.create_view(&TextureViewDescriptor::default());
        let (cloud_radiance_texture, cloud_radiance_view) = create_cloud_temporal_texture(
            &self.device,
            self.width,
            self.height,
            TextureFormat::Rgba16Float,
            "cloud_radiance_current",
        );
        let (cloud_radiance_history_texture, cloud_radiance_history_view) =
            create_cloud_temporal_texture(
                &self.device,
                self.width,
                self.height,
                TextureFormat::Rgba16Float,
                "cloud_radiance_history",
            );
        let (cloud_transmittance_texture, cloud_transmittance_view) = create_cloud_temporal_texture(
            &self.device,
            self.width,
            self.height,
            TextureFormat::R16Float,
            "cloud_transmittance_current",
        );
        let (cloud_transmittance_history_texture, cloud_transmittance_history_view) =
            create_cloud_temporal_texture(
                &self.device,
                self.width,
                self.height,
                TextureFormat::R16Float,
                "cloud_transmittance_history",
            );
        let (cloud_shadow_texture, cloud_shadow_view) = create_cloud_temporal_texture(
            &self.device,
            512,
            512,
            TextureFormat::R16Float,
            "cloud_directional_shadow_optical_depth",
        );
        let (cloud_shadow_history_texture, cloud_shadow_history_view) =
            create_cloud_temporal_texture(
                &self.device,
                512,
                512,
                TextureFormat::R16Float,
                "cloud_directional_shadow_history",
            );
        let create_hybrid_effect_texture = |label: &str| {
            create_cloud_temporal_texture(
                &self.device,
                self.width,
                self.height,
                TextureFormat::Rgba16Float,
                label,
            )
        };
        let (hybrid_rt_shadow_texture, hybrid_rt_shadow_view) =
            create_hybrid_effect_texture("hybrid_rt_shadow_mask");
        let (hybrid_rt_reflection_texture, hybrid_rt_reflection_view) =
            create_hybrid_effect_texture("hybrid_rt_reflection_radiance");
        let (hybrid_rt_reflection_history_texture, hybrid_rt_reflection_history_view) =
            create_hybrid_effect_texture("hybrid_rt_reflection_history");
        let (ssr_color_texture, ssr_color_view) =
            create_hybrid_effect_texture("ssr_reflection_radiance");
        let (ssr_history_texture, ssr_history_view) =
            create_hybrid_effect_texture("ssr_reflection_history");
        let (hybrid_rt_gi_texture, hybrid_rt_gi_view) =
            create_hybrid_effect_texture("hybrid_rt_gi_radiance");
        let (hybrid_rt_transparency_texture, hybrid_rt_transparency_view) =
            create_hybrid_effect_texture("hybrid_rt_transparency_radiance");
        let (ambient_occlusion_texture, ambient_occlusion_view) = create_cloud_temporal_texture(
            &self.device,
            self.width,
            self.height,
            TextureFormat::R16Float,
            "ambient_occlusion_current",
        );
        let (ambient_occlusion_history_texture, ambient_occlusion_history_view) =
            create_cloud_temporal_texture(
                &self.device,
                self.width,
                self.height,
                TextureFormat::R16Float,
                "ambient_occlusion_history",
            );
        initialize_resolve_fallback_textures(
            self.queue.as_ref(),
            self.width,
            self.height,
            &ambient_occlusion_texture,
            &ambient_occlusion_history_texture,
            &ssr_color_texture,
            &ssr_history_texture,
            &hybrid_rt_gi_texture,
            &gi_history_texture,
            &gi_buffer_texture,
        );
        self.screen_texture = st;
        self.screen_view = screen_view;
        self.screen_history_texture = screen_history_texture;
        self.screen_history_view = screen_history_view;
        self.depth_texture = dt;
        self.depth_view = dv;
        self.depth_stencil_texture = dst;
        self.depth_stencil_view = dsv;
        self.normal_texture = nt;
        self.normal_view = normal_view;
        self.color_texture = ct;
        self.color_view = color_view;
        self.gbuf_albedo_texture = ga_t;
        self.gbuf_albedo_view = gbuf_albedo_view;
        self.gbuf_normal_texture = gn_t;
        self.gbuf_normal_view = gbuf_normal_view;
        self.gbuf_material_texture = gm_t;
        self.gbuf_material_view = gbuf_material_view;
        self.gbuf_lightmap_uv_texture = self.device.create_texture(&TextureDescriptor {
            label: Some("gbuf_lightmap_uv"),
            size: Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.gbuf_lightmap_uv_view = self.gbuf_lightmap_uv_texture.create_view(&TextureViewDescriptor::default());
        self.gi_sdf_texture = gi_sdf_texture;
        self.gi_sdf_view = gi_sdf_view;
        self.gi_sdf_storage_view = gi_sdf_storage_view;
        self.gi_radiance_texture = gi_radiance_texture;
        self.gi_radiance_view = gi_radiance_view;
        self.gi_radiance_storage_view = gi_radiance_storage_view;
        self.sdfgi_mip_bind_groups = Self::create_sdfgi_mip_bind_groups(
            &self.device,
            &self.gi_sdf_texture,
            &self.sdfgi_mip_bind_group_layout,
        );
        self.gi_cache.mark_dirty();
        self.gi_history_texture = gi_history_texture;
        self.gi_history_view = gi_history_view;
        self.gi_noisy_texture = gi_noisy_texture;
        self.gi_noisy_view = gi_noisy_view;
        self.gi_buffer_texture = gi_buffer_texture;
        self.gi_buffer_view = gi_buffer_view;
        self.motion_texture = motion_texture;
        self.motion_view = motion_view;
        self.variance_texture = variance_texture;
        self.variance_view = variance_view;
        self.lightmap_texture = lightmap_texture;
        self.lightmap_view = lightmap_view;
        self.transmittance_lut_texture = transmittance_lut_texture;
        self.transmittance_lut_view = transmittance_lut_view;
        self.transmittance_lut_storage_view = transmittance_lut_storage_view;
        self.sky_view_lut_texture = sky_view_lut_texture;
        self.sky_view_lut_view = sky_view_lut_view;
        self.sky_view_lut_storage_view = sky_view_lut_storage_view;
        self.multi_scattering_lut_texture = multi_scattering_lut_texture;
        self.multi_scattering_lut_view = multi_scattering_lut_view;
        self.multi_scattering_lut_storage_view = multi_scattering_lut_storage_view;
        self.aerial_perspective_lut_texture = aerial_perspective_lut_texture;
        self.aerial_perspective_lut_view = aerial_perspective_lut_view;
        self.aerial_perspective_lut_storage_view = aerial_perspective_lut_storage_view;
        self.depth_history_texture = depth_history_texture;
        self.depth_history_view = depth_history_view;
        self.normal_history_texture = normal_history_texture;
        self.normal_history_view = normal_history_view;
        self.occluder_texture = occluder_texture;
        self.occluder_view = occluder_view;
        self.cloud_radiance_texture = cloud_radiance_texture;
        self.cloud_radiance_view = cloud_radiance_view;
        self.cloud_radiance_history_texture = cloud_radiance_history_texture;
        self.cloud_radiance_history_view = cloud_radiance_history_view;
        self.cloud_transmittance_texture = cloud_transmittance_texture;
        self.cloud_transmittance_view = cloud_transmittance_view;
        self.cloud_transmittance_history_texture = cloud_transmittance_history_texture;
        self.cloud_transmittance_history_view = cloud_transmittance_history_view;
        self.cloud_shadow_texture = cloud_shadow_texture;
        self.cloud_shadow_view = cloud_shadow_view;
        self.cloud_shadow_history_texture = cloud_shadow_history_texture;
        self.cloud_shadow_history_view = cloud_shadow_history_view;
        self.hybrid_rt_shadow_texture = hybrid_rt_shadow_texture;
        self.hybrid_rt_shadow_view = hybrid_rt_shadow_view;
        self.hybrid_rt_reflection_texture = hybrid_rt_reflection_texture;
        self.hybrid_rt_reflection_view = hybrid_rt_reflection_view;
        self.hybrid_rt_reflection_history_texture = hybrid_rt_reflection_history_texture;
        self.hybrid_rt_reflection_history_view = hybrid_rt_reflection_history_view;
        self.ssr_color_texture = ssr_color_texture;
        self.ssr_color_view = ssr_color_view;
        self.ssr_history_texture = ssr_history_texture;
        self.ssr_history_view = ssr_history_view;
        self.hybrid_rt_gi_texture = hybrid_rt_gi_texture;
        self.hybrid_rt_gi_view = hybrid_rt_gi_view;
        self.hybrid_rt_transparency_texture = hybrid_rt_transparency_texture;
        self.hybrid_rt_transparency_view = hybrid_rt_transparency_view;
        self.ambient_occlusion_texture = ambient_occlusion_texture;
        self.ambient_occlusion_view = ambient_occlusion_view;
        self.ambient_occlusion_history_texture = ambient_occlusion_history_texture;
        self.ambient_occlusion_history_view = ambient_occlusion_history_view;
        self.blur_src_texture = blur_src_texture;
        self.blur_src_view = blur_src_view;
        self.sampler = sampler;
        self.linear_sampler = linear_sampler;
        self.prev_view_proj = Mat4::IDENTITY.to_cols_array_2d();
        self.transmittance_lut_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("transmittance_lut_bg"),
            layout: &self.atmosphere_lut_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.transmittance_lut_storage_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.sky_view_lut_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&self.blue_noise_texture.0.view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&self.blue_noise_texture.0.sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&self.sky_view_lut_view),
                },
            ],
        });
        self.sky_view_lut_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("sky_view_lut_bg"),
            layout: &self.atmosphere_lut_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.sky_view_lut_storage_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.multi_scattering_lut_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&self.blue_noise_texture.0.view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&self.blue_noise_texture.0.sampler),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&self.transmittance_lut_view),
                },
            ],
        });
        self.multi_scattering_lut_bind_group =
            self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("multi_scattering_lut_bg"),
                layout: &self.atmosphere_lut_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(
                            &self.multi_scattering_lut_storage_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&self.sky_view_lut_view),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&self.blue_noise_texture.0.view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::Sampler(&self.blue_noise_texture.0.sampler),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::TextureView(&self.transmittance_lut_view),
                    },
                ],
            });
        self.aerial_perspective_lut_bind_group =
            self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("aerial_perspective_lut_bg"),
                layout: &self
                    .aerial_perspective_lut_pipeline
                    .get_bind_group_layout(0),
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(
                            &self.aerial_perspective_lut_storage_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&self.multi_scattering_lut_view),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&self.blue_noise_texture.0.view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::Sampler(&self.blue_noise_texture.0.sampler),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::TextureView(&self.transmittance_lut_view),
                    },
                ],
            });
        let tex_limit = Self::texture_array_limit(&self.device) as usize;
        let tex_views: Vec<&TextureView> = self
            .material_textures
            .iter()
            .take(tex_limit)
            .map(|t| &t.0.view)
            .collect();
        self.compute_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("compute_bg"),
            layout: &self.compute_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.triangle_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: self.tri_bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: self.params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&self.color_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&self.depth_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(&self.normal_view),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(&self.gbuf_albedo_view),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(&self.gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&self.gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: self.gi_params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::TextureView(&self.gi_sdf_view),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 14,
                    resource: BindingResource::TextureView(&self.gi_history_view),
                },
                BindGroupEntry {
                    binding: 15,
                    resource: BindingResource::TextureView(&self.gi_noisy_view),
                },
                BindGroupEntry {
                    binding: 16,
                    resource: BindingResource::TextureView(&self.gi_radiance_view),
                },
                BindGroupEntry {
                    binding: 17,
                    resource: self.material_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 18,
                    resource: BindingResource::TextureView(&self.lightmap_view),
                },
                BindGroupEntry {
                    binding: 19,
                    resource: self.light_header_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 20,
                    resource: self.light_index_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 21,
                    resource: BindingResource::TextureViewArray(&tex_views),
                },
                BindGroupEntry {
                    binding: 22,
                    resource: BindingResource::Sampler(&self.linear_sampler),
                },
                BindGroupEntry {
                    binding: 23,
                    resource: self.custom_material_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 24,
                    resource: BindingResource::TextureView(&self.sky_view_lut_view),
                },
                BindGroupEntry {
                    binding: 25,
                    resource: BindingResource::TextureView(&self.aerial_perspective_lut_view),
                },
                BindGroupEntry {
                    binding: 26,
                    resource: self.cloud_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 27,
                    resource: BindingResource::TextureView(&self.blue_noise_texture.0.view),
                },
                BindGroupEntry {
                    binding: 28,
                    resource: BindingResource::Sampler(&self.blue_noise_texture.0.sampler),
                },
                BindGroupEntry {
                    binding: 29,
                    resource: BindingResource::TextureView(&self.transmittance_lut_view),
                },
                BindGroupEntry {
                    binding: 30,
                    resource: BindingResource::TextureView(&self.cloud_shape_noise_view),
                },
                BindGroupEntry {
                    binding: 31,
                    resource: BindingResource::TextureView(&self.cloud_detail_noise_view),
                },
                BindGroupEntry {
                    binding: 32,
                    resource: BindingResource::TextureView(&self.cloud_weather_view),
                },
                BindGroupEntry {
                    binding: 33,
                    resource: BindingResource::TextureView(&self.cloud_radiance_view),
                },
                BindGroupEntry {
                    binding: 34,
                    resource: BindingResource::TextureView(&self.cloud_radiance_history_view),
                },
                BindGroupEntry {
                    binding: 35,
                    resource: BindingResource::TextureView(&self.cloud_transmittance_view),
                },
                BindGroupEntry {
                    binding: 36,
                    resource: BindingResource::TextureView(&self.cloud_transmittance_history_view),
                },
                BindGroupEntry {
                    binding: 37,
                    resource: BindingResource::TextureView(&self.cloud_shadow_view),
                },
                BindGroupEntry {
                    binding: 38,
                    resource: BindingResource::TextureView(&self.cloud_shadow_history_view),
                },
                BindGroupEntry {
                    binding: 39,
                    resource: BindingResource::TextureView(&self.screen_history_view),
                },
                BindGroupEntry {
                    binding: 40,
                    resource: self.raster_shadow_view_proj_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 41,
                    resource: BindingResource::TextureView(&self.raster_shadow_view),
                },
                BindGroupEntry {
                    binding: 42,
                    resource: BindingResource::Sampler(&self.raster_shadow_sampler),
                },
                BindGroupEntry {
                    binding: 43,
                    resource: BindingResource::TextureView(&self.gi_buffer_view),
                },
                BindGroupEntry {
                    binding: 44,
                    resource: BindingResource::TextureView(&self.ambient_occlusion_view),
                },
                BindGroupEntry {
                    binding: 45,
                    resource: BindingResource::TextureView(&self.ssr_color_view),
                },
                BindGroupEntry {
                    binding: 46,
                    resource: BindingResource::TextureView(&self.hybrid_rt_reflection_view),
                },
                BindGroupEntry {
                    binding: 47,
                    resource: BindingResource::TextureView(&self.gbuf_lightmap_uv_view),
                },
            ],
        });
        let hybrid_rt_tex_views: Vec<&TextureView> = self.material_textures.iter().map(|t| &t.0.view).collect();
        let make_hybrid_rt_bind_group = |label: &str, out_view: &TextureView| {
            self.device.create_bind_group(&BindGroupDescriptor {
                label: Some(label),
                layout: &self.hybrid_rt_effect_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&self.depth_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&self.gbuf_normal_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&self.gbuf_material_view),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&self.gbuf_albedo_view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(&self.gbuf_material_view),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::TextureView(&self.gbuf_material_view),
                    },
                    BindGroupEntry {
                        binding: 6,
                        resource: BindingResource::TextureView(out_view),
                    },
                    BindGroupEntry {
                        binding: 7,
                        resource: self.hybrid_rt_params_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 8,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 9,
                        resource: self.object_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 10,
                        resource: self.triangle_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 11,
                        resource: self.bvh_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 12,
                        resource: self.tri_bvh_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 13,
                        resource: self.material_buffer.as_entire_binding(),
                    },
                    BindGroupEntry { binding: 14, resource: BindingResource::TextureView(&self.ssr_color_view) },
                    BindGroupEntry { binding: 21, resource: BindingResource::TextureViewArray(&hybrid_rt_tex_views) },
                    BindGroupEntry { binding: 22, resource: BindingResource::Sampler(&self.linear_sampler) },
                ],
            })
        };
        self.hybrid_rt_shadow_bind_group =
            make_hybrid_rt_bind_group("hybrid_rt_shadow_bg", &self.hybrid_rt_shadow_view);
        self.hybrid_rt_reflection_bind_group =
            make_hybrid_rt_bind_group("hybrid_rt_reflection_bg", &self.hybrid_rt_reflection_view);
        self.ssr_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("ssr_bg"),
            layout: &self.ssr_bind_group_layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&self.depth_view) },
                BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&self.gbuf_normal_view) },
                BindGroupEntry { binding: 2, resource: BindingResource::TextureView(&self.gbuf_albedo_view) },
                BindGroupEntry { binding: 3, resource: BindingResource::TextureView(&self.color_view) },
                BindGroupEntry { binding: 4, resource: BindingResource::TextureView(&self.ssr_history_view) },
                BindGroupEntry { binding: 5, resource: BindingResource::TextureView(&self.gbuf_material_view) },
                BindGroupEntry { binding: 6, resource: BindingResource::TextureView(&self.ssr_color_view) },
                BindGroupEntry { binding: 7, resource: self.ssr_params_buffer.as_entire_binding() },
            ],
        });
        self.hybrid_rt_gi_bind_group =
            make_hybrid_rt_bind_group("hybrid_rt_gi_bg", &self.hybrid_rt_gi_view);
        self.hybrid_rt_transparency_bind_group = make_hybrid_rt_bind_group(
            "hybrid_rt_transparency_bg",
            &self.hybrid_rt_transparency_view,
        );
        self.rtao_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("rtao_bg"),
            layout: &self.rtao_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.depth_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&self.gbuf_albedo_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&self.gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&self.gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&self.ambient_occlusion_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: self.hybrid_rt_params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: self.params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: self.object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: self.triangle_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: self.bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: self.tri_bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: self.material_buffer.as_entire_binding(),
                },
            ],
        });
        self.gi_resolve_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("gi_resolve_bg"),
            layout: &self.gi_resolve_bind_group_layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&self.depth_view) },
                BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&self.gbuf_albedo_view) },
                BindGroupEntry { binding: 2, resource: BindingResource::TextureView(&self.gbuf_normal_view) },
                BindGroupEntry { binding: 3, resource: BindingResource::TextureView(&self.lightmap_view) },
                BindGroupEntry { binding: 4, resource: BindingResource::TextureView(&self.gi_radiance_view) },
                BindGroupEntry { binding: 5, resource: BindingResource::TextureView(&self.hybrid_rt_gi_view) },
                BindGroupEntry { binding: 6, resource: BindingResource::TextureView(&self.gi_history_view) },
                BindGroupEntry { binding: 7, resource: BindingResource::TextureView(&self.gi_buffer_view) },
                BindGroupEntry { binding: 8, resource: self.gi_resolve_params_buffer.as_entire_binding() },
                BindGroupEntry { binding: 9, resource: self.gi_probe_buffer.as_entire_binding() },
                BindGroupEntry { binding: 10, resource: self.gi_probe_sh_buffer.as_entire_binding() },
                BindGroupEntry { binding: 11, resource: BindingResource::TextureView(&self.gbuf_lightmap_uv_view) },
            ],
        });
        self.hybrid_composite_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("hybrid_composite_bg"),
            layout: &self.hybrid_composite_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.color_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.screen_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.gi_buffer_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: self.hybrid_composite_params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&self.hybrid_rt_shadow_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&self.hybrid_rt_reflection_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(&self.hybrid_rt_gi_view),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(&self.hybrid_rt_transparency_view),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(&self.color_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&self.cloud_radiance_view),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureView(&self.cloud_transmittance_view),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::TextureView(&self.gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: BindingResource::TextureView(&self.ambient_occlusion_view),
                },
                BindGroupEntry { binding: 14, resource: BindingResource::TextureView(&self.ssr_color_view) },
                BindGroupEntry { binding: 43, resource: BindingResource::TextureView(&self.gi_buffer_view) },
                BindGroupEntry { binding: 44, resource: BindingResource::TextureView(&self.ambient_occlusion_view) },
                BindGroupEntry { binding: 45, resource: BindingResource::TextureView(&self.ssr_color_view) },
                BindGroupEntry { binding: 46, resource: BindingResource::TextureView(&self.hybrid_rt_reflection_view) },
                BindGroupEntry { binding: 47, resource: BindingResource::TextureView(&self.gbuf_lightmap_uv_view) },
            ],
        });
        self.ambient_occlusion_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("ambient_occlusion_bg"),
            layout: &self.ambient_occlusion_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.depth_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.ambient_occlusion_history_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&self.ambient_occlusion_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: self.ambient_occlusion_params_buffer.as_entire_binding(),
                },
            ],
        });

        self.primitive_gbuffer_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("primitive_gbuffer_bg"),
            layout: &self.primitive_gbuffer_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.sprite_view_proj_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.material_buffer.as_entire_binding(),
                },
            ],
        });

        self.rt_denoise_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("rt_denoise_bg"),
            layout: &self.rt_denoise_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.color_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.screen_history_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.screen_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&self.depth_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&self.normal_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&self.motion_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&self.variance_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(&self.depth_history_view),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(&self.normal_history_view),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: self.params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: self.postfx_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: self.object_buffer.as_entire_binding(),
                },
            ],
        });

        self.denoise_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("denoise_bg"),
            layout: &self.denoise_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 4,
                    resource: self.params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&self.depth_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&self.gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: BindingResource::TextureView(&self.gi_sdf_view),
                },
                BindGroupEntry {
                    binding: 14,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 15,
                    resource: BindingResource::TextureView(&self.gi_history_view),
                },
                BindGroupEntry {
                    binding: 16,
                    resource: BindingResource::TextureView(&self.gi_noisy_view),
                },
                BindGroupEntry {
                    binding: 17,
                    resource: BindingResource::TextureView(&self.gi_buffer_view),
                },
                BindGroupEntry {
                    binding: 18,
                    resource: self.postfx_buffer.as_entire_binding(),
                },
            ],
        });

        self.sdfgi_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("sdfgi_bg"),
            layout: &self.sdfgi_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.gi_sdf_storage_view),
                },
            ],
        });
        self.sdfgi_inject_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("sdfgi_inject_bg"),
            layout: &self.sdfgi_inject_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.gi_sdf_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.gi_radiance_storage_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: self.params_buffer.as_entire_binding(),
                },
            ],
        });
        self.render_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("render_bg"),
            layout: &self.render_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Sampler(&self.linear_sampler),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.screen_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.normal_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: self.postfx_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&self.depth_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&self.occluder_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: self.light_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(&self.screen_history_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&self.depth_history_view),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureView(&self.normal_history_view),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::Sampler(&self.linear_sampler),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: self.blit_params_buffer.as_entire_binding(),
                },
            ],
        });
        self.blur_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("blur_bg"),
            layout: &self.blur_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Sampler(&self.linear_sampler),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.blur_src_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &self.blur_params_buffer,
                        offset: 0,
                        size: Some(NonZeroU64::new(64).unwrap()),
                    }),
                },
            ],
        });
    }

    pub fn update_scene_data(
        &mut self,
        objects: &[GpuObject],
        triangles: &[GpuTriangle],
        bvh: &[GpuBvhNode],
        tri_bvh: &[GpuTriBvhNode],
        materials: &[crate::scene::object::GpuMaterial],
        custom_materials: &[crate::scene::object::GpuCustomMaterial],
        material_names: &[String],
        shaders: &[(String, String)],
        textures: &[crate::gpu::TextureHandle],
    ) {
        // Ensure the object buffer always has space for at least 64 entries to
        // satisfy the shader's minimum binding size requirements.
        const MIN_SCENE_CAPACITY: usize = 64;
        let object_capacity = self
            .object_buffer_capacity
            .max(MIN_SCENE_CAPACITY.max(objects.len()));
        let objects_changed = self.prev_objects.len() != objects.len()
            || bytemuck::cast_slice::<GpuObject, u8>(&self.prev_objects)
                != bytemuck::cast_slice::<GpuObject, u8>(objects);
        let object_capacity_changed = object_capacity != self.object_buffer_capacity;
        if objects_changed || object_capacity_changed {
            let mut obj_data = vec![GpuObject::default(); object_capacity];
            obj_data[..objects.len()].copy_from_slice(objects);
            let obj_bytes = bytemuck::cast_slice(&obj_data);
            if object_capacity_changed {
                self.object_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
                    label: Some("objects"),
                    contents: obj_bytes,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                });
                self.object_buffer_capacity = object_capacity;
            } else {
                self.queue.write_buffer(&self.object_buffer, 0, obj_bytes);
            }
            self.prev_objects = objects.to_vec();
        }
        self.update_static_gi_hash(objects, triangles, bvh, tri_bvh, materials);
        let tri_changed = self.prev_triangles.len() != triangles.len()
            || bytemuck::cast_slice::<GpuTriangle, u8>(&self.prev_triangles)
                != bytemuck::cast_slice::<GpuTriangle, u8>(triangles);
        if tri_changed {
            let default_tri;
            let tri_bytes: &[u8];
            if triangles.is_empty() {
                default_tri = GpuTriangle::zeroed();
                tri_bytes = bytemuck::bytes_of(&default_tri);
            } else {
                tri_bytes = bytemuck::cast_slice::<GpuTriangle, u8>(triangles);
            }
            if triangles.len() == self.prev_triangles.len() {
                self.queue.write_buffer(&self.triangle_buffer, 0, tri_bytes);
            } else {
                self.triangle_buffer =
                    self.device.create_buffer_init(&util::BufferInitDescriptor {
                        label: Some("triangles"),
                        contents: tri_bytes,
                        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                    });
            }
            self.prev_triangles = triangles.to_vec();
        }
        self.triangle_count = triangles.len() as u32;
        let bvh_changed = self.prev_bvh_nodes.len() != bvh.len()
            || bytemuck::cast_slice::<GpuBvhNode, u8>(&self.prev_bvh_nodes)
                != bytemuck::cast_slice::<GpuBvhNode, u8>(bvh);
        if bvh_changed {
            let default_bvh;
            let bvh_bytes: &[u8];
            if bvh.is_empty() {
                default_bvh = GpuBvhNode::default();
                bvh_bytes = bytemuck::bytes_of(&default_bvh);
            } else {
                bvh_bytes = bytemuck::cast_slice::<GpuBvhNode, u8>(bvh);
            }
            if bvh.len() == self.prev_bvh_nodes.len() {
                self.queue.write_buffer(&self.bvh_buffer, 0, bvh_bytes);
            } else {
                self.bvh_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
                    label: Some("bvh"),
                    contents: bvh_bytes,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                });
            }
            self.prev_bvh_nodes = bvh.to_vec();
        }
        let tri_bvh_changed = self.prev_tri_bvh_nodes.len() != tri_bvh.len()
            || bytemuck::cast_slice::<GpuTriBvhNode, u8>(&self.prev_tri_bvh_nodes)
                != bytemuck::cast_slice::<GpuTriBvhNode, u8>(tri_bvh);
        if tri_bvh_changed {
            let default_tbvh;
            let tbvh_bytes: &[u8];
            if tri_bvh.is_empty() {
                default_tbvh = GpuTriBvhNode::default();
                tbvh_bytes = bytemuck::bytes_of(&default_tbvh);
            } else {
                tbvh_bytes = bytemuck::cast_slice::<GpuTriBvhNode, u8>(tri_bvh);
            }
            if tri_bvh.len() == self.prev_tri_bvh_nodes.len() {
                self.queue.write_buffer(&self.tri_bvh_buffer, 0, tbvh_bytes);
            } else {
                self.tri_bvh_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
                    label: Some("tri_bvh"),
                    contents: tbvh_bytes,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                });
            }
            self.prev_tri_bvh_nodes = tri_bvh.to_vec();
        }
        let tex_limit = Self::texture_array_limit(&self.device) as u32;
        let mut clamped_mats: Vec<crate::scene::object::GpuMaterial> = materials.to_vec();
        for m in &mut clamped_mats {
            if m.base_color_tex >= tex_limit {
                m.base_color_tex = 0;
            }
        }
        // Grow material buffers to at least the same minimum capacity used for
        // objects so that the bind group requirements are always met.
        self.prev_material_fallback_tags =
            clamped_mats.iter().fold(0, |tags, mat| tags | mat._pad2[0]);
        let mut mat_vec = vec![
            crate::scene::object::GpuMaterial::default();
            MIN_SCENE_CAPACITY.max(clamped_mats.len())
        ];
        mat_vec[..clamped_mats.len()].copy_from_slice(&clamped_mats);
        self.material_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("materials"),
            contents: bytemuck::cast_slice(&mat_vec),
            usage: BufferUsages::STORAGE,
        });

        let mut custom_vec = vec![
            crate::scene::object::GpuCustomMaterial::default();
            MIN_SCENE_CAPACITY.max(custom_materials.len())
        ];
        custom_vec[..custom_materials.len()].copy_from_slice(custom_materials);
        self.custom_material_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("custom_materials"),
            contents: bytemuck::cast_slice(&custom_vec),
            usage: BufferUsages::STORAGE,
        });
        const MAX_IMPORTANT_LIGHTS: usize = 256;
        let camera_pos = Vec3::new(
            self.prev_cam_pos[0],
            self.prev_cam_pos[1],
            self.prev_cam_pos[2],
        );
        let camera_front = Vec3::new(
            self.prev_cam_front[0],
            self.prev_cam_front[1],
            self.prev_cam_front[2],
        );
        let mut lights: Vec<(u32, f32)> = Vec::new();
        for (i, obj) in objects.iter().enumerate() {
            let mi = obj.material_index as usize;
            if mi < materials.len() && materials[mi].emissive_strength > 0.0 {
                let mat = materials[mi];
                let light_pos = Vec3::from_array(obj.position);
                let to_light = light_pos - camera_pos;
                let dist2 = to_light.length_squared().max(1.0);
                let color_intensity = mat
                    .base_color_factor
                    .iter()
                    .take(3)
                    .copied()
                    .fold(0.0_f32, f32::max)
                    * mat.emissive_strength;
                let radius =
                    (obj.radius * obj.scale.iter().copied().fold(0.0_f32, f32::max)).max(0.25);
                let distance_score = 1.0 / dist2;
                let screen_influence = (radius / dist2.sqrt()).clamp(0.0, 1.0);
                let view_relevance = if camera_front.length_squared() > 0.0 {
                    let facing = camera_front.normalize().dot(to_light.normalize_or_zero());
                    (facing * 0.5 + 0.5).clamp(0.1, 1.0)
                } else {
                    1.0
                };
                let visibility_relevance =
                    obj.shadow_importance
                        .max(if obj.casts_raytraced_shadow != 0 {
                            1.0
                        } else {
                            0.35
                        });
                let score = color_intensity
                    * (0.25 + distance_score * 64.0)
                    * (0.25 + screen_influence)
                    * view_relevance
                    * visibility_relevance;
                lights.push((i as u32, score));
            }
        }
        lights.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        lights.truncate(MAX_IMPORTANT_LIGHTS);
        let lights: Vec<u32> = lights.into_iter().map(|(idx, _)| idx).collect();
        let light_header = LightListHeader {
            count: lights.len() as u32,
        };
        self.light_header_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("light_header"),
            contents: bytemuck::bytes_of(&light_header),
            usage: BufferUsages::UNIFORM,
        });
        let default_light: u32 = 0;
        let light_bytes: &[u8] = if lights.is_empty() {
            bytemuck::bytes_of(&default_light)
        } else {
            bytemuck::cast_slice(lights.as_slice())
        };
        self.light_index_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("light_indices"),
            contents: light_bytes,
            usage: BufferUsages::STORAGE,
        });
        self.triangle_count = triangles.len() as u32;
        self.bvh_node_count = bvh.len() as u32;
        self.tri_bvh_node_count = tri_bvh.len() as u32;
        self.material_count = clamped_mats.len() as u32;
        self.custom_material_count = custom_materials.len() as u32;
        self.material_textures = Vec::with_capacity(tex_limit as usize);
        self.material_textures.push(self.white_texture.clone());
        self.material_textures
            .extend(textures.iter().take((tex_limit - 1) as usize).cloned());
        if self.material_textures.len() < tex_limit as usize {
            self.material_textures
                .resize(tex_limit as usize, self.white_texture.clone());
        }
        let tex_views: Vec<&TextureView> =
            self.material_textures.iter().map(|t| &t.0.view).collect();
        self.compute_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("compute_bg"),
            layout: &self.compute_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.triangle_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: self.tri_bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: self.params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&self.color_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&self.depth_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(&self.normal_view),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(&self.gbuf_albedo_view),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: BindingResource::TextureView(&self.gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&self.gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: self.gi_params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: BindingResource::TextureView(&self.gi_sdf_view),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 14,
                    resource: BindingResource::TextureView(&self.gi_history_view),
                },
                BindGroupEntry {
                    binding: 15,
                    resource: BindingResource::TextureView(&self.gi_noisy_view),
                },
                BindGroupEntry {
                    binding: 16,
                    resource: BindingResource::TextureView(&self.gi_radiance_view),
                },
                BindGroupEntry {
                    binding: 17,
                    resource: self.material_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 18,
                    resource: BindingResource::TextureView(&self.lightmap_view),
                },
                BindGroupEntry {
                    binding: 19,
                    resource: self.light_header_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 20,
                    resource: self.light_index_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 21,
                    resource: BindingResource::TextureViewArray(&tex_views),
                },
                BindGroupEntry {
                    binding: 22,
                    resource: BindingResource::Sampler(&self.linear_sampler),
                },
                BindGroupEntry {
                    binding: 23,
                    resource: self.custom_material_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 24,
                    resource: BindingResource::TextureView(&self.sky_view_lut_view),
                },
                BindGroupEntry {
                    binding: 25,
                    resource: BindingResource::TextureView(&self.aerial_perspective_lut_view),
                },
                BindGroupEntry {
                    binding: 26,
                    resource: self.cloud_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 27,
                    resource: BindingResource::TextureView(&self.blue_noise_texture.0.view),
                },
                BindGroupEntry {
                    binding: 28,
                    resource: BindingResource::Sampler(&self.blue_noise_texture.0.sampler),
                },
                BindGroupEntry {
                    binding: 29,
                    resource: BindingResource::TextureView(&self.transmittance_lut_view),
                },
                BindGroupEntry {
                    binding: 30,
                    resource: BindingResource::TextureView(&self.cloud_shape_noise_view),
                },
                BindGroupEntry {
                    binding: 31,
                    resource: BindingResource::TextureView(&self.cloud_detail_noise_view),
                },
                BindGroupEntry {
                    binding: 32,
                    resource: BindingResource::TextureView(&self.cloud_weather_view),
                },
                BindGroupEntry {
                    binding: 33,
                    resource: BindingResource::TextureView(&self.cloud_radiance_view),
                },
                BindGroupEntry {
                    binding: 34,
                    resource: BindingResource::TextureView(&self.cloud_radiance_history_view),
                },
                BindGroupEntry {
                    binding: 35,
                    resource: BindingResource::TextureView(&self.cloud_transmittance_view),
                },
                BindGroupEntry {
                    binding: 36,
                    resource: BindingResource::TextureView(&self.cloud_transmittance_history_view),
                },
                BindGroupEntry {
                    binding: 37,
                    resource: BindingResource::TextureView(&self.cloud_shadow_view),
                },
                BindGroupEntry {
                    binding: 38,
                    resource: BindingResource::TextureView(&self.cloud_shadow_history_view),
                },
                BindGroupEntry {
                    binding: 39,
                    resource: BindingResource::TextureView(&self.screen_history_view),
                },
                BindGroupEntry {
                    binding: 40,
                    resource: self.raster_shadow_view_proj_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 41,
                    resource: BindingResource::TextureView(&self.raster_shadow_view),
                },
                BindGroupEntry {
                    binding: 42,
                    resource: BindingResource::Sampler(&self.raster_shadow_sampler),
                },
                BindGroupEntry {
                    binding: 43,
                    resource: BindingResource::TextureView(&self.gi_buffer_view),
                },
                BindGroupEntry {
                    binding: 44,
                    resource: BindingResource::TextureView(&self.ambient_occlusion_view),
                },
                BindGroupEntry {
                    binding: 45,
                    resource: BindingResource::TextureView(&self.ssr_color_view),
                },
                BindGroupEntry {
                    binding: 46,
                    resource: BindingResource::TextureView(&self.hybrid_rt_reflection_view),
                },
                BindGroupEntry {
                    binding: 47,
                    resource: BindingResource::TextureView(&self.gbuf_lightmap_uv_view),
                },
            ],
        });
        let hybrid_rt_tex_views: Vec<&TextureView> = self.material_textures.iter().map(|t| &t.0.view).collect();
        let make_hybrid_rt_bind_group = |label: &str, out_view: &TextureView| {
            self.device.create_bind_group(&BindGroupDescriptor {
                label: Some(label),
                layout: &self.hybrid_rt_effect_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&self.depth_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&self.gbuf_normal_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&self.gbuf_material_view),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&self.gbuf_albedo_view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(&self.gbuf_material_view),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::TextureView(&self.gbuf_material_view),
                    },
                    BindGroupEntry {
                        binding: 6,
                        resource: BindingResource::TextureView(out_view),
                    },
                    BindGroupEntry {
                        binding: 7,
                        resource: self.hybrid_rt_params_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 8,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 9,
                        resource: self.object_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 10,
                        resource: self.triangle_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 11,
                        resource: self.bvh_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 12,
                        resource: self.tri_bvh_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 13,
                        resource: self.material_buffer.as_entire_binding(),
                    },
                    BindGroupEntry { binding: 14, resource: BindingResource::TextureView(&self.ssr_color_view) },
                    BindGroupEntry { binding: 21, resource: BindingResource::TextureViewArray(&hybrid_rt_tex_views) },
                    BindGroupEntry { binding: 22, resource: BindingResource::Sampler(&self.linear_sampler) },
                ],
            })
        };
        self.hybrid_rt_shadow_bind_group =
            make_hybrid_rt_bind_group("hybrid_rt_shadow_bg", &self.hybrid_rt_shadow_view);
        self.hybrid_rt_reflection_bind_group =
            make_hybrid_rt_bind_group("hybrid_rt_reflection_bg", &self.hybrid_rt_reflection_view);
        self.ssr_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("ssr_bg"),
            layout: &self.ssr_bind_group_layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&self.depth_view) },
                BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&self.gbuf_normal_view) },
                BindGroupEntry { binding: 2, resource: BindingResource::TextureView(&self.gbuf_albedo_view) },
                BindGroupEntry { binding: 3, resource: BindingResource::TextureView(&self.color_view) },
                BindGroupEntry { binding: 4, resource: BindingResource::TextureView(&self.ssr_history_view) },
                BindGroupEntry { binding: 5, resource: BindingResource::TextureView(&self.gbuf_material_view) },
                BindGroupEntry { binding: 6, resource: BindingResource::TextureView(&self.ssr_color_view) },
                BindGroupEntry { binding: 7, resource: self.ssr_params_buffer.as_entire_binding() },
            ],
        });
        self.hybrid_rt_gi_bind_group =
            make_hybrid_rt_bind_group("hybrid_rt_gi_bg", &self.hybrid_rt_gi_view);
        self.hybrid_rt_transparency_bind_group = make_hybrid_rt_bind_group(
            "hybrid_rt_transparency_bg",
            &self.hybrid_rt_transparency_view,
        );
        self.rtao_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("rtao_bg"),
            layout: &self.rtao_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.depth_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&self.gbuf_albedo_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&self.gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&self.gbuf_material_view),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&self.ambient_occlusion_view),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: self.hybrid_rt_params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: self.params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: self.object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: self.triangle_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: self.bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 12,
                    resource: self.tri_bvh_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: self.material_buffer.as_entire_binding(),
                },
            ],
        });

        self.primitive_gbuffer_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("primitive_gbuffer_bg"),
            layout: &self.primitive_gbuffer_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.object_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.sprite_view_proj_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.material_buffer.as_entire_binding(),
                },
            ],
        });

        let shader_changed =
            self.prev_material_names != material_names || self.prev_shader_defs != shaders;
        if shader_changed {
            self.shader_compiler.material_registry.clear();
            for (name, code) in shaders {
                self.shader_compiler
                    .register_material(name.clone(), code.clone());
            }
            self.pending_material_names = material_names.to_vec();
            self.pending_shader_defs = shaders.to_vec();
            self.prev_material_names = material_names.to_vec();
            self.prev_shader_defs = shaders.to_vec();
            self.cinematic_compute_pipeline = None;
            self.cinematic_cloud_shadow_pipeline = None;
            self.cinematic_pipeline_error = None;
            self.cinematic_pipeline_status = LazyPipelineStatus::NotCompiled;
            render_log(
                "custom material set changed; cinematic pathtrace pipeline invalidated and will compile lazily",
            );
        }

        self.denoise_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("denoise_bg"),
            layout: &self.denoise_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 4,
                    resource: self.params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&self.depth_view),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: BindingResource::TextureView(&self.gbuf_normal_view),
                },
                BindGroupEntry {
                    binding: 13,
                    resource: BindingResource::TextureView(&self.gi_sdf_view),
                },
                BindGroupEntry {
                    binding: 14,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 15,
                    resource: BindingResource::TextureView(&self.gi_history_view),
                },
                BindGroupEntry {
                    binding: 16,
                    resource: BindingResource::TextureView(&self.gi_noisy_view),
                },
                BindGroupEntry {
                    binding: 17,
                    resource: BindingResource::TextureView(&self.gi_buffer_view),
                },
                BindGroupEntry {
                    binding: 18,
                    resource: self.postfx_buffer.as_entire_binding(),
                },
            ],
        });
    }

    fn ensure_cinematic_pipeline(&mut self) -> bool {
        if self.safe_shader_mode {
            return false;
        }
        if self.cinematic_compute_pipeline.is_some()
            && self.cinematic_cloud_shadow_pipeline.is_some()
        {
            self.cinematic_pipeline_status = LazyPipelineStatus::Compiled;
            return true;
        }
        if self.cinematic_pipeline_status == LazyPipelineStatus::Failed {
            return false;
        }

        self.cinematic_pipeline_status = LazyPipelineStatus::NotCompiled;
        let started = Instant::now();
        render_log("compiling cinematic pathtrace pipeline lazily...");

        let compute_module = match self
            .shader_compiler
            .compile_shader(&self.pending_material_names)
        {
            Ok(module) => module,
            Err(err) => {
                self.cinematic_pipeline_status = LazyPipelineStatus::Failed;
                self.cinematic_pipeline_error = Some(err.clone());
                render_log(&format!("cinematic pathtrace shader module failed: {err}"));
                return false;
            }
        };

        let compute_pipeline_layout =
            self.device
                .create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: Some("cinematic_compute_pl"),
                    bind_group_layouts: &[&self.compute_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline = self
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("cinematic_pathtrace_compute_pipeline"),
                layout: Some(&compute_pipeline_layout),
                module: &compute_module,
                entry_point: "main",
                compilation_options: Default::default(),
            });
        let cloud_shadow_pipeline =
            self.device
                .create_compute_pipeline(&ComputePipelineDescriptor {
                    label: Some("cinematic_cloud_directional_shadow_pipeline"),
                    layout: Some(&compute_pipeline_layout),
                    module: &compute_module,
                    entry_point: "cloud_shadow_main",
                    compilation_options: Default::default(),
                });

        self.cinematic_compute_pipeline = Some(compute_pipeline);
        self.cinematic_cloud_shadow_pipeline = Some(cloud_shadow_pipeline);
        self.cinematic_pipeline_status = LazyPipelineStatus::Compiled;
        self.cinematic_pipeline_error = None;
        render_log(&format!(
            "cinematic pathtrace pipeline compiled in {:.3}s",
            started.elapsed().as_secs_f64()
        ));
        true
    }

    pub fn render(
        &mut self,
        params: &RenderParams,
        sprites: &[SpriteRenderData],
        pbr_data: &[PbrRenderData],
        egui: Option<WgpuEguiPaint<'_>>,
    ) {
        let frame_start = Instant::now();
        let mut stats = crate::rendering::renderer::RendererProfilerStats::default();
        const PROF_RASTER_BEGIN: u32 = 0;
        const PROF_RASTER_END: u32 = 1;
        const PROF_RT_SHADOW_BEGIN: u32 = 2;
        const PROF_RT_SHADOW_END: u32 = 3;
        const PROF_RT_REFLECTION_BEGIN: u32 = 4;
        const PROF_RT_REFLECTION_END: u32 = 5;
        const PROF_RT_GI_BEGIN: u32 = 6;
        const PROF_RT_GI_END: u32 = 7;
        const PROF_DENOISE_BEGIN: u32 = 8;
        const PROF_DENOISE_END: u32 = 9;
        const PROF_CLOUDS_BEGIN: u32 = 10;
        const PROF_CLOUDS_END: u32 = 11;
        let mut profiled_raster = false;
        let mut profiled_rt_shadow = false;
        let mut profiled_rt_reflection = false;
        let mut profiled_rt_gi = false;
        let mut profiled_denoise = false;
        let mut profiled_clouds = false;
        let halton = |mut idx: i32, base: i32| -> f32 {
            let mut f = 1.0f32;
            let mut r = 0.0f32;
            while idx > 0 {
                f /= base as f32;
                r += f * (idx % base) as f32;
                idx /= base;
            }
            r
        };
        let prev_jitter = self.prev_taa_jitter;
        let jitter_x = (halton(self.frame_number + 1, 2) - 0.5) / self.width as f32;
        let jitter_y = (halton(self.frame_number + 1, 3) - 0.5) / self.height as f32;
        let current_vp = Mat4::from_cols_array_2d(&params.inv_view_proj).inverse();
        let mut effective_max_bounces = params.max_bounces;
        let mut effective_light_samples = params.light_samples;
        let mut effective_dir_shadow_samples = params.dir_shadow_samples;
        let mut effective_raytraced_shadows = params.raytraced_shadows_enabled;
        let mut effective_shadow_quality = params.shadow_quality;
        let mut effective_max_shadow_rays = params.max_shadow_rays;
        let mut effective_cloud_object_shadows = params.cloud_object_shadows_enabled;
        let mut effective_gi_quality = params.gi_quality;
        let mut effective_gi_mode = params.gi_mode;
        let mut effective_cloud_sample_count = params.cloud_sample_count;
        let mut effective_cloud_temporal_quality = params.cloud_temporal_quality;
        let mut effective_cloud_shadow_mode = params.cloud_shadow_mode;
        let mut effective_cloud_count = params
            .clouds
            .len()
            .min(crate::scene::object::MAX_VOLUMETRIC_CLOUDS)
            as u32;
        let mut effective_renderer_mode = params.renderer_mode;
        match params.profile {
            crate::rendering::renderer::RendererProfile::Indoor60FPS => {
                effective_renderer_mode = crate::rendering::renderer::RendererMode::HybridEffects;
                effective_gi_mode = GI_MODE_LIGHT_PROBES;
                effective_gi_quality = effective_gi_quality.min(2);
                effective_max_bounces = 1;
                effective_raytraced_shadows = 0;
                effective_shadow_quality = 0;
                effective_max_shadow_rays = 0;
                effective_cloud_object_shadows = 0;
                effective_cloud_sample_count = if effective_cloud_sample_count == 0 {
                    12
                } else {
                    effective_cloud_sample_count.min(12)
                };
                effective_cloud_temporal_quality = 1;
                // Indoor maps should not spend time on heavy sky/cloud work when
                // the sky is normally occluded. Keep atmosphere LUTs available for
                // windows/portals, but make cloud passes a no-op.
                effective_cloud_count = 0;
            }
            crate::rendering::renderer::RendererProfile::Low => {
                effective_gi_quality = effective_gi_quality.min(2);
                if matches!(
                    effective_gi_mode,
                    GI_MODE_RTGI_ONE_BOUNCE | GI_MODE_PATH_TRACED_PREVIEW
                ) {
                    effective_gi_mode = GI_MODE_LIGHT_PROBES;
                }
                effective_max_bounces = effective_max_bounces.min(1);
                effective_light_samples = effective_light_samples.min(1);
                effective_dir_shadow_samples = effective_dir_shadow_samples.min(1);
                effective_max_shadow_rays = effective_max_shadow_rays.min(1);
                effective_cloud_sample_count = if effective_cloud_sample_count == 0 {
                    16
                } else {
                    effective_cloud_sample_count.min(16)
                };
            }
            crate::rendering::renderer::RendererProfile::Cinematic => {
                effective_renderer_mode =
                    crate::rendering::renderer::RendererMode::CinematicPathTrace;
                // Cinematic can opt into expensive per-hit volumetric cloud shadowing.
                effective_cloud_shadow_mode = 1;
            }
            _ => {}
        }

        let hardware = crate::rendering::renderer::RendererHardwareCapabilities {
            rt_shadows: self.hybrid_rt_shadow_pipeline.is_some(),
            rt_reflections: self.hybrid_rt_reflection_pipeline.is_some(),
            rt_gi: self.hybrid_rt_gi_pipeline.is_some(),
            rt_transparency: self.hybrid_rt_transparency_pipeline.is_some(),
            rt_ao: self.rtao_pipeline.is_some(),
            path_tracing: !self.safe_shader_mode,
        };
        let policy = crate::rendering::renderer::RendererPolicy::derive(
            params,
            hardware,
            self.prev_material_fallback_tags,
            self.adaptive_quality,
        );
        let ssr_quality = SsrQuality::for_profile(params.profile, self.adaptive_quality);
        effective_renderer_mode = policy.renderer_mode;

        // The policy is the single source of truth for feature routing. Keep the legacy
        // GI constants only as shader ABI values derived from the policy decision.
        let uses_path_traced_primary = policy.primary_visibility
            == crate::rendering::renderer::PrimaryVisibilityMethod::PathTraced;
        let uses_raytraced_primary = policy.primary_visibility
            == crate::rendering::renderer::PrimaryVisibilityMethod::Raytraced;
        let uses_rt_primary = uses_path_traced_primary || uses_raytraced_primary;
        let uses_hybrid_effects = effective_renderer_mode.uses_decomposed_rt_effects();
        let mut final_compositor_wrote_screen = false;
        let mut dispatch_sdfgi = false;
        let mut dispatch_hybrid_rtgi = false;
        effective_gi_mode = match policy.gi {
            crate::rendering::renderer::GiMethod::Off => GI_MODE_OFF,
            crate::rendering::renderer::GiMethod::BakedLightmap => GI_MODE_BAKED_LIGHTMAP,
            crate::rendering::renderer::GiMethod::LightProbes => GI_MODE_LIGHT_PROBES,
            crate::rendering::renderer::GiMethod::SDFGI => {
                dispatch_sdfgi = true;
                GI_MODE_SDFGI
            }
            crate::rendering::renderer::GiMethod::RTGIOneBounce => {
                dispatch_hybrid_rtgi = true;
                GI_MODE_RTGI_ONE_BOUNCE
            }
            crate::rendering::renderer::GiMethod::PathTraced => GI_MODE_PATH_TRACED_PREVIEW,
        };
        let mut gi_resolve_method = match policy.gi {
            crate::rendering::renderer::GiMethod::Off | crate::rendering::renderer::GiMethod::PathTraced => GI_RESOLVE_METHOD_OFF,
            crate::rendering::renderer::GiMethod::BakedLightmap => GI_RESOLVE_METHOD_BAKED_LIGHTMAP,
            crate::rendering::renderer::GiMethod::LightProbes => GI_RESOLVE_METHOD_LIGHT_PROBES,
            crate::rendering::renderer::GiMethod::SDFGI => GI_RESOLVE_METHOD_SDFGI,
            crate::rendering::renderer::GiMethod::RTGIOneBounce => GI_RESOLVE_METHOD_RTGI_ONE_BOUNCE,
        };
        let baked_gi_ready = self.gi_cache.has_lightmap_atlas && self.gi_cache.has_lightmap_uvs;
        let probe_gi_ready = self.gi_cache.has_probe_data && self.gi_probe_count > 0;
        let sdfgi_ready = self.gi_cache.has_sdfgi_volume && !self.gi_cache.dirty;
        if gi_resolve_method == GI_RESOLVE_METHOD_BAKED_LIGHTMAP && !baked_gi_ready {
            gi_resolve_method = GI_RESOLVE_METHOD_OFF;
        }
        if gi_resolve_method == GI_RESOLVE_METHOD_LIGHT_PROBES && !probe_gi_ready {
            gi_resolve_method = GI_RESOLVE_METHOD_OFF;
        }
        if gi_resolve_method == GI_RESOLVE_METHOD_SDFGI && !sdfgi_ready {
            gi_resolve_method = GI_RESOLVE_METHOD_OFF;
        }
        if gi_resolve_method == GI_RESOLVE_METHOD_RTGI_ONE_BOUNCE && self.hybrid_rt_gi_pipeline.is_none() {
            gi_resolve_method = GI_RESOLVE_METHOD_OFF;
            dispatch_hybrid_rtgi = false;
        }

        let wants_cinematic_pipeline = uses_rt_primary && !self.safe_shader_mode;
        let cinematic_pipeline_ready = if wants_cinematic_pipeline {
            if self.active_compute_pipeline_kind != MainComputePipelineKind::CinematicPathTrace {
                render_log("Renderer mode changed to CinematicPathTrace/PathTracePreview");
            }
            self.ensure_cinematic_pipeline()
        } else {
            if self.active_compute_pipeline_kind != MainComputePipelineKind::HybridCompose
                && self.hybrid_compose_pipeline.is_some()
                && !self.safe_shader_mode
            {
                render_log(&format!(
                    "using raster/hybrid pipeline for mode {:?}",
                    effective_renderer_mode
                ));
            }
            false
        };
        self.active_compute_pipeline_kind = if cinematic_pipeline_ready {
            MainComputePipelineKind::CinematicPathTrace
        } else if !wants_cinematic_pipeline
            && !self.safe_shader_mode
            && self.hybrid_compose_pipeline.is_some()
        {
            MainComputePipelineKind::HybridCompose
        } else {
            if wants_cinematic_pipeline
                && self.active_compute_pipeline_kind != MainComputePipelineKind::Bootstrap
            {
                // TODO: Replace bootstrap fallback with lightweight raster/hybrid compute pipeline.
                render_log("cinematic pathtrace pipeline unavailable; using bootstrap fallback");
            } else if self.safe_shader_mode
                && self.active_compute_pipeline_kind != MainComputePipelineKind::Bootstrap
            {
                render_log("using bootstrap fallback because VETRACE_SAFE_SHADER=1 is set");
            } else if let Some(err) = &self.hybrid_compose_pipeline_error
                && self.active_compute_pipeline_kind != MainComputePipelineKind::Bootstrap
            {
                render_log(&format!(
                    "using bootstrap fallback because lightweight hybrid pipeline failed: {}",
                    err
                ));
            }
            MainComputePipelineKind::Bootstrap
        };
        let profiler_query_set = self.profiler_query_set.as_ref();
        let has_bvh_accel_data = self.bvh_node_count > 0 && self.tri_bvh_node_count > 0;
        let profile_budget_downgraded = self.adaptive_quality < 0.99;
        let pipeline_reason = |pipeline_ready: bool| {
            if self.safe_shader_mode {
                crate::rendering::renderer::RendererFallbackReason::SafeShaderMode
            } else if !pipeline_ready {
                crate::rendering::renderer::RendererFallbackReason::MissingPipeline
            } else if !has_bvh_accel_data {
                crate::rendering::renderer::RendererFallbackReason::MissingBvhAccelerationData
            } else if profile_budget_downgraded {
                crate::rendering::renderer::RendererFallbackReason::ProfileBudgetDowngrade
            } else {
                crate::rendering::renderer::RendererFallbackReason::MissingHardware
            }
        };

        let mut feature_status = crate::rendering::renderer::RendererHybridFeatureStatus {
            pathtrace_primary_active: cinematic_pipeline_ready && uses_path_traced_primary,
            requested_primary_visibility_method: policy.primary_visibility,
            active_primary_visibility_method: if cinematic_pipeline_ready && uses_path_traced_primary {
                crate::rendering::renderer::PrimaryVisibilityMethod::PathTraced
            } else if cinematic_pipeline_ready && uses_raytraced_primary {
                crate::rendering::renderer::PrimaryVisibilityMethod::Raytraced
            } else {
                crate::rendering::renderer::PrimaryVisibilityMethod::Raster
            },
            requested_shadow_method: policy.shadows,
            requested_reflection_method: policy.reflections,
            requested_ambient_occlusion_method: policy.ambient_occlusion,
            requested_gi_method: policy.gi,
            requested_transparency_method: policy.transparency,
            active_transparency_method: match policy.transparency {
                crate::rendering::renderer::TransparencyMethod::PathTraced
                    if cinematic_pipeline_ready && uses_path_traced_primary =>
                {
                    crate::rendering::renderer::TransparencyMethod::PathTraced
                }
                crate::rendering::renderer::TransparencyMethod::Raytraced
                    if uses_hybrid_effects && self.hybrid_rt_transparency_pipeline.is_some() =>
                {
                    crate::rendering::renderer::TransparencyMethod::Raytraced
                }
                crate::rendering::renderer::TransparencyMethod::Raytraced => {
                    crate::rendering::renderer::TransparencyMethod::ScreenSpaceRefraction
                }
                crate::rendering::renderer::TransparencyMethod::PathTraced => crate::rendering::renderer::TransparencyMethod::RasterAlpha,
                method => method,
            },
            ..Default::default()
        };

        let adaptive_sample_cap = if self.adaptive_quality < 0.67 {
            1
        } else if self.adaptive_quality < 0.9 {
            2
        } else {
            8
        };
        effective_light_samples = effective_light_samples.min(adaptive_sample_cap);
        effective_dir_shadow_samples = effective_dir_shadow_samples.min(adaptive_sample_cap);
        effective_max_shadow_rays = effective_max_shadow_rays.min(adaptive_sample_cap as u32);

        if effective_renderer_mode.uses_decomposed_rt_effects() {
            if self.safe_shader_mode {
                render_log("optional AO/SSR/GI pipelines disabled by VETRACE_SAFE_SHADER=1; using initialized fallback resolve textures");
            }
            feature_status.hybrid_rt_shadows_active = matches!(
                policy.shadows,
                crate::rendering::renderer::ShadowMethod::Raytraced
                    | crate::rendering::renderer::ShadowMethod::RasterPlusRtContact
            ) && self.hybrid_rt_shadow_pipeline.is_some();
            feature_status.ssr_reflections_active = matches!(
                policy.reflections,
                crate::rendering::renderer::ReflectionMethod::SSR
                    | crate::rendering::renderer::ReflectionMethod::SsrThenRtFallback
            ) && self.ssr_pipeline.is_some();
            feature_status.hybrid_rt_reflections_active = matches!(
                policy.reflections,
                crate::rendering::renderer::ReflectionMethod::SsrThenRtFallback
                    | crate::rendering::renderer::ReflectionMethod::Raytraced
            ) && self.hybrid_rt_reflection_pipeline.is_some();
            feature_status.hybrid_rtgi_active =
                dispatch_hybrid_rtgi && self.hybrid_rt_gi_pipeline.is_some();
            if matches!(
                policy.reflections,
                crate::rendering::renderer::ReflectionMethod::SSR
                    | crate::rendering::renderer::ReflectionMethod::SsrThenRtFallback
            ) && self.ssr_pipeline.is_none()
            {
                render_log("SSR pipeline unavailable; compositor will sample transparent SSR fallback");
            }
            if dispatch_hybrid_rtgi && self.hybrid_rt_gi_pipeline.is_none() {
                render_log("RTGI producer pipeline unavailable; GI resolve will use black RTGI fallback");
            }
            feature_status.active_reflection_method = if feature_status.hybrid_rt_reflections_active
                && feature_status.ssr_reflections_active
            {
                crate::rendering::renderer::ReflectionMethod::SsrThenRtFallback
            } else if feature_status.hybrid_rt_reflections_active {
                crate::rendering::renderer::ReflectionMethod::Raytraced
            } else if feature_status.ssr_reflections_active {
                crate::rendering::renderer::ReflectionMethod::SSR
            } else if matches!(
                policy.reflections,
                crate::rendering::renderer::ReflectionMethod::Probe
            ) {
                crate::rendering::renderer::ReflectionMethod::Probe
            } else {
                crate::rendering::renderer::ReflectionMethod::Off
            };
        } else {
            feature_status.active_reflection_method = if matches!(
                policy.reflections,
                crate::rendering::renderer::ReflectionMethod::PathTraced
            ) && cinematic_pipeline_ready
                && uses_path_traced_primary
            {
                crate::rendering::renderer::ReflectionMethod::PathTraced
            } else if matches!(
                policy.reflections,
                crate::rendering::renderer::ReflectionMethod::Probe
            ) {
                crate::rendering::renderer::ReflectionMethod::Probe
            } else if matches!(
                policy.reflections,
                crate::rendering::renderer::ReflectionMethod::SSR
            ) {
                crate::rendering::renderer::ReflectionMethod::SSR
            } else {
                crate::rendering::renderer::ReflectionMethod::Off
            };
        }
        feature_status.raster_shadow_maps_active = matches!(
            policy.shadows,
            crate::rendering::renderer::ShadowMethod::RasterShadowMap
                | crate::rendering::renderer::ShadowMethod::CascadedShadowMap
                | crate::rendering::renderer::ShadowMethod::RasterPlusRtContact
        );
        feature_status.active_shadow_method = if feature_status.hybrid_rt_shadows_active {
            match policy.shadows {
                crate::rendering::renderer::ShadowMethod::RasterPlusRtContact => {
                    crate::rendering::renderer::ShadowMethod::RasterPlusRtContact
                }
                crate::rendering::renderer::ShadowMethod::Raytraced => {
                    crate::rendering::renderer::ShadowMethod::Raytraced
                }
                _ => crate::rendering::renderer::ShadowMethod::RasterShadowMap,
            }
        } else if feature_status.raster_shadow_maps_active {
            // Only a single raster shadow map is allocated today. Keep requested CSM visible
            // through requested_shadow_method, but report the active method truthfully until
            // a real cascade atlas/matrix-array path exists.
            crate::rendering::renderer::ShadowMethod::RasterShadowMap
        } else {
            crate::rendering::renderer::ShadowMethod::Off
        };
        if feature_status.requested_primary_visibility_method != feature_status.active_primary_visibility_method {
            feature_status.primary_visibility_fallback_reason = pipeline_reason(cinematic_pipeline_ready);
        }
        if feature_status.requested_shadow_method != feature_status.active_shadow_method {
            feature_status.shadow_fallback_reason = pipeline_reason(self.hybrid_rt_shadow_pipeline.is_some());
        }
        if feature_status.requested_reflection_method != feature_status.active_reflection_method {
            feature_status.reflection_fallback_reason = pipeline_reason(
                self.ssr_pipeline.is_some() || self.hybrid_rt_reflection_pipeline.is_some(),
            );
        }
        if feature_status.requested_transparency_method != feature_status.active_transparency_method {
            feature_status.transparency_fallback_reason = pipeline_reason(self.hybrid_rt_transparency_pipeline.is_some());
        }
        if policy.gi == crate::rendering::renderer::GiMethod::BakedLightmap && !baked_gi_ready {
            feature_status.gi_fallback_reason = crate::rendering::renderer::RendererFallbackReason::MissingLightmaps;
        } else if policy.gi == crate::rendering::renderer::GiMethod::LightProbes && !probe_gi_ready {
            feature_status.gi_fallback_reason = crate::rendering::renderer::RendererFallbackReason::MissingProbes;
        } else if policy.gi != feature_status.active_gi_method {
            feature_status.gi_fallback_reason = pipeline_reason(self.hybrid_rt_gi_pipeline.is_some());
        }

        let effective_raytraced_shadows_param =
            if effective_renderer_mode.uses_decomposed_rt_effects() {
                u32::from(feature_status.hybrid_rt_shadows_active)
            } else {
                effective_raytraced_shadows
            };
        let effective_raytraced_reflections =
            if effective_renderer_mode.uses_decomposed_rt_effects() {
                u32::from(feature_status.hybrid_rt_reflections_active)
            } else {
                params.raytraced_reflections_enabled
            };

        let shader_params = ShaderParams {
            camera_pos: [
                params.camera_pos[0],
                params.camera_pos[1],
                params.camera_pos[2],
                0.0,
            ],
            camera_front: [
                params.camera_front[0],
                params.camera_front[1],
                params.camera_front[2],
                0.0,
            ],
            camera_up: [
                params.camera_up[0],
                params.camera_up[1],
                params.camera_up[2],
                0.0,
            ],
            camera_right: [
                params.camera_right[0],
                params.camera_right[1],
                params.camera_right[2],
                0.0,
            ],
            prev_camera_pos: [
                self.prev_cam_pos[0],
                self.prev_cam_pos[1],
                self.prev_cam_pos[2],
                0.0,
            ],
            fov: params.fov,
            num_objects: params.num_objects,
            is_fisheye: params.is_fisheye,
            _pad0: 0,
            skycolor: [
                params.skycolor[0],
                params.skycolor[1],
                params.skycolor[2],
                0.0,
            ],
            taa_jitter: [jitter_x, jitter_y],
            current_time: params.current_time,
            frame_number: self.frame_number,
            selected_index: params.selected_index,
            max_bounces: effective_max_bounces,
            light_samples: effective_light_samples,
            dir_shadow_samples: effective_dir_shadow_samples,
            shadow_mode: params.shadow_mode,
            raytraced_shadows_enabled: effective_raytraced_shadows_param,
            shadow_quality: effective_shadow_quality,
            max_shadow_rays: effective_max_shadow_rays,
            emissive_shadow_samples: params.emissive_shadow_samples,
            directional_shadow_samples: params.directional_shadow_samples,
            cloud_object_shadows_enabled: effective_cloud_object_shadows,
            max_rt_shadow_distance: params.max_rt_shadow_distance,
            rt_shadow_ray_t_max: params.rt_shadow_ray_t_max,
            min_soft_shadow_radius: params.min_soft_shadow_radius,
            raytraced_reflections_enabled: effective_raytraced_reflections,
            _pad_reflections: 0,
            inv_view_proj: params.inv_view_proj,
            prev_view_proj: self.prev_view_proj,
            dir_light_dir: [
                params.dir_light_dir[0],
                params.dir_light_dir[1],
                params.dir_light_dir[2],
                params.dir_light_intensity,
            ],
            dir_light_color: [
                params.dir_light_color[0],
                params.dir_light_color[1],
                params.dir_light_color[2],
                0.0,
            ],
            sky_occlusion: params.sky_occlusion,
            total_triangles: self.triangle_count,
            total_bvh_nodes: self.bvh_node_count,
            total_tri_bvh_nodes: self.tri_bvh_node_count,
            dof_aperture: params.dof_aperture,
            dof_focus_dist: params.dof_focus_dist,
            dof_enable: params.dof_enable,
            _pad_dof: 0,
            atmosphere: params.atmosphere,
            atmo_count: params.atmos.len() as u32,
            cloud_count: effective_cloud_count,
            atmosphere_mode: params.atmosphere_mode,
            atmosphere_sun_controls: params.atmosphere_sun_controls,
            cloud_history_weight: params.cloud_history_weight,
            cloud_sample_count: effective_cloud_sample_count,
            cloud_temporal_quality: effective_cloud_temporal_quality,
            cloud_shadow_mode: effective_cloud_shadow_mode,
            renderer_mode: effective_renderer_mode as u32,
            rt_debug_view: params.rt_debug_view,
            rt_debug_counters: params.rt_debug_counters,
            max_traversal_steps: params.max_traversal_steps.max(1),
            max_transparent_surfaces: params.max_transparent_surfaces,
            shadow_max_distance: params.shadow_max_distance.max(0.01),
            reflection_max_distance: params.reflection_max_distance.max(0.01),
            gi_max_distance: params.gi_max_distance.max(0.01),
            min_ray_offset: params.min_ray_offset.max(0.00001),
            _pad_atmos: [0; 7],
            atmos: {
                let mut arr = [GpuAtmosphere::default(); MAX_ATMOSPHERES];
                let count = params.atmos.len().min(MAX_ATMOSPHERES);
                arr[..count].copy_from_slice(&params.atmos[..count]);
                arr
            },
        };
        if !params.clouds.is_empty() {
            let count = params
                .clouds
                .len()
                .min(crate::scene::object::MAX_VOLUMETRIC_CLOUDS);
            self.queue.write_buffer(
                &self.cloud_buffer,
                0,
                bytemuck::cast_slice(&params.clouds[..count]),
            );
        }
        if self.prev_shader_params.map_or(true, |p| p != shader_params) {
            self.queue
                .write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&shader_params));
            self.prev_shader_params = Some(shader_params);
        }
        let gi_params = GiParams {
            quality: effective_gi_quality,
            debug_mode: params.gi_debug_mode,
            mode: effective_gi_mode,
            _pad: 0,
        };
        if self.prev_gi_params.map_or(true, |p| p != gi_params) {
            self.queue
                .write_buffer(&self.gi_params_buffer, 0, bytemuck::bytes_of(&gi_params));
            self.prev_gi_params = Some(gi_params);
            self.gi_cache.mark_dirty();
        }
        let gi_resolve_params = GiResolveParams {
            selected_method: gi_resolve_method,
            frame_number: self.frame_number.max(0) as u32,
            debug_flags: params.gi_debug_mode | (params.rt_debug_view << 16),
            _pad0: 0,
            temporal_blend: if gi_resolve_method == GI_RESOLVE_METHOD_RTGI_ONE_BOUNCE { self.post_fx_uniforms.gi_temporal_blend } else { 0.0 },
            baked_blend: 1.0,
            probe_blend: 1.0,
            sdfgi_blend: 1.0,
            rtgi_blend: 1.0,
            probe_count: self.gi_probe_count,
            gi_resource_flags: (self.gi_cache.has_lightmap_atlas as u32)
                | ((self.gi_cache.has_lightmap_uvs as u32) << 1)
                | ((self.gi_cache.has_probe_data as u32) << 2)
                | ((self.gi_cache.has_sdfgi_volume as u32) << 3),
            _pad1: [0; 5],
            sdfgi_origin: [-32.0, -32.0, -32.0, 0.0],
            sdfgi_extent_voxel: [64.0, 64.0, 64.0 / GI_SDF_RES as f32, 0.05],
            inv_view_proj: params.inv_view_proj,
            prev_view_proj: self.prev_view_proj,
        };
        if self.prev_gi_resolve_params.map_or(true, |p| p != gi_resolve_params) {
            self.queue.write_buffer(&self.gi_resolve_params_buffer, 0, bytemuck::bytes_of(&gi_resolve_params));
            self.prev_gi_resolve_params = Some(gi_resolve_params);
        }
        if self
            .prev_post_fx_uniforms
            .map_or(true, |p| p != self.post_fx_uniforms)
        {
            self.queue.write_buffer(
                &self.postfx_buffer,
                0,
                bytemuck::bytes_of(&self.post_fx_uniforms),
            );
            self.prev_post_fx_uniforms = Some(self.post_fx_uniforms);
        }
        let blit_params = BlitParams {
            camera_pos: [
                params.camera_pos[0],
                params.camera_pos[1],
                params.camera_pos[2],
                0.0,
            ],
            prev_camera_pos: [
                self.prev_cam_pos[0],
                self.prev_cam_pos[1],
                self.prev_cam_pos[2],
                0.0,
            ],
            inv_view_proj: params.inv_view_proj,
            prev_view_proj: self.prev_view_proj,
            taa_jitter: [jitter_x, jitter_y],
            prev_taa_jitter: prev_jitter,
            tex_size: [self.width as f32, self.height as f32],
            sharpness: self.sharpness,
            selected_index: params.selected_index,
            _pad0: [0; 2],
            _pad1: [0.0; 2],
        };
        if self.prev_blit_params.map_or(true, |p| p != blit_params) {
            self.queue.write_buffer(
                &self.blit_params_buffer,
                0,
                bytemuck::bytes_of(&blit_params),
            );
            self.prev_blit_params = Some(blit_params);
        }

        // Update sprite uniform buffers
        let cam_pos = Vec3::from(params.camera_pos);
        let cam_front = Vec3::from(params.camera_front);
        let cam_up = Vec3::from(params.camera_up);
        let aspect = self.surface_width as f32 / self.surface_height as f32;
        let view_proj = if self.is_2d {
            let scale = self.surface_height as f32 / (params.fov * 10.0);
            let sx = 2.0 * scale / self.surface_width as f32;
            let sy = 2.0 * scale / self.surface_height as f32;
            Mat4::from_cols_array(&[
                sx,
                0.0,
                0.0,
                0.0,
                0.0,
                sy,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                -cam_pos.x * sx,
                -cam_pos.y * sy,
                0.0,
                1.0,
            ])
        } else {
            OPENGL_TO_WGPU_MATRIX
                * perspective(params.fov, aspect, 0.1, 1000.0)
                * look_at(&cam_pos, &(cam_pos + cam_front), &cam_up)
        };
        let light_dir_for_shadow = Vec3::new(
            params.dir_light_dir[0],
            params.dir_light_dir[1],
            params.dir_light_dir[2],
        )
        .normalize_or_zero();
        let shadow_light_dir = if light_dir_for_shadow.length_squared() > 0.0 {
            -light_dir_for_shadow
        } else {
            Vec3::new(-0.35, -0.8, -0.45).normalize()
        };
        let shadow_center = cam_pos + cam_front * 35.0;
        let shadow_view = look_at(
            &(shadow_center - shadow_light_dir * 80.0),
            &shadow_center,
            &Vec3::Y,
        );
        let shadow_proj =
            OPENGL_TO_WGPU_MATRIX * Mat4::orthographic_rh(-70.0, 70.0, -70.0, 70.0, 0.1, 180.0);
        let shadow_view_proj = shadow_proj * shadow_view;
        let shadow_view_proj_arr = shadow_view_proj.to_cols_array();
        self.queue.write_buffer(
            &self.raster_shadow_view_proj_buffer,
            0,
            bytemuck::cast_slice(&shadow_view_proj_arr),
        );

        let view_proj_arr = view_proj.to_cols_array();
        if self
            .prev_sprite_view_proj
            .map_or(true, |p| p != view_proj_arr)
        {
            self.queue.write_buffer(
                &self.sprite_view_proj_buffer,
                0,
                bytemuck::cast_slice(&view_proj_arr),
            );
            self.prev_sprite_view_proj = Some(view_proj_arr);
        }

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(SurfaceError::Outdated) => return,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                match self.surface.get_current_texture() {
                    Ok(f) => f,
                    Err(e) => panic!("Failed to acquire next swap chain texture: {:?}", e),
                }
            }
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("render"),
            });
        {
            // Neutral fallbacks for optional hybrid inputs. Active passes overwrite these
            // later in the frame; inactive passes leave composition inputs safe.
            let _fallback_clear = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("hybrid_input_fallback_clear"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.ambient_occlusion_view,
                        resolve_target: None,
                        ops: Operations { load: LoadOp::Clear(Color::WHITE), store: StoreOp::Store },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.ssr_color_view,
                        resolve_target: None,
                        ops: Operations { load: LoadOp::Clear(Color::BLACK), store: StoreOp::Store },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.hybrid_rt_reflection_view,
                        resolve_target: None,
                        ops: Operations { load: LoadOp::Clear(Color::BLACK), store: StoreOp::Store },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gi_buffer_view,
                        resolve_target: None,
                        ops: Operations { load: LoadOp::Clear(Color::BLACK), store: StoreOp::Store },
                    }),
                ],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        let mut cube_instances = Vec::new();
        let mut sphere_instances = Vec::new();
        let mut shadow_cube_instances = Vec::new();
        let mut shadow_sphere_instances = Vec::new();
        if !self.is_2d && !effective_renderer_mode.uses_rt_primary_visibility() {
            for (idx, obj) in self.prev_objects.iter().enumerate() {
                if obj.is_mesh != 0 || obj.is_shaded == 0 {
                    continue;
                }
                let instance = PrimitiveInstance {
                    object_index: idx as u32,
                    _pad: [0; 3],
                };
                if obj.is_cube != 0 {
                    cube_instances.push(instance);
                    if obj.casts_raster_shadow != 0 {
                        shadow_cube_instances.push(instance);
                    }
                } else {
                    sphere_instances.push(instance);
                    if obj.casts_raster_shadow != 0 {
                        shadow_sphere_instances.push(instance);
                    }
                }
            }
            let primitive_count = (cube_instances.len() + sphere_instances.len()) as u32;
            if self.prev_raster_primitive_count != Some(primitive_count) {
                render_log(&format!(
                    "raster primitive visibility: objects={}",
                    primitive_count
                ));
                self.prev_raster_primitive_count = Some(primitive_count);
            }
        }
        let has_primitive_gbuffer = !cube_instances.is_empty() || !sphere_instances.is_empty();
        {
            let cube_instance_buffer = if cube_instances.is_empty() {
                None
            } else {
                Some(self.device.create_buffer_init(&util::BufferInitDescriptor {
                    label: Some("primitive_cube_instances"),
                    contents: bytemuck::cast_slice(&cube_instances),
                    usage: BufferUsages::VERTEX,
                }))
            };
            let sphere_instance_buffer = if sphere_instances.is_empty() {
                None
            } else {
                Some(self.device.create_buffer_init(&util::BufferInitDescriptor {
                    label: Some("primitive_sphere_instances"),
                    contents: bytemuck::cast_slice(&sphere_instances),
                    usage: BufferUsages::VERTEX,
                }))
            };
            let shadow_cube_instance_buffer = if shadow_cube_instances.is_empty() {
                None
            } else {
                Some(self.device.create_buffer_init(&util::BufferInitDescriptor {
                    label: Some("primitive_shadow_cube_instances"),
                    contents: bytemuck::cast_slice(&shadow_cube_instances),
                    usage: BufferUsages::VERTEX,
                }))
            };
            let shadow_sphere_instance_buffer = if shadow_sphere_instances.is_empty() {
                None
            } else {
                Some(self.device.create_buffer_init(&util::BufferInitDescriptor {
                    label: Some("primitive_shadow_sphere_instances"),
                    contents: bytemuck::cast_slice(&shadow_sphere_instances),
                    usage: BufferUsages::VERTEX,
                }))
            };
            if !shadow_cube_instances.is_empty()
                || !shadow_sphere_instances.is_empty()
                || !pbr_data.is_empty()
            {
                let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("raster_shadow_primitives"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &self.raster_shadow_view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(1.0),
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                rpass.set_pipeline(&self.primitive_shadow_pipeline);
                rpass.set_bind_group(0, &self.primitive_gbuffer_bind_group, &[]);
                if let Some(buffer) = &cube_instance_buffer {
                    if let Some(shadow_buffer) = &shadow_cube_instance_buffer {
                        rpass.set_vertex_buffer(0, self.primitive_cube_vertex_buffer.slice(..));
                        rpass.set_vertex_buffer(1, shadow_buffer.slice(..));
                        rpass.set_index_buffer(
                            self.primitive_cube_index_buffer.slice(..),
                            IndexFormat::Uint32,
                        );
                        rpass.draw_indexed(
                            0..self.primitive_cube_index_count,
                            0,
                            0..shadow_cube_instances.len() as u32,
                        );
                    }
                    _ = buffer;
                }
                if let Some(shadow_buffer) = &shadow_sphere_instance_buffer {
                    rpass.set_vertex_buffer(0, self.primitive_sphere_vertex_buffer.slice(..));
                    rpass.set_vertex_buffer(1, shadow_buffer.slice(..));
                    rpass.set_index_buffer(
                        self.primitive_sphere_index_buffer.slice(..),
                        IndexFormat::Uint32,
                    );
                    rpass.draw_indexed(
                        0..self.primitive_sphere_index_count,
                        0,
                        0..shadow_sphere_instances.len() as u32,
                    );
                }
            }
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("primitive_gbuffer"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.gbuf_albedo_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gbuf_normal_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gbuf_material_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.depth_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::WHITE),
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gbuf_lightmap_uv_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
                            store: StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_stencil_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: profiler_query_set.map(|query_set| RenderPassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(PROF_RASTER_BEGIN),
                    end_of_pass_write_index: Some(PROF_RASTER_END),
                }),
            });
            profiled_raster = profiler_query_set.is_some();
            if has_primitive_gbuffer {
                rpass.set_pipeline(&self.primitive_gbuffer_pipeline);
                rpass.set_bind_group(0, &self.primitive_gbuffer_bind_group, &[]);
                if let Some(buffer) = &cube_instance_buffer {
                    rpass.set_vertex_buffer(0, self.primitive_cube_vertex_buffer.slice(..));
                    rpass.set_vertex_buffer(1, buffer.slice(..));
                    rpass.set_index_buffer(
                        self.primitive_cube_index_buffer.slice(..),
                        IndexFormat::Uint32,
                    );
                    rpass.draw_indexed(
                        0..self.primitive_cube_index_count,
                        0,
                        0..cube_instances.len() as u32,
                    );
                }
                if let Some(buffer) = &sphere_instance_buffer {
                    rpass.set_vertex_buffer(0, self.primitive_sphere_vertex_buffer.slice(..));
                    rpass.set_vertex_buffer(1, buffer.slice(..));
                    rpass.set_index_buffer(
                        self.primitive_sphere_index_buffer.slice(..),
                        IndexFormat::Uint32,
                    );
                    rpass.draw_indexed(
                        0..self.primitive_sphere_index_count,
                        0,
                        0..sphere_instances.len() as u32,
                    );
                }
            }
        }
        if !pbr_data.is_empty() {
            feature_status.raster_shadow_maps_active = true;
            let mut bind_groups = Vec::new();
            let mut shadow_bind_groups = Vec::new();
            for inst in pbr_data {
                let uni = Uniforms {
                    mvp: inst.mvp,
                    model: inst.model,
                };
                let uni_buf = self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("pbr_mvp"),
                        contents: bytemuck::bytes_of(&uni),
                        usage: BufferUsages::UNIFORM,
                    });
                let mat_uni = MaterialUniforms {
                    base_color: inst.material.base_color,
                    metallic: inst.material.metallic,
                    roughness: inst.material.roughness,
                    _pad: [0.0; 2],
                };
                let mat_buf = self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("pbr_mat"),
                        contents: bytemuck::bytes_of(&mat_uni),
                        usage: BufferUsages::UNIFORM,
                    });
                let tex = inst
                    .material
                    .base_color_tex
                    .as_ref()
                    .map(|t| &t.0)
                    .unwrap_or(&self.white_texture.0);
                // Ensure joint buffers meet the minimum size required by the shader
                // by allocating space for at least 64 matrices.
                const MIN_JOINT_CAPACITY: usize = 64;
                let mut joint_data: Vec<[[f32; 4]; 4]> =
                    inst.joint_mats.clone().unwrap_or_else(|| {
                        vec![[
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, 1.0, 0.0, 0.0],
                            [0.0, 0.0, 1.0, 0.0],
                            [0.0, 0.0, 0.0, 1.0],
                        ]]
                    });
                if joint_data.len() < MIN_JOINT_CAPACITY {
                    joint_data.resize(
                        MIN_JOINT_CAPACITY,
                        [
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, 1.0, 0.0, 0.0],
                            [0.0, 0.0, 1.0, 0.0],
                            [0.0, 0.0, 0.0, 1.0],
                        ],
                    );
                }
                let joint_buf = self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("pbr_joints"),
                        contents: bytemuck::cast_slice(&joint_data),
                        usage: BufferUsages::UNIFORM,
                    });
                let bg = self.device.create_bind_group(&BindGroupDescriptor {
                    label: Some("pbr_bg"),
                    layout: &self.pbr_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: uni_buf.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(&tex.view),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::Sampler(&tex.sampler),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: mat_buf.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: joint_buf.as_entire_binding(),
                        },
                    ],
                });
                let shadow_bg = self.device.create_bind_group(&BindGroupDescriptor {
                    label: Some("pbr_shadow_bg"),
                    layout: &self.pbr_shadow_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: uni_buf.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: self.raster_shadow_view_proj_buffer.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: joint_buf.as_entire_binding(),
                        },
                    ],
                });
                bind_groups.push((bg, inst.mesh.clone()));
                shadow_bind_groups.push((shadow_bg, inst.mesh.clone()));
            }
            {
                let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("pbr_shadow"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &self.raster_shadow_view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                rpass.set_pipeline(&self.pbr_shadow_pipeline);
                for (bg, mesh) in shadow_bind_groups.iter() {
                    rpass.set_bind_group(0, bg, &[]);
                    rpass.set_vertex_buffer(0, mesh.0.vbuf.slice(..));
                    rpass.set_index_buffer(mesh.0.ibuf.slice(..), IndexFormat::Uint32);
                    rpass.draw_indexed(0..mesh.0.index_count, 0, 0..1);
                }
            }
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pbr_gbuffer"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.gbuf_albedo_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gbuf_normal_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gbuf_material_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.depth_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &self.gbuf_lightmap_uv_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_stencil_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            rpass.set_pipeline(&self.pbr_pipeline);
            for (bg, mesh) in bind_groups.iter() {
                rpass.set_bind_group(0, bg, &[]);
                rpass.set_vertex_buffer(0, mesh.0.vbuf.slice(..));
                rpass.set_index_buffer(mesh.0.ibuf.slice(..), IndexFormat::Uint32);
                rpass.draw_indexed(0..mesh.0.index_count, 0, 0..1);
            }
        }
        if !self.is_2d && dispatch_sdfgi && self.gi_cache.dirty {
            {
                let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("sdfgi_prepass"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.sdfgi_pipeline);
                cpass.set_bind_group(0, &self.sdfgi_bind_group, &[]);
                cpass.dispatch_workgroups(
                    (GI_SDF_RES + 7) / 8,
                    (GI_SDF_RES + 7) / 8,
                    (GI_SDF_RES + 3) / 4,
                );
            }

            for (index, mip_bind_group) in self.sdfgi_mip_bind_groups.iter().enumerate() {
                let level = (index as u32) + 1;
                let size = GI_SDF_RES >> level;
                let mut mip_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("sdfgi_mips"),
                    timestamp_writes: None,
                });
                mip_pass.set_pipeline(&self.sdfgi_mip_pipeline);
                mip_pass.set_bind_group(0, mip_bind_group, &[]);
                mip_pass.dispatch_workgroups((size + 7) / 8, (size + 7) / 8, (size + 3) / 4);
            }
            {
                let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("sdfgi_inject"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.sdfgi_inject_pipeline);
                cpass.set_bind_group(0, &self.sdfgi_inject_bind_group, &[]);
                cpass.dispatch_workgroups(
                    (GI_SDF_RES + 7) / 8,
                    (GI_SDF_RES + 7) / 8,
                    (GI_SDF_RES + 3) / 4,
                );
            }
            self.gi_cache.last_baked_scene_hash = self.gi_cache.static_scene_hash;
            self.gi_cache.has_sdfgi_volume = true;
            self.gi_cache.dirty = false;
        }
        if !self.is_2d {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("atmosphere_luts"),
                timestamp_writes: profiler_query_set.map(|query_set| ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(PROF_CLOUDS_BEGIN),
                    end_of_pass_write_index: Some(PROF_CLOUDS_END),
                }),
            });
            profiled_clouds = profiler_query_set.is_some();
            cpass.set_pipeline(&self.transmittance_lut_pipeline);
            cpass.set_bind_group(0, &self.transmittance_lut_bind_group, &[]);
            cpass.dispatch_workgroups(
                (super::setup::TRANSMITTANCE_LUT_WIDTH + 7) / 8,
                (super::setup::TRANSMITTANCE_LUT_HEIGHT + 7) / 8,
                1,
            );
            cpass.set_pipeline(&self.multi_scattering_lut_pipeline);
            cpass.set_bind_group(0, &self.multi_scattering_lut_bind_group, &[]);
            cpass.dispatch_workgroups(
                (super::setup::MULTI_SCATTERING_LUT_WIDTH + 7) / 8,
                (super::setup::MULTI_SCATTERING_LUT_HEIGHT + 7) / 8,
                1,
            );
            cpass.set_pipeline(&self.sky_view_lut_pipeline);
            cpass.set_bind_group(0, &self.sky_view_lut_bind_group, &[]);
            cpass.dispatch_workgroups(
                (super::setup::SKY_VIEW_LUT_WIDTH + 7) / 8,
                (super::setup::SKY_VIEW_LUT_HEIGHT + 7) / 8,
                1,
            );
            cpass.set_pipeline(&self.aerial_perspective_lut_pipeline);
            cpass.set_bind_group(0, &self.aerial_perspective_lut_bind_group, &[]);
            cpass.dispatch_workgroups(
                (super::setup::AERIAL_PERSPECTIVE_LUT_WIDTH + 7) / 8,
                (super::setup::AERIAL_PERSPECTIVE_LUT_HEIGHT + 7) / 8,
                (super::setup::AERIAL_PERSPECTIVE_LUT_DEPTH + 3) / 4,
            );
        }
        if effective_cloud_count > 0 && effective_cloud_shadow_mode == 0 {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("cloud_directional_shadow"),
                timestamp_writes: None,
            });
            let cloud_shadow_pipeline = if cinematic_pipeline_ready {
                self.cinematic_cloud_shadow_pipeline
                    .as_ref()
                    .unwrap_or(&self.cloud_shadow_pipeline)
            } else {
                &self.cloud_shadow_pipeline
            };
            cpass.set_pipeline(cloud_shadow_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.dispatch_workgroups((512 + 7) / 8, (512 + 7) / 8, 1);
        }
        if effective_cloud_count > 0 && effective_cloud_shadow_mode == 0 {
            encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &self.cloud_shadow_texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                ImageCopyTexture {
                    texture: &self.cloud_shadow_history_texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width: 512,
                    height: 512,
                    depth_or_array_layers: 1,
                },
            );
        }
        if effective_renderer_mode.uses_decomposed_rt_effects() {
            let rt_params = HybridRtEffectParams {
                inv_view_proj: params.inv_view_proj,
                view_proj: current_vp.to_cols_array_2d(),
                camera_pos: [
                    params.camera_pos[0],
                    params.camera_pos[1],
                    params.camera_pos[2],
                    0.0,
                ],
                dir_light_dir: [
                    params.dir_light_dir[0],
                    params.dir_light_dir[1],
                    params.dir_light_dir[2],
                    params.dir_light_intensity,
                ],
                dir_light_color: [
                    params.dir_light_color[0],
                    params.dir_light_color[1],
                    params.dir_light_color[2],
                    0.0,
                ],
                enabled: 1,
                mode: 1,
                gi_mode: effective_gi_mode,
                rtao_sample_count: if self.adaptive_quality < 0.9 { 6 } else { 8 },
                rtao_radius_bits: f32::to_bits(params.gi_max_distance.min(2.0).max(0.05)),
                _pad: [0; 3],
            };
            self.queue.write_buffer(
                &self.hybrid_rt_params_buffer,
                0,
                bytemuck::bytes_of(&rt_params),
            );
            let comp_params = HybridCompositeParams {
                temporal_blend: 0.10,
                rt_gi_enabled: u32::from(feature_status.hybrid_rtgi_active),
                rt_reflections_enabled: u32::from(feature_status.hybrid_rt_reflections_active),
                ssr_enabled: u32::from(feature_status.ssr_reflections_active),
                rt_shadows_enabled: u32::from(feature_status.hybrid_rt_shadows_active),
                rt_transparency_enabled: u32::from(matches!(
                    policy.transparency,
                    crate::rendering::renderer::TransparencyMethod::Raytraced
                )),
                atmosphere_enabled: 0,
                clouds_enabled: if effective_cloud_count > 0 { 1 } else { 0 },
                _pad: 0,
            };
            self.queue.write_buffer(
                &self.hybrid_composite_params_buffer,
                0,
                bytemuck::bytes_of(&comp_params),
            );
            let (x, y) = ((self.width + 7) / 8, (self.height + 7) / 8);
            let ao_method = Self::ambient_occlusion_method_constant(policy.ambient_occlusion);
            let rtao_dispatchable = !self.is_2d
                && ao_method == AO_METHOD_RTAO
                && self.rtao_pipeline.is_some()
                && hardware.rt_ao;
            let screen_ao_method = if ao_method == AO_METHOD_RTAO && !rtao_dispatchable {
                if self.ambient_occlusion_pipeline.is_some() {
                    AO_METHOD_GTAO
                } else {
                    AO_METHOD_OFF
                }
            } else {
                ao_method
            };
            let ao_dispatchable =
                !self.is_2d && matches!(screen_ao_method, AO_METHOD_SSAO | AO_METHOD_GTAO);
            if ao_dispatchable && self.ambient_occlusion_pipeline.is_none() {
                render_log("AO pipeline unavailable; compositor will sample neutral AO fallback");
            }
            feature_status.active_ambient_occlusion_method =
                crate::rendering::renderer::AmbientOcclusionMethod::Off;
            feature_status.ambient_occlusion_fallback = ao_method != screen_ao_method
                || (ao_dispatchable && self.ambient_occlusion_pipeline.is_none())
                || (ao_method == AO_METHOD_RTAO && !rtao_dispatchable);
            if rtao_dispatchable {
                if let Some(pipeline) = &self.rtao_pipeline {
                    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("rt_ao"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(pipeline);
                    cpass.set_bind_group(0, &self.rtao_bind_group, &[]);
                    cpass.dispatch_workgroups(x, y, 1);
                    feature_status.ambient_occlusion_active = true;
                    feature_status.ambient_occlusion_fallback = false;
                    feature_status.active_ambient_occlusion_method =
                        crate::rendering::renderer::AmbientOcclusionMethod::RTAO;
                }
            } else if ao_dispatchable {
                let ao_params = AmbientOcclusionParams {
                    inv_view_proj: params.inv_view_proj,
                    camera_pos: [
                        params.camera_pos[0],
                        params.camera_pos[1],
                        params.camera_pos[2],
                        0.0,
                    ],
                    tex_size: [self.width as f32, self.height as f32],
                    radius: 2.0,
                    intensity: 1.4,
                    method: screen_ao_method,
                    frame_number: self.frame_number.max(0) as u32,
                    temporal_enabled: u32::from(self.frame_number > 0),
                    _pad: 0,
                };
                self.queue.write_buffer(
                    &self.ambient_occlusion_params_buffer,
                    0,
                    bytemuck::bytes_of(&ao_params),
                );
                if let Some(pipeline) = &self.ambient_occlusion_pipeline {
                    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("ambient_occlusion"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(pipeline);
                    cpass.set_bind_group(0, &self.ambient_occlusion_bind_group, &[]);
                    cpass.dispatch_workgroups(x, y, 1);
                    feature_status.ambient_occlusion_active = true;
                    feature_status.ambient_occlusion_fallback = false;
                    feature_status.active_ambient_occlusion_method = match screen_ao_method {
                        AO_METHOD_GTAO => crate::rendering::renderer::AmbientOcclusionMethod::GTAO,
                        AO_METHOD_SSAO => crate::rendering::renderer::AmbientOcclusionMethod::SSAO,
                        _ => crate::rendering::renderer::AmbientOcclusionMethod::Off,
                    };
                }
            }
            if feature_status.hybrid_rt_shadows_active {
                if let Some(pipeline) = &self.hybrid_rt_shadow_pipeline {
                    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("hybrid_rt_shadows"),
                        timestamp_writes: profiler_query_set.map(|query_set| {
                            ComputePassTimestampWrites {
                                query_set,
                                beginning_of_pass_write_index: Some(PROF_RT_SHADOW_BEGIN),
                                end_of_pass_write_index: Some(PROF_RT_SHADOW_END),
                            }
                        }),
                    });
                    profiled_rt_shadow = profiler_query_set.is_some();
                    cpass.set_pipeline(pipeline);
                    cpass.set_bind_group(0, &self.hybrid_rt_shadow_bind_group, &[]);
                    cpass.dispatch_workgroups(x, y, 1);
                }
            }
            // Reflection order is intentional: resolve SSR first, run RT only as the fallback layer,
            // then let hybrid compose blend probe/SSR/RT using their per-pixel confidence.
            if feature_status.ssr_reflections_active {
                let ssr_params = SsrParams {
                    inv_view_proj: params.inv_view_proj,
                    view_proj: current_vp.to_cols_array_2d(),
                    prev_view_proj: self.prev_view_proj,
                    camera_pos: [params.camera_pos[0], params.camera_pos[1], params.camera_pos[2], 0.0],
                    tex_size: [self.width as f32, self.height as f32],
                    max_distance: params.reflection_max_distance.max(0.01),
                    thickness: ssr_quality.thickness,
                    temporal_blend: ssr_quality.temporal_blend,
                    roughness_cutoff: ssr_quality.roughness_cutoff,
                    confidence_threshold: ssr_quality.confidence_threshold,
                    stride: ssr_quality.stride,
                    max_steps: ssr_quality.max_steps,
                    frame_number: self.frame_number.max(0) as u32,
                    enabled: 1,
                    _pad: 0,
                };
                self.queue.write_buffer(&self.ssr_params_buffer, 0, bytemuck::bytes_of(&ssr_params));
                if let Some(pipeline) = &self.ssr_pipeline {
                    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("ssr_reflections"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(pipeline);
                    cpass.set_bind_group(0, &self.ssr_bind_group, &[]);
                    cpass.dispatch_workgroups(x, y, 1);
                }
            }
            if feature_status.hybrid_rt_reflections_active {
                if let Some(pipeline) = &self.hybrid_rt_reflection_pipeline {
                    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("hybrid_rt_reflections"),
                        timestamp_writes: profiler_query_set.map(|query_set| {
                            ComputePassTimestampWrites {
                                query_set,
                                beginning_of_pass_write_index: Some(PROF_RT_REFLECTION_BEGIN),
                                end_of_pass_write_index: Some(PROF_RT_REFLECTION_END),
                            }
                        }),
                    });
                    profiled_rt_reflection = profiler_query_set.is_some();
                    cpass.set_pipeline(pipeline);
                    cpass.set_bind_group(0, &self.hybrid_rt_reflection_bind_group, &[]);
                    cpass.dispatch_workgroups(x, y, 1);
                }
            }
            if feature_status.hybrid_rtgi_active {
                if let Some(pipeline) = &self.hybrid_rt_gi_pipeline {
                    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("hybrid_rt_gi"),
                        timestamp_writes: profiler_query_set.map(|query_set| {
                            ComputePassTimestampWrites {
                                query_set,
                                beginning_of_pass_write_index: Some(PROF_RT_GI_BEGIN),
                                end_of_pass_write_index: Some(PROF_RT_GI_END),
                            }
                        }),
                    });
                    profiled_rt_gi = profiler_query_set.is_some();
                    cpass.set_pipeline(pipeline);
                    cpass.set_bind_group(0, &self.hybrid_rt_gi_bind_group, &[]);
                    cpass.dispatch_workgroups(x, y, 1);
                }
            }
            if let Some(pipeline) = &self.gi_resolve_pipeline {
                let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor { label: Some("gi_resolve"), timestamp_writes: None });
                cpass.set_pipeline(pipeline);
                cpass.set_bind_group(0, &self.gi_resolve_bind_group, &[]);
                cpass.dispatch_workgroups(x, y, 1);
                feature_status.active_gi_method = match gi_resolve_method {
                    GI_RESOLVE_METHOD_BAKED_LIGHTMAP => crate::rendering::renderer::GiMethod::BakedLightmap,
                    GI_RESOLVE_METHOD_LIGHT_PROBES => crate::rendering::renderer::GiMethod::LightProbes,
                    GI_RESOLVE_METHOD_SDFGI => crate::rendering::renderer::GiMethod::SDFGI,
                    GI_RESOLVE_METHOD_RTGI_ONE_BOUNCE => crate::rendering::renderer::GiMethod::RTGIOneBounce,
                    _ => crate::rendering::renderer::GiMethod::Off,
                };
                feature_status.hybrid_rtgi_active = gi_resolve_method == GI_RESOLVE_METHOD_RTGI_ONE_BOUNCE;
            } else {
                feature_status.active_gi_method = crate::rendering::renderer::GiMethod::Off;
                feature_status.hybrid_rtgi_active = false;
                if feature_status.gi_fallback_reason == crate::rendering::renderer::RendererFallbackReason::None {
                    feature_status.gi_fallback_reason = pipeline_reason(self.hybrid_rt_gi_pipeline.is_some());
                }
            }
            {
                let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("hybrid_transparency_composite"),
                    timestamp_writes: None,
                });
                if matches!(
                    policy.transparency,
                    crate::rendering::renderer::TransparencyMethod::Raytraced
                ) {
                    if let Some(pipeline) = &self.hybrid_rt_transparency_pipeline {
                        cpass.set_pipeline(pipeline);
                        cpass.set_bind_group(0, &self.hybrid_rt_transparency_bind_group, &[]);
                        cpass.dispatch_workgroups(x, y, 1);
                    }
                }
                if let Some(pipeline) = &self.hybrid_compose_pipeline {
                    cpass.set_pipeline(pipeline);
                    cpass.set_bind_group(0, &self.compute_bind_group, &[]);
                    cpass.dispatch_workgroups(x, y, 1);
                }
                if let Some(pipeline) = &self.hybrid_composite_pipeline {
                    cpass.set_pipeline(pipeline);
                    cpass.set_bind_group(0, &self.hybrid_composite_bind_group, &[]);
                    cpass.dispatch_workgroups(x, y, 1);
                    final_compositor_wrote_screen = true;
                }
            }
            // History ownership: SSR owns only ssr_reflection_history; RT reflections own only
            // hybrid_rt_reflection_history. No pass copies into another reflection history.
            if feature_status.ssr_reflections_active {
                encoder.copy_texture_to_texture(
                    ImageCopyTexture { texture: &self.ssr_color_texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
                    ImageCopyTexture { texture: &self.ssr_history_texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
                    Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
                );
            }
            if feature_status.hybrid_rt_reflections_active {
                encoder.copy_texture_to_texture(
                    ImageCopyTexture {
                        texture: &self.hybrid_rt_reflection_texture,
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    ImageCopyTexture {
                        texture: &self.hybrid_rt_reflection_history_texture,
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    Extent3d {
                        width: self.width,
                        height: self.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            if feature_status.ambient_occlusion_active {
                encoder.copy_texture_to_texture(
                    ImageCopyTexture {
                        texture: &self.ambient_occlusion_texture,
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    ImageCopyTexture {
                        texture: &self.ambient_occlusion_history_texture,
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    Extent3d {
                        width: self.width,
                        height: self.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            if !matches!(
                feature_status.active_gi_method,
                crate::rendering::renderer::GiMethod::Off
            ) {
                encoder.copy_texture_to_texture(
                    ImageCopyTexture {
                        texture: &self.gi_buffer_texture,
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    ImageCopyTexture {
                        texture: &self.gi_history_texture,
                        mip_level: 0,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    Extent3d {
                        width: self.width,
                        height: self.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
        } else {
            {
                let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("raytrace"),
                    timestamp_writes: None,
                });
                let main_compute_pipeline = match self.active_compute_pipeline_kind {
                    MainComputePipelineKind::CinematicPathTrace if cinematic_pipeline_ready => self
                        .cinematic_compute_pipeline
                        .as_ref()
                        .unwrap_or(&self.compute_pipeline),
                    MainComputePipelineKind::HybridCompose => self
                        .hybrid_compose_pipeline
                        .as_ref()
                        .unwrap_or(&self.compute_pipeline),
                    MainComputePipelineKind::Bootstrap
                    | MainComputePipelineKind::CinematicPathTrace => &self.compute_pipeline,
                };
                cpass.set_pipeline(main_compute_pipeline);
                cpass.set_bind_group(0, &self.compute_bind_group, &[]);
                let (x, y) = if self.is_2d {
                    ((self.width + 15) / 16, (self.height + 15) / 16)
                } else {
                    ((self.width + 7) / 8, (self.height + 7) / 8)
                };
                cpass.dispatch_workgroups(x, y, 1);
            }
        }
        if !uses_rt_primary && !final_compositor_wrote_screen {
            // The lightweight bootstrap compute path writes into `color_texture`, while
            // the existing postprocess blit samples `screen_texture`. Mirror bootstrap
            // output before postprocessing so raster fallback modes do not present a
            // stale black screen. Decomposed hybrid effects skip this only after their
            // final compositor writes directly into `screen_texture`, and path-traced modes keep using
            // rt_denoise to populate `screen_texture`.
            encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &self.color_texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                ImageCopyTexture {
                    texture: &self.screen_texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        encoder.copy_texture_to_texture(
            ImageCopyTexture {
                texture: &self.cloud_radiance_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyTexture {
                texture: &self.cloud_radiance_history_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        encoder.copy_texture_to_texture(
            ImageCopyTexture {
                texture: &self.cloud_transmittance_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyTexture {
                texture: &self.cloud_transmittance_history_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        if !self.is_2d && effective_renderer_mode.uses_rt_primary_visibility() {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("rt_denoise"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.rt_denoise_pipeline);
            cpass.set_bind_group(0, &self.rt_denoise_bind_group, &[]);
            cpass.dispatch_workgroups((self.width + 15) / 16, (self.height + 15) / 16, 1);
        }
        if !self.is_2d && effective_renderer_mode.uses_rt_primary_visibility() {
            // Propagate the denoised frame to the color texture so subsequent
            // passes operate on filtered pixels rather than the raw noisy
            // output.
            encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &self.screen_texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                ImageCopyTexture {
                    texture: &self.color_texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        if !self.is_2d && uses_path_traced_primary {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("denoise"),
                timestamp_writes: profiler_query_set.map(|query_set| ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(PROF_DENOISE_BEGIN),
                    end_of_pass_write_index: Some(PROF_DENOISE_END),
                }),
            });
            profiled_denoise = profiler_query_set.is_some();
            cpass.set_pipeline(&self.denoise_pipeline);
            cpass.set_bind_group(0, &self.denoise_bind_group, &[]);
            cpass.dispatch_workgroups((self.width + 7) / 8, (self.height + 7) / 8, 1);
        }
        // Preserve the denoised image and G-buffer data before subsequent
        // post-processing passes overwrite the alpha channel or otherwise
        // modify these textures. The history copies occur here so the temporal
        // accumulator sees the correct object IDs and depth information on the
        // next frame.
        encoder.copy_texture_to_texture(
            ImageCopyTexture {
                texture: &self.gi_buffer_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyTexture {
                texture: &self.gi_history_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        encoder.copy_texture_to_texture(
            ImageCopyTexture {
                texture: &self.screen_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyTexture {
                texture: &self.screen_history_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        encoder.copy_texture_to_texture(
            ImageCopyTexture {
                texture: &self.depth_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyTexture {
                texture: &self.depth_history_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        encoder.copy_texture_to_texture(
            ImageCopyTexture {
                texture: &self.normal_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyTexture {
                texture: &self.normal_history_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        {
            let mut clear = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("clear_screen"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.screen_view,
                    resolve_target: None,
                    ops: Operations {
                        load: if self.is_2d {
                            LoadOp::Clear(Color {
                                r: params.skycolor[0] as f64,
                                g: params.skycolor[1] as f64,
                                b: params.skycolor[2] as f64,
                                a: 1.0,
                            })
                        } else {
                            LoadOp::Load
                        },
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_stencil_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        {
            let mut clear_occ = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("clear_occluder"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.occluder_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        let light_data = LightUniform {
            dir: [-params.dir_light_dir[0], -params.dir_light_dir[1]],
            _pad: [0.0; 2],
            color: [
                params.dir_light_color[0],
                params.dir_light_color[1],
                params.dir_light_color[2],
            ],
            intensity: params.dir_light_intensity,
        };
        if self.prev_light_data.map_or(true, |p| p != light_data) {
            self.queue
                .write_buffer(&self.light_buffer, 0, bytemuck::bytes_of(&light_data));
            self.prev_light_data = Some(light_data);
            self.gi_cache.mark_dirty();
        }
        let sprite_stride = (6 * std::mem::size_of::<[f32; 5]>()) as u64;
        let mut vertex_data: Vec<[f32; 5]> = Vec::with_capacity(sprites.len() * 6);
        let mut bind_groups = Vec::with_capacity(sprites.len());
        for sprite in sprites {
            vertex_data.extend_from_slice(&sprite.vertices);
            let bg = self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("sprite_bg"),
                layout: &self.sprite_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: self.sprite_view_proj_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.linear_sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&sprite.texture),
                    },
                ],
            });
            bind_groups.push(bg);
        }
        let needed = (vertex_data.len() * std::mem::size_of::<[f32; 5]>()) as u64;
        if self.sprite_vertex_buffer.size() < needed {
            self.sprite_vertex_buffer = self.device.create_buffer(&BufferDescriptor {
                label: Some("sprite_vbo"),
                size: needed,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        let vertex_changed = vertex_data != self.sprite_vertices_cache;
        if vertex_changed && !vertex_data.is_empty() {
            self.queue.write_buffer(
                &self.sprite_vertex_buffer,
                0,
                bytemuck::cast_slice(&vertex_data),
            );
        }
        self.sprite_vertices_cache = vertex_data.clone();
        if !vertex_data.is_empty() {
            {
                let mut op = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("occluder"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &self.occluder_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                op.set_pipeline(&self.occluder_pipeline);
                for (i, bg) in bind_groups.iter().enumerate() {
                    let offset = i as u64 * sprite_stride;
                    op.set_bind_group(0, bg, &[]);
                    op.set_vertex_buffer(
                        0,
                        self.sprite_vertex_buffer
                            .slice(offset..offset + sprite_stride),
                    );
                    op.draw(0..6, 0..1);
                }
            }
            {
                let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("sprite"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &self.screen_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &self.depth_stencil_view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                rpass.set_pipeline(&self.sprite_pipeline);
                for (i, bg) in bind_groups.iter().enumerate() {
                    let offset = i as u64 * sprite_stride;
                    rpass.set_bind_group(0, bg, &[]);
                    rpass.set_vertex_buffer(
                        0,
                        self.sprite_vertex_buffer
                            .slice(offset..offset + sprite_stride),
                    );
                    rpass.draw(0..6, 0..1);
                }
            }
        }
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("blit"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.render_bind_group, &[]);
            rpass.draw(0..4, 0..1);
        }
        encoder.copy_texture_to_texture(
            ImageCopyTexture {
                texture: &frame.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyTexture {
                texture: &self.blur_src_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        if !self.pending_blur_regions.is_empty() {
            for (i, &(x, y, w, h)) in self.pending_blur_regions.iter().enumerate() {
                let params = BlurParams {
                    resolution: [self.width as f32, self.height as f32],
                    _pad0: [0.0; 2],
                    region: [x as f32, y as f32, w as f32, h as f32],
                    feather: self.blur_feather,
                    _pad1: [0.0; 7],
                };
                let offset = (i as u64) * 256;
                self.queue.write_buffer(
                    &self.blur_params_buffer,
                    offset,
                    bytemuck::bytes_of(&params),
                );
            }
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("ui_blur"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            rpass.set_pipeline(&self.blur_pipeline);
            for (i, &(x, y, w, h)) in self.pending_blur_regions.iter().enumerate() {
                let max_w = self.width.saturating_sub(x);
                let max_h = self.height.saturating_sub(y);
                let clamp_w = w.min(max_w);
                let clamp_h = h.min(max_h);
                if clamp_w > 0 && clamp_h > 0 {
                    let offset = (i as u32) * 256;
                    rpass.set_bind_group(0, &self.blur_bind_group, &[offset]);
                    rpass.set_scissor_rect(x, y, clamp_w, clamp_h);
                    rpass.draw(0..4, 0..1);
                }
            }
        }
        self.pending_blur_regions.clear();

        #[cfg(all(feature = "wgpu", feature = "use_epi"))]
        {
            if let Some((erender, prims, delta)) = egui {
                erender.paint_jobs(&self.device, &self.queue, &mut encoder, &view, delta, prims);
            }
        }
        #[cfg(not(all(feature = "wgpu", feature = "use_epi")))]
        {
            let _ = egui;
        }

        if let (Some(query_set), Some(query_buffer), Some(readback_buffer)) = (
            &self.profiler_query_set,
            &self.profiler_query_buffer,
            &self.profiler_readback_buffer,
        ) {
            for (begin, end, active) in [
                (PROF_RASTER_BEGIN, PROF_RASTER_END, profiled_raster),
                (PROF_RT_SHADOW_BEGIN, PROF_RT_SHADOW_END, profiled_rt_shadow),
                (
                    PROF_RT_REFLECTION_BEGIN,
                    PROF_RT_REFLECTION_END,
                    profiled_rt_reflection,
                ),
                (PROF_RT_GI_BEGIN, PROF_RT_GI_END, profiled_rt_gi),
                (PROF_DENOISE_BEGIN, PROF_DENOISE_END, profiled_denoise),
                (PROF_CLOUDS_BEGIN, PROF_CLOUDS_END, profiled_clouds),
            ] {
                if !active {
                    let _unused_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("profiler_inactive_pass_timestamp"),
                        timestamp_writes: Some(ComputePassTimestampWrites {
                            query_set,
                            beginning_of_pass_write_index: Some(begin),
                            end_of_pass_write_index: Some(end),
                        }),
                    });
                }
            }
            encoder.resolve_query_set(query_set, 0..12, query_buffer, 0);
            encoder.copy_buffer_to_buffer(
                query_buffer,
                0,
                readback_buffer,
                0,
                12 * std::mem::size_of::<u64>() as u64,
            );
        }

        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::Maintain::Poll);
        stats.total_frame_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
        stats.feature_status = feature_status;
        stats.profiler_timestamps_supported = self.profiler_query_set.is_some();
        stats.profiler_status = if stats.profiler_timestamps_supported {
            "gpu_timestamp_queries"
        } else {
            "timestamp_queries_unavailable"
        };
        if let Some(readback_buffer) = &self.profiler_readback_buffer {
            let slice = readback_buffer.slice(..);
            let (sender, receiver) = std::sync::mpsc::channel();
            slice.map_async(MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
            self.device.poll(wgpu::Maintain::Wait);
            if receiver.recv().ok().and_then(Result::ok).is_some() {
                let data = slice.get_mapped_range();
                let timestamps: &[u64] = bytemuck::cast_slice(&data);
                let elapsed_ms = |begin: u32, end: u32, active: bool| -> f32 {
                    if active {
                        timestamps
                            .get(begin as usize)
                            .zip(timestamps.get(end as usize))
                            .map(|(&start, &finish)| {
                                finish.saturating_sub(start) as f32 * self.profiler_timestamp_period
                                    / 1_000_000.0
                            })
                            .unwrap_or(0.0)
                    } else {
                        0.0
                    }
                };
                stats.raster_pass_ms =
                    elapsed_ms(PROF_RASTER_BEGIN, PROF_RASTER_END, profiled_raster);
                stats.rt_shadow_pass_ms =
                    elapsed_ms(PROF_RT_SHADOW_BEGIN, PROF_RT_SHADOW_END, profiled_rt_shadow);
                stats.rt_reflection_pass_ms = elapsed_ms(
                    PROF_RT_REFLECTION_BEGIN,
                    PROF_RT_REFLECTION_END,
                    profiled_rt_reflection,
                );
                stats.rt_gi_pass_ms = elapsed_ms(PROF_RT_GI_BEGIN, PROF_RT_GI_END, profiled_rt_gi);
                stats.denoise_ms =
                    elapsed_ms(PROF_DENOISE_BEGIN, PROF_DENOISE_END, profiled_denoise);
                stats.clouds_fog_atmosphere_ms =
                    elapsed_ms(PROF_CLOUDS_BEGIN, PROF_CLOUDS_END, profiled_clouds);
                drop(data);
                readback_buffer.unmap();
            } else {
                stats.profiler_status = "timestamp_readback_failed";
            }
        }
        if stats.total_frame_ms > 16.6 {
            self.slow_frame_streak += 1;
            self.fast_frame_streak = 0;
        } else if stats.total_frame_ms < 13.0 {
            self.fast_frame_streak += 1;
            self.slow_frame_streak = 0;
        } else {
            self.slow_frame_streak = 0;
            self.fast_frame_streak = 0;
        }
        if self.slow_frame_streak >= 6 {
            self.adaptive_quality = (self.adaptive_quality - 0.15).max(0.5);
            self.slow_frame_streak = 0;
        }
        if self.fast_frame_streak >= 60 {
            self.adaptive_quality = (self.adaptive_quality + 0.05).min(1.0);
            self.fast_frame_streak = 0;
        }
        stats.adaptive_quality = self.adaptive_quality;
        self.profiler_stats = stats;
        frame.present();
        self.prev_view_proj = current_vp.to_cols_array_2d();
        self.prev_taa_jitter = [jitter_x, jitter_y];
        self.prev_cam_pos = params.camera_pos;
        self.prev_cam_front = params.camera_front;
        self.prev_cam_up = params.camera_up;
        self.prev_cam_right = params.camera_right;
        self.prev_num_objects = params.num_objects;
        self.frame_number += 1;

        // keep impl open for helper methods below
    }

    pub fn profiler_stats(&self) -> crate::rendering::renderer::RendererProfilerStats {
        self.profiler_stats
    }

    pub fn draw_profiler_hud(&self, ctx: &egui::Context) {
        fn fallback_suffix(reason: crate::rendering::renderer::RendererFallbackReason) -> String {
            let label = reason.hud_label();
            if label.is_empty() { String::new() } else { format!(" [{label}]") }
        }
        let s = self.profiler_stats();
        egui::Window::new("Renderer Profiler")
            .default_pos(egui::pos2(12.0, 12.0))
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Raster: {:.2} ms (shadow maps active: {})",
                    s.raster_pass_ms, s.feature_status.raster_shadow_maps_active
                ));
                ui.label(format!(
                    "Primary visibility: requested {:?} / active {:?}{}",
                    s.feature_status.requested_primary_visibility_method,
                    s.feature_status.active_primary_visibility_method,
                    fallback_suffix(s.feature_status.primary_visibility_fallback_reason)
                ));
                ui.label(format!(
                    "Shadows: requested {:?} / active {:?}{} ({:.2} ms)",
                    s.feature_status.requested_shadow_method,
                    s.feature_status.active_shadow_method,
                    fallback_suffix(s.feature_status.shadow_fallback_reason),
                    s.rt_shadow_pass_ms
                ));
                ui.label(format!(
                    "Reflections: requested {:?} / active {:?}{} ({:.2} ms)",
                    s.feature_status.requested_reflection_method,
                    s.feature_status.active_reflection_method,
                    fallback_suffix(s.feature_status.reflection_fallback_reason),
                    s.rt_reflection_pass_ms
                ));
                ui.label(format!(
                    "AO: requested {:?} / active {:?}{} (fallback: {})",
                    s.feature_status.requested_ambient_occlusion_method,
                    s.feature_status.active_ambient_occlusion_method,
                    fallback_suffix(s.feature_status.ambient_occlusion_fallback_reason),
                    s.feature_status.ambient_occlusion_fallback
                ));
                ui.label(format!(
                    "GI: requested {:?} / active {:?}{} ({:.2} ms)",
                    s.feature_status.requested_gi_method,
                    s.feature_status.active_gi_method,
                    fallback_suffix(s.feature_status.gi_fallback_reason),
                    s.rt_gi_pass_ms
                ));
                ui.label(format!(
                    "Transparency: requested {:?} / active {:?}{}",
                    s.feature_status.requested_transparency_method,
                    s.feature_status.active_transparency_method,
                    fallback_suffix(s.feature_status.transparency_fallback_reason)
                ));
                ui.label(format!("Denoise: {:.2} ms", s.denoise_ms));
                ui.label(format!(
                    "Clouds/fog/atmosphere: {:.2} ms",
                    s.clouds_fog_atmosphere_ms
                ));
                ui.separator();
                ui.label(format!("Total frame: {:.2} ms", s.total_frame_ms));
                ui.label(format!("Profiler status: {}", s.profiler_status));
                ui.label(format!(
                    "Adaptive RT quality: {:.0}%",
                    s.adaptive_quality * 100.0
                ));
                ui.label(format!(
                    "Path-traced primary active: {}",
                    s.feature_status.pathtrace_primary_active
                ));
            });
    }

    pub fn capture_screen(&self) {}
    pub fn blur_regions(&mut self, regions: &[(i32, i32, i32, i32)], feather: f32) {
        self.pending_blur_regions = regions
            .iter()
            .filter(|&&(x, y, w, h)| w > 0 && h > 0)
            .map(|&(x, y, w, h)| (x.max(0) as u32, y.max(0) as u32, w as u32, h as u32))
            .take(MAX_BLUR_REGIONS)
            .collect();
        self.blur_feather = feather;
    }
    pub fn reset_frame(&mut self) {
        self.frame_number = 0;
    }
    pub fn screen_dimensions(&self) -> (i32, i32) {
        (self.surface_width as i32, self.surface_height as i32)
    }

    pub fn set_render_scale(&mut self, scale: f32) {
        self.render_scale = scale.clamp(0.1, 1.0);
        self.resize(self.surface_width as i32, self.surface_height as i32);
    }

    pub fn enable_fsr(&mut self, sharpness: f32) {
        self.sharpness = sharpness;
    }

    pub fn disable_fsr(&mut self) {
        self.sharpness = 0.0;
    }

    pub fn set_post_fx_uniforms(&mut self, fx: PostFxUniforms) {
        if fx.temporal_blend != self.post_fx_uniforms.temporal_blend
            || fx.history_clamp_k != self.post_fx_uniforms.history_clamp_k
            || fx.gi_temporal_blend != self.post_fx_uniforms.gi_temporal_blend
            || fx.shadow_history_weight != self.post_fx_uniforms.shadow_history_weight
            || fx.reflection_history_weight != self.post_fx_uniforms.reflection_history_weight
            || fx.cloud_history_weight != self.post_fx_uniforms.cloud_history_weight
            || fx.denoise_mode != self.post_fx_uniforms.denoise_mode
        {
            self.reset_frame();
        }
        self.post_fx_uniforms = fx;
        self.prev_post_fx_uniforms = None;
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn surface_format(&self) -> TextureFormat {
        self.config.format
    }

    pub fn white_texture_handle(&self) -> crate::gpu::TextureHandle {
        self.white_texture.clone()
    }

    /// Returns the current frame number used for temporal effects.
    pub fn frame_number(&self) -> i32 {
        self.frame_number
    }
}
