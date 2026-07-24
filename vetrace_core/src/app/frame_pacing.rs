use std::error::Error;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

pub(super) fn validate_frame_delta(dt: f32) -> Result<(), Box<dyn Error>> {
    if dt.is_finite() && dt >= 0.0 {
        Ok(())
    } else {
        Err(format!("frame delta must be finite and non-negative, got {dt}").into())
    }
}

pub(super) fn pace_frame(frame_start: Instant, target_frame: Duration) {
    let Some(target_end) = frame_start.checked_add(target_frame) else { return; };
    loop {
        let now = Instant::now();
        if now >= target_end { break; }
        let remaining = target_end.saturating_duration_since(now);
        if remaining > Duration::from_millis(2) {
            std::thread::sleep(remaining - Duration::from_millis(1));
        } else if remaining > Duration::from_micros(500) {
            std::thread::yield_now();
        } else {
            std::hint::spin_loop();
        }
    }
}
