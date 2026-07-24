use super::*;

// WGPU adapter/device initialization helpers.

pub(super) struct InitialDevice {
    pub(super) adapter: wgpu::Adapter,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    #[cfg(feature = "profiler")]
    pub(super) gpu_timestamp_profiler: Option<GpuTimestampProfiler>,
}

pub(super) async fn request_initial_device(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
    adapter_preference: AdapterPreference,
) -> Result<InitialDevice, String> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: adapter_preference.wgpu_power_preference(),
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| "no suitable WGPU adapter found".to_string())?;
    eprintln!("vetrace_render: WGPU adapter preference {:?}, selected adapter: {:?}", adapter_preference, adapter.get_info());

    #[cfg(feature = "profiler")]
    let mut required_features = wgpu::Features::empty();
    #[cfg(feature = "profiler")]
    {
        if adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            required_features |= wgpu::Features::TIMESTAMP_QUERY;
        }
    }
    #[cfg(not(feature = "profiler"))]
    let required_features = wgpu::Features::empty();
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("vetrace wgpu device"),
                required_features,
                required_limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .map_err(|err| err.to_string())?;

    #[cfg(feature = "profiler")]
    let gpu_timestamp_profiler = if required_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
        Some(GpuTimestampProfiler::new(&device, &queue))
    } else {
        None
    };

    Ok(InitialDevice {
        adapter,
        device,
        queue,
        #[cfg(feature = "profiler")]
        gpu_timestamp_profiler,
    })
}
