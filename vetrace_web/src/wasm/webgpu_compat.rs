use wasm_bindgen::prelude::*;

// wgpu 0.20.1 targets an older WebGPU limit name and unconditionally adds
// `maxInterStageShaderComponents` to GPUDeviceDescriptor.requiredLimits.
// Current browsers reject unknown required-limit names before creating the
// device. Keep the Rust renderer on wgpu 0.20.1 for desktop/API compatibility,
// but remove that retired browser-only key at the JavaScript boundary.
#[wasm_bindgen(inline_js = r#"
const VETRACE_WGPU_LIMIT_PATCH = Symbol.for("vetrace.wgpu-0.20-limit-compat");
const RETIRED_LIMIT = "maxInterStageShaderComponents";


function reportVetraceGpuFailure(kind, message) {
    const text = String(message || "Unknown WebGPU failure");
    const derivative = /invalid due to a previous error|while calling \[Queue\]\.Submit/i.test(text);

    // Browsers commonly emit a useful validation error first and then a much
    // less useful "invalid CommandBuffer" error during queue submission. Keep
    // the first root error visible instead of replacing it with the consequence.
    if (derivative && globalThis.__vetraceFirstGpuError) {
        console.warn("Vetrace WebGPU follow-on error:", text);
        return;
    }

    const detail = { kind, message: text };
    if (!globalThis.__vetraceFirstGpuError) {
        globalThis.__vetraceFirstGpuError = detail;
    }
    console.error("Vetrace WebGPU " + kind + ":", detail.message);
    if (typeof globalThis.dispatchEvent === "function" && typeof globalThis.CustomEvent === "function") {
        globalThis.dispatchEvent(new CustomEvent("vetrace-webgpu-error", { detail }));
    }
}

function monitorDevice(device) {
    if (!device) {
        return device;
    }
    if (typeof device.addEventListener === "function") {
        device.addEventListener("uncapturederror", (event) => {
            reportVetraceGpuFailure("validation error", event?.error?.message || event?.error || event);
        });
    }
    if (device.lost && typeof device.lost.then === "function") {
        device.lost.then((info) => {
            reportVetraceGpuFailure("device lost", info?.message || info?.reason || info);
        });
    }
    return device;
}

function sanitizeDeviceDescriptor(descriptor) {
    if (!descriptor || !descriptor.requiredLimits) {
        return descriptor;
    }

    const requiredLimits = descriptor.requiredLimits;
    if (!Object.prototype.hasOwnProperty.call(requiredLimits, RETIRED_LIMIT)) {
        return descriptor;
    }

    const sanitizedLimits = {};
    for (const [name, value] of Object.entries(requiredLimits)) {
        if (name !== RETIRED_LIMIT) {
            sanitizedLimits[name] = value;
        }
    }

    return {
        ...descriptor,
        requiredLimits: sanitizedLimits,
    };
}

function patchAdapterPrototype(prototype) {
    if (!prototype || prototype[VETRACE_WGPU_LIMIT_PATCH]) {
        return Boolean(prototype);
    }

    const originalRequestDevice = prototype.requestDevice;
    if (typeof originalRequestDevice !== "function") {
        return false;
    }

    try {
        Object.defineProperty(prototype, "requestDevice", {
            configurable: true,
            writable: true,
            value: function requestDeviceVetraceCompat(descriptor) {
                return originalRequestDevice
                    .call(this, sanitizeDeviceDescriptor(descriptor))
                    .then(monitorDevice);
            },
        });
        Object.defineProperty(prototype, VETRACE_WGPU_LIMIT_PATCH, {
            configurable: false,
            enumerable: false,
            value: true,
        });
        return true;
    } catch (error) {
        console.warn("Vetrace could not patch GPUAdapter.requestDevice directly", error);
        return false;
    }
}

function patchAdapterInstance(adapter) {
    if (!adapter || adapter[VETRACE_WGPU_LIMIT_PATCH]) {
        return Boolean(adapter);
    }

    const originalRequestDevice = adapter.requestDevice;
    if (typeof originalRequestDevice !== "function") {
        return false;
    }

    try {
        Object.defineProperty(adapter, "requestDevice", {
            configurable: true,
            writable: true,
            value: function requestDeviceVetraceCompat(descriptor) {
                return originalRequestDevice
                    .call(adapter, sanitizeDeviceDescriptor(descriptor))
                    .then(monitorDevice);
            },
        });
        Object.defineProperty(adapter, VETRACE_WGPU_LIMIT_PATCH, {
            configurable: false,
            enumerable: false,
            value: true,
        });
        return true;
    } catch (error) {
        console.warn("Vetrace could not patch this GPUAdapter instance", error);
        return false;
    }
}

export function install_vetrace_webgpu_limit_compatibility() {
    if (typeof navigator === "undefined" || !navigator.gpu) {
        return;
    }

    // Chromium and current Firefox expose GPUAdapter globally.
    if (globalThis.GPUAdapter?.prototype) {
        patchAdapterPrototype(globalThis.GPUAdapter.prototype);
    }

    // Fallback for implementations that do not expose the GPUAdapter
    // constructor: intercept adapter creation once, then patch the returned
    // adapter's actual prototype before wgpu asks it for a device.
    const gpuPrototype = Object.getPrototypeOf(navigator.gpu);
    if (!gpuPrototype || gpuPrototype[VETRACE_WGPU_LIMIT_PATCH]) {
        return;
    }

    const originalRequestAdapter = gpuPrototype.requestAdapter;
    if (typeof originalRequestAdapter !== "function") {
        return;
    }

    try {
        Object.defineProperty(gpuPrototype, "requestAdapter", {
            configurable: true,
            writable: true,
            value: function requestAdapterVetraceCompat(options) {
                return originalRequestAdapter.call(this, options).then((adapter) => {
                    if (adapter) {
                        if (!patchAdapterPrototype(Object.getPrototypeOf(adapter))) {
                            patchAdapterInstance(adapter);
                        }
                    }
                    return adapter;
                });
            },
        });
        Object.defineProperty(gpuPrototype, VETRACE_WGPU_LIMIT_PATCH, {
            configurable: false,
            enumerable: false,
            value: true,
        });
    } catch (error) {
        console.warn("Vetrace could not install the WebGPU adapter compatibility wrapper", error);
    }
}
"#)]
unsafe extern "C" {
    fn install_vetrace_webgpu_limit_compatibility();
}

pub(super) fn install() {
    // SAFETY: the imported function only patches browser JavaScript methods and
    // does not read or write Rust memory. Calling it repeatedly is idempotent.
    unsafe { install_vetrace_webgpu_limit_compatibility() };
}
