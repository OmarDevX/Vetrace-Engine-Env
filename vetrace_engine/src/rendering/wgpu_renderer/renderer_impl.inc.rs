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

const MAX_BLUR_REGIONS: usize = 16;

impl WgpuRenderer {
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
        let (device, queue, surface, config) =
            pollster::block_on(init_wgpu(window, width as u32, height as u32));
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
        // Create placeholder buffers large enough to satisfy the minimum
        // binding size required by the shaders. Even with an empty scene the
        // renderer expects space for at least 64 objects and materials.
        const MIN_SCENE_CAPACITY: u64 = 64;
        let object_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("objects"),
            size: std::mem::size_of::<GpuObject>() as u64 * MIN_SCENE_CAPACITY,
            usage: BufferUsages::STORAGE,
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

        let texture_array_limit = Self::texture_array_limit(&device);
        let mut shader_compiler = RaytraceShaderCompiler {
            device: device.clone(),
            base_shader_template: include_str!(
                "../../../assets/shaders/wgpu/hybrid/raytrace.comp.wgsl"
            )
            .to_string(),
            material_registry: std::collections::HashMap::new(),
        };
        let compute_shader = shader_compiler
            .compile_shader(&[])
            .expect("failed to compile base shader");
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
                ],
            });

        let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("compute_pl"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });
        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
            compilation_options: Default::default(),
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
                buffers: &[pbr_vertex_layout],
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

        Self {
            surface,
            device,
            queue,
            config,
            surface_width,
            surface_height,
            width: render_width,
            height: render_height,
            render_scale: 1.0,
            sharpness: 0.0,
            is_2d: is_2d,
            object_buffer,
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
            blur_src_texture,
            blur_src_view,
            white_texture,
            material_textures,
            gi_params_buffer,
            sdfgi_bind_group_layout,
            sdfgi_bind_group,
            sdfgi_pipeline,
            sdfgi_inject_bind_group_layout,
            sdfgi_inject_bind_group,
            sdfgi_inject_pipeline,
            sdfgi_mip_bind_group_layout,
            sdfgi_mip_pipeline,
            sampler,
            linear_sampler,
            shader_compiler,
            compute_bind_group_layout,
            compute_bind_group,
            denoise_bind_group_layout,
            denoise_bind_group,
            compute_pipeline,
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
            pbr_pipeline,
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
            prev_blit_params: None,
            prev_post_fx_uniforms: None,
            prev_sprite_view_proj: None,
            prev_light_data: None,
            sprite_vertices_cache: Vec::new(),
            prev_material_names: Vec::new(),
            prev_shader_defs: Vec::new(),
            prev_triangles: vec![GpuTriangle::zeroed()],
            prev_bvh_nodes: Vec::new(),
            prev_tri_bvh_nodes: Vec::new(),
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
        self.gi_sdf_texture = gi_sdf_texture;
        self.gi_sdf_view = gi_sdf_view;
        self.gi_sdf_storage_view = gi_sdf_storage_view;
        self.gi_radiance_texture = gi_radiance_texture;
        self.gi_radiance_view = gi_radiance_view;
        self.gi_radiance_storage_view = gi_radiance_storage_view;
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
        self.depth_history_texture = depth_history_texture;
        self.depth_history_view = depth_history_view;
        self.normal_history_texture = normal_history_texture;
        self.normal_history_view = normal_history_view;
        self.occluder_texture = occluder_texture;
        self.occluder_view = occluder_view;
        self.blur_src_texture = blur_src_texture;
        self.blur_src_view = blur_src_view;
        self.sampler = sampler;
        self.linear_sampler = linear_sampler;
        self.prev_view_proj = Mat4::IDENTITY.to_cols_array_2d();
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
        let mut obj_data = vec![GpuObject::default(); MIN_SCENE_CAPACITY.max(objects.len())];
        obj_data[..objects.len()].copy_from_slice(objects);
        self.object_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("objects"),
            contents: bytemuck::cast_slice(&obj_data),
            usage: BufferUsages::STORAGE,
        });
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
        let mut lights: Vec<u32> = Vec::new();
        for (i, obj) in objects.iter().enumerate() {
            let mi = obj.material_index as usize;
            if mi < materials.len() && materials[mi].emissive_strength > 0.0 {
                lights.push(i as u32);
            }
        }
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
            let compute_module = self
                .shader_compiler
                .compile_shader(material_names)
                .expect("failed to compile shader");
            let compute_pipeline_layout =
                self.device
                    .create_pipeline_layout(&PipelineLayoutDescriptor {
                        label: Some("compute_pl"),
                        bind_group_layouts: &[&self.compute_bind_group_layout],
                        push_constant_ranges: &[],
                    });
            self.compute_pipeline =
                self.device
                    .create_compute_pipeline(&ComputePipelineDescriptor {
                        label: Some("compute_pipeline"),
                        layout: Some(&compute_pipeline_layout),
                        module: &compute_module,
                        entry_point: "main",
                        compilation_options: Default::default(),
                    });
            self.prev_material_names = material_names.to_vec();
            self.prev_shader_defs = shaders.to_vec();
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

    pub fn render(
        &mut self,
        params: &RenderParams,
        sprites: &[SpriteRenderData],
        pbr_data: &[PbrRenderData],
        egui: Option<(
            &mut crate::rendering::egui_wgpu::EguiRenderer,
            &[egui::ClippedPrimitive],
            &egui::TexturesDelta,
        )>,
    ) {
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
            max_bounces: params.max_bounces,
            light_samples: params.light_samples,
            dir_shadow_samples: params.dir_shadow_samples,
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
            _pad_atmos: [0; 2],
            atmos: {
                let mut arr = [GpuAtmosphere::default(); MAX_ATMOSPHERES];
                let count = params.atmos.len().min(MAX_ATMOSPHERES);
                arr[..count].copy_from_slice(&params.atmos[..count]);
                arr
            },
        };
        if self.prev_shader_params.map_or(true, |p| p != shader_params) {
            self.queue
                .write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&shader_params));
            self.prev_shader_params = Some(shader_params);
        }
        let gi_params = GiParams {
            quality: params.gi_quality,
            debug_mode: params.gi_debug_mode,
            mode: params.gi_mode,
            _pad: 0,
        };
        if self.prev_gi_params.map_or(true, |p| p != gi_params) {
            self.queue
                .write_buffer(&self.gi_params_buffer, 0, bytemuck::bytes_of(&gi_params));
            self.prev_gi_params = Some(gi_params);
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
        if !pbr_data.is_empty() {
            let mut bind_groups = Vec::new();
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
                bind_groups.push((bg, inst.mesh.clone()));
            }
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pbr_gbuffer"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.gbuf_albedo_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK),
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
        if params.rt.sdfgi
            && !self.is_2d
            && params.gi_quality != crate::rendering::wgpu_renderer::types::GI_QUALITY_OFF
            && params.gi_mode == crate::rendering::wgpu_renderer::types::GI_MODE_SDF
        {
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

            let mip_count = (GI_SDF_RES as f32).log2().floor() as u32 + 1;
            for level in 1..mip_count {
                let src_view = self.gi_sdf_texture.create_view(&TextureViewDescriptor {
                    label: Some("sdfgi_mip_src"),
                    format: None,
                    dimension: Some(TextureViewDimension::D3),
                    aspect: TextureAspect::All,
                    base_mip_level: level - 1,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
                });
                let dst_view = self.gi_sdf_texture.create_view(&TextureViewDescriptor {
                    label: Some("sdfgi_mip_dst"),
                    format: None,
                    dimension: Some(TextureViewDimension::D3),
                    aspect: TextureAspect::All,
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
                });
                let mip_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                    label: Some("sdfgi_mip_bg"),
                    layout: &self.sdfgi_mip_bind_group_layout,
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
                });
                let size = GI_SDF_RES >> level;
                let mut mip_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("sdfgi_mips"),
                    timestamp_writes: None,
                });
                mip_pass.set_pipeline(&self.sdfgi_mip_pipeline);
                mip_pass.set_bind_group(0, &mip_bind_group, &[]);
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
        }
        if params.rt.raytracing {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("raytrace"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            let (x, y) = if self.is_2d {
                ((self.width + 15) / 16, (self.height + 15) / 16)
            } else {
                ((self.width + 7) / 8, (self.height + 7) / 8)
            };
            cpass.dispatch_workgroups(x, y, 1);
        }
        if params.rt.raytracing && params.rt.rt_denoise && !self.is_2d {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("rt_denoise"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.rt_denoise_pipeline);
            cpass.set_bind_group(0, &self.rt_denoise_bind_group, &[]);
            cpass.dispatch_workgroups((self.width + 15) / 16, (self.height + 15) / 16, 1);
        }
        if params.rt.raytracing && !self.is_2d {
            // Propagate the (possibly denoised) frame to the color texture so
            // subsequent passes operate on filtered pixels rather than the raw
            // noisy output.
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
        if params.rt.raytracing && params.rt.denoise && !self.is_2d {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("denoise"),
                timestamp_writes: None,
            });
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

        if let Some((erender, prims, delta)) = egui {
            erender.paint_jobs(&self.device, &self.queue, &mut encoder, &view, delta, prims);
        }

        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::Maintain::Poll);
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
