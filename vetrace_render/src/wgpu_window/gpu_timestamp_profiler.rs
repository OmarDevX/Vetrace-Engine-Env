use super::*;

#[cfg(feature = "profiler")]
pub(super) const GPU_TIMESTAMP_QUERY_CAPACITY: u32 = 64;

#[cfg(feature = "profiler")]
pub(super) struct GpuTimestampProfiler {
    pub(super) query_set: wgpu::QuerySet,
    pub(super) resolve_buffer: wgpu::Buffer,
    pub(super) readback_buffer: wgpu::Buffer,
    pub(super) timestamp_period_ns: f32,
    pub(super) active: bool,
    pub(super) query_count: u32,
    pub(super) labels: Vec<GpuTimestampLabel>,
    pub(super) submitted: Option<GpuTimestampSubmitted>,
    pub(super) pending: Option<GpuTimestampPending>,
}

#[cfg(feature = "profiler")]
#[derive(Clone, Debug)]
pub(super) struct GpuTimestampLabel {
    pub(super) name: &'static str,
    pub(super) start_index: u32,
    pub(super) end_index: u32,
}

#[cfg(feature = "profiler")]
pub(super) struct GpuTimestampSubmitted {
    pub(super) labels: Vec<GpuTimestampLabel>,
    pub(super) bytes: u64,
}

#[cfg(feature = "profiler")]
pub(super) struct GpuTimestampPending {
    pub(super) labels: Vec<GpuTimestampLabel>,
    pub(super) bytes: u64,
    pub(super) receiver: std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
}

#[cfg(feature = "profiler")]
impl GpuTimestampProfiler {
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("vetrace profiler gpu timestamp queries"),
            ty: wgpu::QueryType::Timestamp,
            count: GPU_TIMESTAMP_QUERY_CAPACITY,
        });
        let buffer_size = GPU_TIMESTAMP_QUERY_CAPACITY as u64 * std::mem::size_of::<u64>() as u64;
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vetrace profiler gpu timestamp resolve buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vetrace profiler gpu timestamp readback buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            query_set,
            resolve_buffer,
            readback_buffer,
            timestamp_period_ns: queue.get_timestamp_period(),
            active: false,
            query_count: 0,
            labels: Vec::new(),
            submitted: None,
            pending: None,
        }
    }

    pub(super) fn begin_frame(&mut self, device: &wgpu::Device) {
        self.poll_results(device);
        self.query_count = 0;
        self.labels.clear();
        self.active = self.pending.is_none() && self.submitted.is_none();
        vetrace_profiler::record_counter("wgpu.gpu.timestamp_queries_enabled", 1.0, "");
        vetrace_profiler::record_counter("wgpu.gpu.timestamp_period_ns", self.timestamp_period_ns as f64, "ns");
    }

    pub(super) fn reserve_pass(&mut self, name: &'static str) -> Option<(u32, u32)> {
        if !self.active || self.query_count + 2 > GPU_TIMESTAMP_QUERY_CAPACITY {
            return None;
        }
        let start_index = self.query_count;
        let end_index = self.query_count + 1;
        self.query_count += 2;
        self.labels.push(GpuTimestampLabel { name, start_index, end_index });
        Some((start_index, end_index))
    }

    pub(super) fn timestamp_writes_for(&self, indices: Option<(u32, u32)>) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let (start_index, end_index) = indices?;
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(start_index),
            end_of_pass_write_index: Some(end_index),
        })
    }

    pub(super) fn finish_encoder(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if !self.active || self.query_count == 0 || self.pending.is_some() || self.submitted.is_some() {
            return;
        }
        let bytes = self.query_count as u64 * std::mem::size_of::<u64>() as u64;
        encoder.resolve_query_set(&self.query_set, 0..self.query_count, &self.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(&self.resolve_buffer, 0, &self.readback_buffer, 0, bytes);
        self.submitted = Some(GpuTimestampSubmitted { labels: self.labels.clone(), bytes });
        self.active = false;
    }

    pub(super) fn after_submit(&mut self) {
        let Some(submitted) = self.submitted.take() else { return; };
        let (sender, receiver) = std::sync::mpsc::channel();
        self.readback_buffer
            .slice(0..submitted.bytes)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        self.pending = Some(GpuTimestampPending { labels: submitted.labels, bytes: submitted.bytes, receiver });
    }

    pub(super) fn poll_results(&mut self, device: &wgpu::Device) {
        let Some(pending) = self.pending.as_ref() else { return; };
        device.poll(wgpu::Maintain::Poll);
        match pending.receiver.try_recv() {
            Ok(Ok(())) => {}
            Ok(Err(_)) => {
                self.pending = None;
                vetrace_profiler::record_counter("wgpu.gpu.timestamp_readback_failed", 1.0, "");
                return;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                return;
            }
        }

        let Some(pending) = self.pending.take() else {
            return;
        };
        let mapped = self.readback_buffer.slice(0..pending.bytes).get_mapped_range();
        let values = mapped
            .chunks_exact(std::mem::size_of::<u64>())
            .map(|bytes| {
                let mut value = [0_u8; std::mem::size_of::<u64>()];
                value.copy_from_slice(bytes);
                u64::from_ne_bytes(value)
            })
            .collect::<Vec<_>>();

        for label in &pending.labels {
            let Some(&start) = values.get(label.start_index as usize) else { continue; };
            let Some(&end) = values.get(label.end_index as usize) else { continue; };
            if end <= start {
                continue;
            }
            let elapsed_ns = (end - start) as f64 * self.timestamp_period_ns as f64;
            if elapsed_ns.is_finite() && elapsed_ns >= 0.0 {
                vetrace_profiler::record_timing(label.name, Duration::from_secs_f64(elapsed_ns / 1_000_000_000.0));
            }
        }
        drop(mapped);
        self.readback_buffer.unmap();
    }
}

/// A real WGPU/GPU render target. It uses winit for window/events and WGPU for
/// presentation/rasterization. There is no SDL software drawing in this path.
#[cfg(not(all(feature = "egui_render", feature = "profiler", feature = "wgpu_window")))]
pub(super) struct DetachedProfilerWindow;

