const BOOT_VERSION = "11";
const params = new URLSearchParams(window.location.search);
const requestedSlug = params.get("example") ?? "rotating-cube";
const requestedGpuPreference = params.get("gpu") === "high-performance"
  ? "high-performance"
  : "low-power";
const titleNodes = document.querySelectorAll("[data-example-title]");
const description = document.querySelector("[data-example-description]");
const categoryNodes = document.querySelectorAll("[data-example-category]");
const code = document.querySelector("[data-example-source]");
const canvas = document.querySelector("#vetrace-canvas");
const backend = document.querySelector("[data-backend]");
const status = document.querySelector("#runtime-status");
const overlay = document.querySelector("[data-runtime-overlay]");
const overlayTitle = document.querySelector("[data-runtime-overlay-title]");
const overlayMessage = document.querySelector("[data-runtime-overlay-message]");
const overlayDetails = document.querySelector("[data-runtime-overlay-details]");

window.__vetraceModuleStarted = true;
window.addEventListener("vetrace-webgpu-error", (event) => {
  const detail = event.detail ?? {};
  const message = detail.message ?? "The GPU device reported an uncaptured error.";
  const derivative = /invalid due to a previous error|while calling \[Queue\]\.Submit/i.test(message);
  if (derivative && window.__vetraceDisplayedGpuRootError) {
    console.warn("Ignoring follow-on WebGPU submission error", message);
    return;
  }
  window.__vetraceDisplayedGpuRootError = true;
  const externalMemoryFailure = /Requested allocation size .* smaller than the image requires|ImportMemory/i.test(message);
  fail(
    externalMemoryFailure ? "Browser GPU import failure" : "WebGPU runtime error",
    externalMemoryFailure
      ? browserGpuImportFailureMessage()
      : message,
    `${detail.kind ?? "WebGPU error"}: ${message}`,
  );
});

function setText(nodes, value) {
  nodes.forEach((node) => { node.textContent = value; });
}

function setPhase(message) {
  status.textContent = message;
  status.className = "runtime-status";
}

function formatError(error) {
  if (error instanceof Error) {
    return error.stack || error.message;
  }
  return String(error);
}

function fail(title, message, details = "") {
  status.textContent = message;
  status.className = "runtime-status error";
  overlayTitle.textContent = title;
  overlayMessage.textContent = message;
  overlayDetails.textContent = details;
  overlayDetails.hidden = !details;
  overlay.classList.add("visible");
}

function browserGpuImportFailureMessage() {
  if (requestedGpuPreference === "high-performance") {
    return "The browser/driver rejected the dedicated-GPU canvas image. Reload without ?gpu=high-performance so Vetrace can use the low-power adapter.";
  }
  return "The browser/driver rejected the WebGPU canvas image. This is outside Vetrace's texture allocator. Update the browser/GPU driver or run the page on a working integrated-GPU WebGPU adapter.";
}

function versioned(url) {
  const value = new URL(url, import.meta.url);
  value.searchParams.set("v", BOOT_VERSION);
  return value.href;
}

async function loadExampleMetadata() {
  try {
    const data = await import(versioned("./examples-data.js"));
    return data.exampleBySlug(requestedSlug);
  } catch (error) {
    console.error("Unable to load example metadata", error);
    return {
      slug: requestedSlug,
      title: requestedSlug.split("-").map((part) => part[0]?.toUpperCase() + part.slice(1)).join(" "),
      category: "Example",
      description: "The example metadata module could not be loaded.",
      source: "// Example metadata unavailable",
    };
  }
}

async function verifyWebGpu() {
  if (!window.isSecureContext) {
    throw new Error(
      "WebGPU requires a secure context. Use http://127.0.0.1 or http://localhost for local development, or HTTPS when hosted."
    );
  }
  if (!("gpu" in navigator)) {
    throw new Error(
      "This browser does not expose navigator.gpu. Open the site in a WebGPU-enabled browser or enable WebGPU in your browser settings."
    );
  }
  const adapter = await navigator.gpu.requestAdapter({ powerPreference: requestedGpuPreference });
  if (!adapter) {
    throw new Error(
      "WebGPU is present, but the browser could not obtain a GPU adapter. Check hardware acceleration, GPU drivers, and browser WebGPU support."
    );
  }
}

async function verifyRuntimeFile(url, label) {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`${label} request failed with HTTP ${response.status}: ${url}`);
  }
  return response;
}

async function boot() {
  const example = await loadExampleMetadata();
  params.set("example", example.slug);
  if (requestedGpuPreference === "high-performance") {
    params.set("gpu", "high-performance");
  } else {
    params.delete("gpu");
  }
  window.history.replaceState(null, "", `?${params.toString()}`);
  document.title = `${example.title} · Vetrace Examples`;
  setText(titleNodes, example.title);
  setText(categoryNodes, example.category);
  description.textContent = example.description;
  code.textContent = example.source;
  canvas.dataset.example = example.slug;
  canvas.dataset.gpuPreference = requestedGpuPreference;

  setPhase("Checking browser WebGPU support…");
  await verifyWebGpu();
  backend.textContent = `WebGPU · full Vetrace renderer · ${requestedGpuPreference}`;

  const packageUrl = new URL("../pkg/vetrace_web.js", import.meta.url);
  const wasmUrl = new URL("../pkg/vetrace_web_bg.wasm", import.meta.url);

  setPhase("Checking generated Vetrace WebAssembly package…");
  await verifyRuntimeFile(packageUrl.href, "JavaScript package");
  await verifyRuntimeFile(wasmUrl.href, "WebAssembly binary");

  setPhase("Loading Vetrace WebAssembly…");
  const runtime = await import(versioned(packageUrl.href));
  if (typeof runtime.default !== "function") {
    throw new Error("Generated package does not export the wasm-bindgen initializer.");
  }
  if (typeof runtime.start_example !== "function") {
    throw new Error("Generated package does not export start_example. Rebuild with ./scripts/build_web.sh.");
  }

  await runtime.default(wasmUrl.href);
  setPhase("Starting example…");
  await runtime.start_example("vetrace-canvas", example.slug);
}

boot().catch((error) => {
  console.error("Vetrace web boot failed", error);
  const details = formatError(error);
  const missingPackage =
    details.includes("JavaScript package request failed with HTTP 404") ||
    details.includes("WebAssembly binary request failed with HTTP 404") ||
    (details.includes("Failed to fetch dynamically imported module") && details.includes("vetrace_web.js"));
  const obsoleteWgpuLimit = details.includes("maxInterStageShaderComponents") && details.includes("not recognized");
  const externalMemoryFailure = /Requested allocation size .* smaller than the image requires|ImportMemory/i.test(details);
  fail(
    missingPackage
      ? "WebAssembly package missing"
      : (obsoleteWgpuLimit
        ? "Outdated WebGPU limit request"
        : (externalMemoryFailure ? "Browser GPU import failure" : "Browser runtime unavailable")),
    missingPackage
      ? "The generated browser package is missing. Run ./scripts/build_web.sh before serving the website."
      : (obsoleteWgpuLimit
        ? "The loaded package was built before the WebGPU compatibility fix. Rebuild it with ./scripts/build_web.sh, then reload the page."
        : (externalMemoryFailure
          ? browserGpuImportFailureMessage()
          : (error?.message ?? String(error)))),
    details,
  );
});

document.querySelector("[data-copy-source]")?.addEventListener("click", async (event) => {
  await navigator.clipboard.writeText(code.textContent);
  const button = event.currentTarget;
  button.textContent = "Copied";
  setTimeout(() => { button.textContent = "Copy source"; }, 1200);
});
