use super::*;

// GPU-owned depth, shadow, AO, EVSM, and render-texture targets.

pub(super) struct DepthTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) sample_view: wgpu::TextureView,
}

impl DepthTarget {
    pub(super) fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vetrace wgpu depth"),
            size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sample_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("vetrace sampled scene depth view"),
            // Keep the view format implicit. On wgpu 0.20 a Depth24PlusStencil8
            // texture can be rendered with the default full depth/stencil view,
            // but a sampled depth-only view must select only the depth aspect.
            // Forcing `format: Some(Depth24PlusStencil8)` here trips validation
            // on some backends even though the texture format is the same.
            format: None,
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
        });
        Self { _texture: texture, view, sample_view }
    }
}

pub(super) struct GpuRenderTextureTarget {
    pub(super) color: GpuTextureResource,
    pub(super) depth: DepthTarget,
    pub(super) camera_buffer: wgpu::Buffer,
    pub(super) camera_bind_group: wgpu::BindGroup,
}

impl GpuRenderTextureTarget {
    pub(super) fn new(
        device: &wgpu::Device,
        camera_layout: &wgpu::BindGroupLayout,
        label: &str,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let color = GpuTextureResource::new_render_target(
            device,
            &format!("vetrace render texture color: {label}"),
            width,
            height,
            format,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let depth = DepthTarget::new(device, width, height);
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("vetrace render texture camera uniform: {label}")),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("vetrace render texture camera bind group: {label}")),
            layout: camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        Self { color, depth, camera_buffer, camera_bind_group }
    }
}

pub(super) struct AmbientOcclusionTarget {
    pub(super) scene_color: GpuTextureResource,
    pub(super) raw: GpuTextureResource,
    pub(super) blurred: GpuTextureResource,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) surface_format: wgpu::TextureFormat,
}

impl AmbientOcclusionTarget {
    pub(super) fn new(device: &wgpu::Device, width: u32, height: u32, surface_format: wgpu::TextureFormat) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            scene_color: GpuTextureResource::new_render_target(
                device,
                "vetrace SSAO scene color",
                width,
                height,
                surface_format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ),
            raw: GpuTextureResource::new_render_target(
                device,
                "vetrace SSAO raw AO",
                width,
                height,
                SSAO_TEXTURE_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ),
            blurred: GpuTextureResource::new_render_target(
                device,
                "vetrace SSAO blurred AO",
                width,
                height,
                SSAO_TEXTURE_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ),
            width,
            height,
            surface_format,
        }
    }
}


pub(super) struct ShadowTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) layer_views: Vec<wgpu::TextureView>,
    pub(super) evsm_moments_a: Option<EvsmMomentTarget>,
    pub(super) evsm_moments_b: Option<EvsmMomentTarget>,
    pub(super) size: u32,
    pub(super) layers: u32,
}

pub(super) struct EvsmMomentTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) layer_views: Vec<wgpu::TextureView>,
}

impl EvsmMomentTarget {
    pub(super) fn new(device: &wgpu::Device, label: &str, size: u32, layers: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: layers },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: EVSM_MOMENT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("{label} array view")),
            format: Some(EVSM_MOMENT_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(layers),
        });
        let mut layer_views = Vec::with_capacity(layers as usize);
        for layer in 0..layers {
            layer_views.push(texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("{label} layer {layer}")),
                format: Some(EVSM_MOMENT_FORMAT),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: layer,
                array_layer_count: Some(1),
            }));
        }
        Self { _texture: texture, view, layer_views }
    }
}

impl ShadowTarget {
    pub(super) fn new(device: &wgpu::Device, size: u32, layers: u32) -> Self {
        let size = size.max(1);
        let layers = layers.clamp(1, SHADOW_CASCADE_COUNT as u32);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vetrace directional shadow cascade array"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: layers },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("vetrace directional shadow cascade texture array view"),
            format: Some(SHADOW_DEPTH_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(layers),
        });
        let mut layer_views = Vec::with_capacity(layers as usize);
        for layer in 0..layers {
            layer_views.push(texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("vetrace directional shadow cascade layer view"),
                format: Some(SHADOW_DEPTH_FORMAT),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: layer,
                array_layer_count: Some(1),
            }));
        }
        Self { _texture: texture, view, layer_views, evsm_moments_a: None, evsm_moments_b: None, size, layers }
    }

    pub(super) fn evsm_view_or<'a>(&'a self, fallback: &'a EvsmMomentTarget) -> &'a wgpu::TextureView {
        self.evsm_moments_a.as_ref().map(|target| &target.view).unwrap_or(&fallback.view)
    }

    pub(super) fn ensure_evsm_moments(&mut self, device: &wgpu::Device) -> bool {
        if self.evsm_moments_a.is_some() && self.evsm_moments_b.is_some() {
            return false;
        }
        self.evsm_moments_a = Some(EvsmMomentTarget::new(device, "vetrace EVSM moments A", self.size, self.layers));
        self.evsm_moments_b = Some(EvsmMomentTarget::new(device, "vetrace EVSM moments B", self.size, self.layers));
        true
    }

    pub(super) fn drop_evsm_moments(&mut self) -> bool {
        let had_evsm = self.evsm_moments_a.is_some() || self.evsm_moments_b.is_some();
        self.evsm_moments_a = None;
        self.evsm_moments_b = None;
        had_evsm
    }
}
