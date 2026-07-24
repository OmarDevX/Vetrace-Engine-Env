struct ProcessSample {
    cpu_percent: Option<f32>,
    memory_bytes: Option<u64>,
    system_memory_total_bytes: Option<u64>,
    system_memory_available_bytes: Option<u64>,
}

#[derive(Default)]
struct ProcessSampler {
    previous_cpu_ticks: Option<u64>,
    previous_wall: Option<Instant>,
}

impl ProcessSampler {
    fn sample(&mut self) -> ProcessSample {
        sample_process_impl(self)
    }
}

#[cfg(target_os = "linux")]
fn sample_process_impl(sampler: &mut ProcessSampler) -> ProcessSample {
    let now = Instant::now();
    let cpu_ticks = read_linux_process_cpu_ticks();
    let cpu_percent = match (cpu_ticks, sampler.previous_cpu_ticks, sampler.previous_wall) {
        (Some(current), Some(previous), Some(previous_wall)) => {
            let delta_ticks = current.saturating_sub(previous) as f32;
            let delta_seconds = now.duration_since(previous_wall).as_secs_f32().max(0.000_001);
            // Most Linux desktops use USER_HZ=100. Keeping this dependency-free is
            // intentional; the value is good enough to spot spikes/regressions.
            Some((delta_ticks / 100.0) / delta_seconds * 100.0)
        }
        _ => None,
    };
    if let Some(current) = cpu_ticks {
        sampler.previous_cpu_ticks = Some(current);
        sampler.previous_wall = Some(now);
    }

    let (system_total, system_available) = read_linux_meminfo();
    ProcessSample {
        cpu_percent,
        memory_bytes: read_linux_process_rss_bytes(),
        system_memory_total_bytes: system_total,
        system_memory_available_bytes: system_available,
    }
}

#[cfg(not(target_os = "linux"))]
fn sample_process_impl(_sampler: &mut ProcessSampler) -> ProcessSample {
    ProcessSample::default()
}

#[cfg(target_os = "linux")]
fn read_linux_process_cpu_ticks() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let (_, rest) = stat.rsplit_once(") ")?;
    let fields = rest.split_whitespace().collect::<Vec<_>>();
    let utime = fields.get(11)?.parse::<u64>().ok()?;
    let stime = fields.get(12)?.parse::<u64>().ok()?;
    Some(utime.saturating_add(stime))
}

#[cfg(target_os = "linux")]
fn read_linux_process_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            let kib = value.split_whitespace().next()?.parse::<u64>().ok()?;
            return Some(kib.saturating_mul(1024));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn read_linux_meminfo() -> (Option<u64>, Option<u64>) {
    let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") else { return (None, None); };
    let mut total = None;
    let mut available = None;
    for line in meminfo.lines() {
        if let Some(value) = line.strip_prefix("MemTotal:") {
            total = value.split_whitespace().next().and_then(|v| v.parse::<u64>().ok()).map(|kib| kib.saturating_mul(1024));
        } else if let Some(value) = line.strip_prefix("MemAvailable:") {
            available = value.split_whitespace().next().and_then(|v| v.parse::<u64>().ok()).map(|kib| kib.saturating_mul(1024));
        }
    }
    (total, available)
}
