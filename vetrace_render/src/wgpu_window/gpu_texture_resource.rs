use super::*;

// Uploaded material texture resource and sampler.

pub(super) struct GpuTextureResource {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) format: wgpu::TextureFormat,
}

impl GpuTextureResource {
    pub(super) fn new_render_target(
        device: &wgpu::Device,
        label: &str,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { _texture: texture, view, width, height, format }
    }

    pub(super) fn from_rgba8_with_format(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        width: u32,
        height: u32,
        rgba8: &[u8],
        format: wgpu::TextureFormat,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let expected_len = width as usize * height as usize * 4;
        let mut pixels = vec![255_u8; expected_len];
        let copy_len = rgba8.len().min(expected_len);
        pixels[..copy_len].copy_from_slice(&rgba8[..copy_len]);
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
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
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { _texture: texture, view, width, height, format }
    }


    pub(super) fn from_rgba16_float(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        width: u32,
        height: u32,
        rgba16f: &[u16],
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let expected_values = width as usize * height as usize * 4;
        assert_eq!(
            rgba16f.len(),
            expected_values,
            "RGBA16F texture data must contain exactly width * height * 4 values",
        );
        let bytes: &[u8] = bytemuck::cast_slice(rgba16f);
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let format = wgpu::TextureFormat::Rgba16Float;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(8 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { _texture: texture, view, width, height, format }
    }

    pub(super) fn from_rgba8_srgb(device: &wgpu::Device, queue: &wgpu::Queue, label: &str, width: u32, height: u32, rgba8: &[u8]) -> Self {
        Self::from_rgba8_with_format(device, queue, label, width, height, rgba8, wgpu::TextureFormat::Rgba8UnormSrgb)
    }

    pub(super) fn from_rgba8_linear(device: &wgpu::Device, queue: &wgpu::Queue, label: &str, width: u32, height: u32, rgba8: &[u8]) -> Self {
        Self::from_rgba8_with_format(device, queue, label, width, height, rgba8, wgpu::TextureFormat::Rgba8Unorm)
    }
}
