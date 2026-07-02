use super::types::GI_SDF_RES;
use super::{RayQuerySupport, RayTraversalBackend};
use sdl2::video::Window;
use wgpu::SurfaceTargetUnsafe;
use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};
use wgpu::{util::DeviceExt, *};

fn boot_log(stage: &str) {
    eprintln!("[VETRACE BOOT] {}", stage);
    let _ = std::io::Write::flush(&mut std::io::stderr());
}

pub async fn init_wgpu(
    window: &Window,
    width: u32,
    height: u32,
) -> (Device, Queue, Surface<'static>, SurfaceConfiguration) {
    // Wayland's drm-syncobj extension is disabled earlier during engine
    // initialization to avoid "surface already exists" validation errors on
    // compositors that don't support or mis-handle the protocol. Users can
    // override this by explicitly setting `WGPU_DRM_SYNCOBJ` in their
    // environment before launching.

    // Force Vulkan (or another primary backend) to avoid conflicts with
    // the existing OpenGL context used by the egui renderer. Using the GL
    // backend could fail with `BadAccess` when SDL already created a GL
    // context, so we prefer the primary non-GL backends instead.
    boot_log("init_wgpu: before Instance::new");
    let instance = Instance::new(InstanceDescriptor {
        backends: Backends::PRIMARY,
        ..Default::default()
    });
    boot_log("init_wgpu: before create surface");
    let surface = unsafe {
        let target = SurfaceTargetUnsafe::from_window(window).expect("raw handles");
        instance
            .create_surface_unsafe(target)
            .expect("Failed to create surface")
    };
    let surface: Surface<'static> = unsafe { std::mem::transmute(surface) };
    boot_log("init_wgpu: after create surface");
    boot_log("init_wgpu: before request_adapter");
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find adapter");
    boot_log("init_wgpu: after request_adapter");
    let adapter_features = adapter.features();
    let adapter_info = adapter.get_info();
    let hw_ray_query_requested = std::env::var("VETRACE_HW_RAY_QUERY")
        .map(|v| v == "1")
        .unwrap_or(false);
    let ray_query_support = RayQuerySupport::resolve(
        adapter_features,
        hw_ray_query_requested,
        adapter_info.backend,
    );
    if let Some(reason) = &ray_query_support.fallback_reason {
        eprintln!("[VETRACE BOOT] hardware ray query fallback: {}", reason);
    } else {
        eprintln!(
            "[VETRACE BOOT] hardware ray query backend active: {}",
            RayTraversalBackend::HardwareRayQuery.as_str()
        );
    }
    let limits = adapter.limits();
    boot_log("init_wgpu: before request_device");
    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                required_features: {
                    let optional_features = adapter_features & Features::TIMESTAMP_QUERY;
                    Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                        | Features::TEXTURE_BINDING_ARRAY
                        | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
                        | optional_features
                        | ray_query_support.required_features()
                },
                // Request the adapter's reported limits so the renderer can
                // bind large texture arrays for materials.
                required_limits: limits,
                label: None,
            },
            None,
        )
        .await
        .expect("Failed to request device");
    boot_log("init_wgpu: after request_device");
    let caps = surface.get_capabilities(&adapter);
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(caps.formats[0]);
    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        format,
        width,
        height,
        present_mode: PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
    };
    boot_log("init_wgpu: before surface.configure");
    surface.configure(&device, &config);
    boot_log("init_wgpu: after surface.configure");
    (device, queue, surface, config)
}

pub const TRANSMITTANCE_LUT_WIDTH: u32 = 256;
pub const TRANSMITTANCE_LUT_HEIGHT: u32 = 64;
pub const SKY_VIEW_LUT_WIDTH: u32 = 256;
pub const SKY_VIEW_LUT_HEIGHT: u32 = 128;
// Multi-scattering LUT axes are view/sun cosine (x) and normalized altitude (y).
pub const MULTI_SCATTERING_LUT_WIDTH: u32 = 64;
pub const MULTI_SCATTERING_LUT_HEIGHT: u32 = 32;
pub const AERIAL_PERSPECTIVE_LUT_WIDTH: u32 = 160;
pub const AERIAL_PERSPECTIVE_LUT_HEIGHT: u32 = 90;
pub const AERIAL_PERSPECTIVE_LUT_DEPTH: u32 = 48;

pub fn create_atmosphere_lut_textures(
    device: &Device,
) -> (
    Texture,
    TextureView,
    TextureView,
    Texture,
    TextureView,
    TextureView,
    Texture,
    TextureView,
    TextureView,
    Texture,
    TextureView,
    TextureView,
) {
    let lut_usage = TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING;
    let transmittance_lut_texture = device.create_texture(&TextureDescriptor {
        label: Some("transmittance_lut"),
        size: Extent3d {
            width: TRANSMITTANCE_LUT_WIDTH,
            height: TRANSMITTANCE_LUT_HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: lut_usage,
        view_formats: &[],
    });
    let transmittance_lut_view =
        transmittance_lut_texture.create_view(&TextureViewDescriptor::default());
    let transmittance_lut_storage_view =
        transmittance_lut_texture.create_view(&TextureViewDescriptor {
            label: Some("transmittance_lut_write"),
            ..Default::default()
        });

    let sky_view_lut_texture = device.create_texture(&TextureDescriptor {
        label: Some("sky_view_lut"),
        size: Extent3d {
            width: SKY_VIEW_LUT_WIDTH,
            height: SKY_VIEW_LUT_HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: lut_usage,
        view_formats: &[],
    });
    let sky_view_lut_view = sky_view_lut_texture.create_view(&TextureViewDescriptor::default());
    let sky_view_lut_storage_view = sky_view_lut_texture.create_view(&TextureViewDescriptor {
        label: Some("sky_view_lut_write"),
        ..Default::default()
    });

    let multi_scattering_lut_texture = device.create_texture(&TextureDescriptor {
        label: Some("multi_scattering_lut"),
        size: Extent3d {
            width: MULTI_SCATTERING_LUT_WIDTH,
            height: MULTI_SCATTERING_LUT_HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: lut_usage,
        view_formats: &[],
    });
    let multi_scattering_lut_view =
        multi_scattering_lut_texture.create_view(&TextureViewDescriptor::default());
    let multi_scattering_lut_storage_view =
        multi_scattering_lut_texture.create_view(&TextureViewDescriptor {
            label: Some("multi_scattering_lut_write"),
            ..Default::default()
        });

    let aerial_perspective_lut_texture = device.create_texture(&TextureDescriptor {
        label: Some("aerial_perspective_lut"),
        size: Extent3d {
            width: AERIAL_PERSPECTIVE_LUT_WIDTH,
            height: AERIAL_PERSPECTIVE_LUT_HEIGHT,
            depth_or_array_layers: AERIAL_PERSPECTIVE_LUT_DEPTH,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D3,
        format: TextureFormat::Rgba16Float,
        usage: lut_usage,
        view_formats: &[],
    });
    let aerial_perspective_lut_view =
        aerial_perspective_lut_texture.create_view(&TextureViewDescriptor::default());
    let aerial_perspective_lut_storage_view =
        aerial_perspective_lut_texture.create_view(&TextureViewDescriptor {
            label: Some("aerial_perspective_lut_write"),
            dimension: Some(TextureViewDimension::D3),
            ..Default::default()
        });

    (
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
    )
}

pub fn create_textures(
    device: &Device,
    _format: TextureFormat,
    width: u32,
    height: u32,
) -> (
    Texture,
    TextureView, // screen
    Texture,
    TextureView, // screen history
    Texture,
    TextureView, // depth (storage)
    Texture,
    TextureView, // depth stencil
    Texture,
    TextureView, // normal
    Texture,
    TextureView, // color
    Texture,
    TextureView, // gbuf albedo
    Texture,
    TextureView, // gbuf normal
    Texture,
    TextureView, // gbuf material
    Texture,
    TextureView,
    TextureView, // gi sdf (texture, sample view, storage view)
    Texture,
    TextureView,
    TextureView, // gi radiance (texture, sample view, storage view)
    Texture,
    TextureView, // gi history
    Texture,
    TextureView, // gi noisy
    Texture,
    TextureView, // gi buffer
    Texture,
    TextureView, // motion
    Texture,
    TextureView, // variance
    Texture,
    TextureView, // lightmap
    Texture,
    TextureView, // depth history
    Texture,
    TextureView, // normal history
    Texture,
    TextureView, // occluder mask
    Sampler,
    Sampler,
) {
    let screen_texture = device.create_texture(&TextureDescriptor {
        label: Some("screen_tex"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let screen_view = screen_texture.create_view(&TextureViewDescriptor::default());
    let screen_history_texture = device.create_texture(&TextureDescriptor {
        label: Some("screen_history"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let screen_history_view = screen_history_texture.create_view(&TextureViewDescriptor::default());
    let depth_texture = device.create_texture(&TextureDescriptor {
        label: Some("depth_storage"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R32Float,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth_view = depth_texture.create_view(&TextureViewDescriptor::default());
    let depth_stencil_texture = device.create_texture(&TextureDescriptor {
        label: Some("depth_tex"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth_stencil_view = depth_stencil_texture.create_view(&TextureViewDescriptor::default());
    let normal_texture = device.create_texture(&TextureDescriptor {
        label: Some("normal_tex"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let normal_view = normal_texture.create_view(&TextureViewDescriptor::default());
    let color_texture = device.create_texture(&TextureDescriptor {
        label: Some("color_tex"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let color_view = color_texture.create_view(&TextureViewDescriptor::default());
    let gbuf_albedo_texture = device.create_texture(&TextureDescriptor {
        label: Some("gbuf_albedo"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let gbuf_albedo_view = gbuf_albedo_texture.create_view(&TextureViewDescriptor::default());
    let gbuf_normal_texture = device.create_texture(&TextureDescriptor {
        label: Some("gbuf_normal"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let gbuf_normal_view = gbuf_normal_texture.create_view(&TextureViewDescriptor::default());
    let gbuf_material_texture = device.create_texture(&TextureDescriptor {
        label: Some("gbuf_material"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Uint,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let gbuf_material_view = gbuf_material_texture.create_view(&TextureViewDescriptor::default());
    let gi_sdf_mips = (GI_SDF_RES as f32).log2().floor() as u32 + 1;
    let gi_sdf_texture = device.create_texture(&TextureDescriptor {
        label: Some("gi_sdf"),
        size: Extent3d {
            width: GI_SDF_RES,
            height: GI_SDF_RES,
            depth_or_array_layers: GI_SDF_RES,
        },
        mip_level_count: gi_sdf_mips,
        sample_count: 1,
        dimension: TextureDimension::D3,
        format: TextureFormat::R32Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let gi_sdf_view = gi_sdf_texture.create_view(&TextureViewDescriptor::default());
    let gi_sdf_storage_view = gi_sdf_texture.create_view(&TextureViewDescriptor {
        label: Some("gi_sdf_write"),
        format: None,
        dimension: Some(TextureViewDimension::D3),
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(1),
    });
    let gi_radiance_texture = device.create_texture(&TextureDescriptor {
        label: Some("gi_radiance"),
        size: Extent3d {
            width: GI_SDF_RES,
            height: GI_SDF_RES,
            depth_or_array_layers: GI_SDF_RES,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D3,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let gi_radiance_view = gi_radiance_texture.create_view(&TextureViewDescriptor::default());
    let gi_radiance_storage_view = gi_radiance_texture.create_view(&TextureViewDescriptor {
        label: Some("gi_radiance_write"),
        format: None,
        dimension: Some(TextureViewDimension::D3),
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(1),
    });
    let gi_history_texture = device.create_texture(&TextureDescriptor {
        label: Some("gi_history"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let gi_history_view = gi_history_texture.create_view(&TextureViewDescriptor::default());
    let gi_noisy_texture = device.create_texture(&TextureDescriptor {
        label: Some("gi_noisy"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let gi_noisy_view = gi_noisy_texture.create_view(&TextureViewDescriptor::default());
    let gi_buffer_texture = device.create_texture(&TextureDescriptor {
        label: Some("gi_buffer"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let gi_buffer_view = gi_buffer_texture.create_view(&TextureViewDescriptor::default());

    let motion_texture = device.create_texture(&TextureDescriptor {
        label: Some("motion"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rg16Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let motion_view = motion_texture.create_view(&TextureViewDescriptor::default());

    let variance_texture = device.create_texture(&TextureDescriptor {
        label: Some("variance"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R32Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let variance_view = variance_texture.create_view(&TextureViewDescriptor::default());
    let lightmap_texture = device.create_texture(&TextureDescriptor {
        label: Some("lightmap"),
        size: Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let lightmap_view = lightmap_texture.create_view(&TextureViewDescriptor::default());

    let depth_history_texture = device.create_texture(&TextureDescriptor {
        label: Some("depth_history"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R32Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let depth_history_view = depth_history_texture.create_view(&TextureViewDescriptor::default());

    let normal_history_texture = device.create_texture(&TextureDescriptor {
        label: Some("normal_history"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let normal_history_view = normal_history_texture.create_view(&TextureViewDescriptor::default());

    // shadow occluder mask rendered at half resolution for smoother shadows
    let occ_width = (width / 2).max(1);
    let occ_height = (height / 2).max(1);
    let occluder_texture = device.create_texture(&TextureDescriptor {
        label: Some("occluder_mask"),
        size: Extent3d {
            width: occ_width,
            height: occ_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R8Unorm,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let occluder_view = occluder_texture.create_view(&TextureViewDescriptor::default());
    // use a nearest sampler for compute passes and a separate linear sampler
    // for sprite rendering and occluder upsampling
    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("sampler"),
        mag_filter: FilterMode::Nearest,
        min_filter: FilterMode::Nearest,
        ..Default::default()
    });
    let linear_sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("linear_sampler"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });
    (
        screen_texture,
        screen_view,
        screen_history_texture,
        screen_history_view,
        depth_texture,
        depth_view,
        depth_stencil_texture,
        depth_stencil_view,
        normal_texture,
        normal_view,
        color_texture,
        color_view,
        gbuf_albedo_texture,
        gbuf_albedo_view,
        gbuf_normal_texture,
        gbuf_normal_view,
        gbuf_material_texture,
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
    )
}
