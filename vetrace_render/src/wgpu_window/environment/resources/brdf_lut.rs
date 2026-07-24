use super::*;

pub(super) fn create_environment_brdf_lut(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> GpuEnvironmentBrdfLut {
    const SIZE: u32 = 128;
    const SAMPLE_COUNT: u32 = 128;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vetrace environment BRDF integration LUT"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rg16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("vetrace environment BRDF LUT sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let row_bytes = SIZE as usize * 4;
    let padded_row_bytes = (row_bytes + 255) & !255;
    let mut pixels = vec![0_u8; padded_row_bytes * SIZE as usize];
    for y in 0..SIZE {
        let roughness = (y as f32 + 0.5) / SIZE as f32;
        for x in 0..SIZE {
            let ndotv = ((x as f32 + 0.5) / SIZE as f32).clamp(0.0001, 1.0);
            let integrated = integrate_environment_brdf(ndotv, roughness, SAMPLE_COUNT);
            let offset = y as usize * padded_row_bytes + x as usize * 4;
            let a = f16::from_f32(integrated[0]).to_bits().to_le_bytes();
            let b = f16::from_f32(integrated[1]).to_bits().to_le_bytes();
            pixels[offset..offset + 2].copy_from_slice(&a);
            pixels[offset + 2..offset + 4].copy_from_slice(&b);
        }
    }
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(padded_row_bytes as u32),
            rows_per_image: Some(SIZE),
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    GpuEnvironmentBrdfLut {
        _texture: texture,
        view,
        sampler,
    }
}

pub(super) fn integrate_environment_brdf(ndotv: f32, roughness: f32, sample_count: u32) -> [f32; 2] {
    let view = Vec3::new((1.0 - ndotv * ndotv).max(0.0).sqrt(), 0.0, ndotv);
    let mut scale = 0.0_f32;
    let mut bias = 0.0_f32;
    for index in 0..sample_count {
        let xi = Vec2::new(
            index as f32 / sample_count.max(1) as f32,
            radical_inverse_vdc(index),
        );
        let half_vector = importance_sample_ggx_cpu(xi, roughness);
        let light = (2.0 * view.dot(half_vector) * half_vector - view).normalize_or_zero();
        let ndotl = light.z.max(0.0);
        let ndoth = half_vector.z.max(0.0);
        let vdoth = view.dot(half_vector).max(0.0);
        if ndotl <= 0.0 || ndoth <= 0.0 || vdoth <= 0.0 {
            continue;
        }
        let geometry = geometry_smith_ibl(ndotv, ndotl, roughness);
        let visibility = geometry * vdoth / (ndoth * ndotv).max(0.0001);
        let fresnel = (1.0 - vdoth).powi(5);
        scale += (1.0 - fresnel) * visibility;
        bias += fresnel * visibility;
    }
    let denominator = sample_count.max(1) as f32;
    [scale / denominator, bias / denominator]
}

pub(super) fn importance_sample_ggx_cpu(xi: Vec2, roughness: f32) -> Vec3 {
    let alpha = (roughness * roughness).max(0.0001);
    let phi = std::f32::consts::TAU * xi.x;
    let cos_theta = ((1.0 - xi.y) / (1.0 + (alpha * alpha - 1.0) * xi.y).max(0.0001)).sqrt();
    let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
    Vec3::new(phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta).normalize_or_zero()
}

pub(super) fn geometry_smith_ibl(ndotv: f32, ndotl: f32, roughness: f32) -> f32 {
    let k = roughness * roughness * 0.5;
    let gv = ndotv / (ndotv * (1.0 - k) + k).max(0.0001);
    let gl = ndotl / (ndotl * (1.0 - k) + k).max(0.0001);
    gv * gl
}

pub(super) fn radical_inverse_vdc(bits: u32) -> f32 {
    bits.reverse_bits() as f32 * 2.328_306_4e-10
}
