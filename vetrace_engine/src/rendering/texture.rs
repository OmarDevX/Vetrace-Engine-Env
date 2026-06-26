use std::collections::HashMap;
use std::path::Path;
#[cfg(not(feature = "wgpu"))]
use gl::types::*;
#[cfg(feature = "wgpu")]
use wgpu::{self, util::DeviceExt};
#[cfg(feature = "wgpu")]
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[cfg(feature = "wgpu")]
static DEVICE_QUEUE: OnceCell<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> = OnceCell::new();

#[cfg(feature = "wgpu")]
pub fn set_wgpu_device_queue(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) {
    let _ = DEVICE_QUEUE.set((device, queue));
}

#[cfg(feature = "wgpu")]
#[derive(Clone, Debug)]
pub struct TextureHandle {
    pub texture: Arc<wgpu::Texture>,
    pub view: Arc<wgpu::TextureView>,
}

#[cfg(feature = "wgpu")]
impl Default for TextureHandle {
    fn default() -> Self {
        let (device, queue) = DEVICE_QUEUE
            .get()
            .expect("wgpu device/queue not initialized");
        let size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("default_tex"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
            &[255, 255, 255, 255],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture: Arc::new(texture), view: Arc::new(view) }
    }
}

#[cfg(not(feature = "wgpu"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct TextureHandle(pub u32); // u32 == GLuint in OpenGL

pub struct TextureStorage {
    textures: HashMap<String, TextureHandle>,
}

impl TextureStorage {
    pub fn new() -> Self {
        Self { textures: HashMap::new() }
    }

    #[cfg(feature = "wgpu")]
    pub fn load_texture<P: AsRef<Path>>(&mut self, path: P) -> TextureHandle {
        let (device, queue) = DEVICE_QUEUE
            .get()
            .expect("wgpu device/queue not initialized");
        let path_str = path.as_ref().to_string_lossy().to_string();
        if let Some(handle) = self.textures.get(&path_str) {
            return handle.clone();
        }

        let img = image::open(&path).expect("Failed to load image").flipv().to_rgba8();
        let (width, height) = img.dimensions();
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&path_str),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
            &img,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let handle = TextureHandle { texture: Arc::new(texture), view: Arc::new(view) };
        self.textures.insert(path_str, handle.clone());
        handle
    }

    #[cfg(not(feature = "wgpu"))]
    pub fn load_texture<P: AsRef<Path>>(&mut self, path: P) -> TextureHandle {
        let path_str = path.as_ref().to_string_lossy().to_string();
        if let Some(handle) = self.textures.get(&path_str) {
            return *handle;
        }

        let img = image::open(&path).expect("Failed to load image");
        // flip vertically for OpenGL and convert to RGBA8
        let img = img.flipv().to_rgba8();
        let (width, height) = img.dimensions();
        let data = img.as_raw();

        let mut texture_id: GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as GLint,
                width as GLsizei,
                height as GLsizei,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                data.as_ptr() as *const _,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        let handle = TextureHandle(texture_id);
        self.textures.insert(path_str, handle);
        handle
    }
}