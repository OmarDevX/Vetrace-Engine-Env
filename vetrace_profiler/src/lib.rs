//! Vetrace Profiler.
//!
//! What it measures:
//! - Core app/plugin update/render timings through the `vetrace_core` profiler hook.
//! - Any custom scope recorded with [`profile_scope!`] or [`ScopeTimer`].
//! - Process CPU/RAM on Linux through `/proc` with zero extra dependencies.
//! - Renderer/WGPU counters when `vetrace_render/profiler` is enabled.
//!
//! WGPU does not expose portable OS-level GPU utilization or VRAM totals.
//! The WGPU values recorded by the renderer are Vetrace-owned GPU resource
//! estimates, CPU-side pass encode/submit timings, and real pass-level GPU
//! timestamp rows (`wgpu.gpu.*`) when the active adapter supports timestamp
//! queries.

use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use vetrace_core::app::Plugin;
use vetrace_core::backends::ProfilerBackend;
use vetrace_core::engine::Engine;
use vetrace_core::DebugTextOverlayPanel;

static GLOBAL_PROFILER: OnceLock<Mutex<Weak<Mutex<ProfilerInner>>>> = OnceLock::new();

// Profiler internals are physically split but included into the crate root so
// the public API and private helper visibility stay unchanged.
include!("profiler/public_types.rs");
include!("profiler/plugin.rs");
include!("profiler/inner.rs");
include!("profiler/report.rs");
include!("profiler/process.rs");
