pub mod renderer;
#[cfg(feature = "wgpu")]
pub mod wgpu_renderer;
pub mod resource;
pub mod ssbo;
pub mod texture;
#[cfg(feature = "use_epi")]
pub mod egui;
#[cfg(all(feature = "wgpu", feature = "use_epi"))]
pub mod egui_wgpu;

#[cfg(feature = "wgpu")]
pub use wgpu_renderer::WgpuRenderer as Renderer;
#[cfg(not(feature = "wgpu"))]
pub use renderer::Renderer;
pub use renderer::{RenderParams, RayTracingConfig};
pub use resource::{compile_shader, link_program, load_obj_file, Vec3, Triangle};
pub use ssbo::{create_ssbo, update_ssbo};
pub use texture::{TextureHandle, TextureStorage};
#[cfg(all(feature = "use_epi", feature = "wgpu"))]
pub use egui_wgpu::EguiRenderer;
#[cfg(all(feature = "use_epi", not(feature = "wgpu")))]
pub use egui::EguiRenderer;
