use super::*;

// Platform-neutral surface configuration helpers.

pub(super) fn configure_initial_surface(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    width: u32,
    height: u32,
    present_mode_preference: PresentModePreference,
) -> wgpu::SurfaceConfiguration {
    let caps = surface.get_capabilities(adapter);
    #[cfg(target_arch = "wasm32")]
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|format| matches!(format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm))
        .unwrap_or_else(|| caps.formats[0]);
    #[cfg(not(target_arch = "wasm32"))]
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|format| format.is_srgb())
        .unwrap_or_else(|| caps.formats[0]);
    let surface_view_format = render_view_format(format);
    let present_mode = choose_present_mode(&caps.present_modes, present_mode_preference);
    eprintln!(
        "vetrace_render: WGPU present mode {:?} selected from {:?} using {:?}",
        present_mode, caps.present_modes, present_mode_preference
    );
    #[cfg(target_arch = "wasm32")]
    let alpha_mode = caps
        .alpha_modes
        .iter()
        .copied()
        .find(|mode| *mode == wgpu::CompositeAlphaMode::Opaque)
        .unwrap_or_else(|| caps.alpha_modes[0]);
    #[cfg(not(target_arch = "wasm32"))]
    let alpha_mode = caps.alpha_modes[0];
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: width.max(1),
        height: height.max(1),
        present_mode,
        alpha_mode,
        // Browser canvas textures are most portable when rendered through
        // their configured format directly. In particular, wgpu 0.20's
        // WebGPU backend can configure an alternate sRGB surface view that
        // current browsers accept during setup but later reject when the view
        // is used as a render attachment, invalidating the command encoder.
        // Native surfaces already select an sRGB format when available.
        view_formats: Vec::new(),
        desired_maximum_frame_latency: 2,
    };
    surface.configure(device, &config);
    config
}

pub(super) fn render_view_format(surface_format: wgpu::TextureFormat) -> wgpu::TextureFormat {
    // Keep the render-pipeline target identical to the configured surface
    // format. Desktop selects an sRGB surface format when one is available;
    // browser WebGPU normally exposes an unorm canvas format and must not be
    // reinterpreted through an alternate view on the wgpu 0.20 web backend.
    surface_format
}

pub(super) fn choose_present_mode(available: &[wgpu::PresentMode], preference: PresentModePreference) -> wgpu::PresentMode {
    fn first_available(available: &[wgpu::PresentMode], ordered: &[wgpu::PresentMode]) -> Option<wgpu::PresentMode> {
        ordered.iter().copied().find(|candidate| available.contains(candidate))
    }

    let ordered: &[wgpu::PresentMode] = match preference {
        PresentModePreference::Vsync | PresentModePreference::Fifo => &[
            wgpu::PresentMode::Fifo,
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
        ],
        PresentModePreference::LowLatency => &[
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::Fifo,
        ],
        PresentModePreference::Immediate => &[
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Fifo,
        ],
        PresentModePreference::Mailbox => &[
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Fifo,
            wgpu::PresentMode::Immediate,
        ],
    };

    first_available(available, ordered)
        .or_else(|| available.first().copied())
        .unwrap_or(wgpu::PresentMode::Fifo)
}
