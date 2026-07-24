#!/usr/bin/env python3
"""Static consistency checks for the Vetrace browser runtime and examples site."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUST_EXAMPLES = ROOT / "vetrace_web/src/wasm/examples.rs"
JS_EXAMPLES = ROOT / "website/assets/examples-data.js"
REQUIRED = [
    ROOT / "website/index.html",
    ROOT / "website/examples/index.html",
    ROOT / "website/examples/play.html",
    ROOT / "website/assets/styles.css",
    ROOT / "website/assets/play.js",
]

errors: list[str] = []
for path in REQUIRED:
    if not path.is_file() or path.stat().st_size == 0:
        errors.append(f"missing or empty required file: {path.relative_to(ROOT)}")

rust_text = RUST_EXAMPLES.read_text(encoding="utf-8")
js_text = JS_EXAMPLES.read_text(encoding="utf-8")
rust_slugs = set(re.findall(r'=> "([a-z0-9-]+)"', rust_text))
# rotating-cube is the fallback match arm rather than an explicit string pattern.
rust_slugs.add("rotating-cube")
js_slugs = set(re.findall(r'slug:\s*"([a-z0-9-]+)"', js_text))

if rust_slugs != js_slugs:
    errors.append(
        "example slugs differ between Rust and website metadata: "
        f"Rust-only={sorted(rust_slugs - js_slugs)}, JS-only={sorted(js_slugs - rust_slugs)}"
    )

play_html = (ROOT / "website/examples/play.html").read_text(encoding="utf-8")
for expected in ("vetrace-canvas", "runtime-status", "data-example-source"):
    if expected not in play_html:
        errors.append(f"play page is missing {expected}")


web_cargo = (ROOT / "vetrace_web/Cargo.toml").read_text(encoding="utf-8")
if 'uuid = { workspace = true, features = ["js"] }' not in web_cargo:
    errors.append("vetrace_web must enable uuid's js feature for browser Actor IDs")
if 'features = ["wgpu_render", "egui_render"]' not in web_cargo:
    errors.append("vetrace_web must enable the shared WGPU renderer and runtime UI overlay")

render_cargo = (ROOT / "vetrace_render/Cargo.toml").read_text(encoding="utf-8")
if 'egui_render = ["wgpu_render"' not in render_cargo:
    errors.append("vetrace_render is missing the platform-neutral egui_render feature")
if 'egui_overlay = ["wgpu_window", "egui_render"]' not in render_cargo:
    errors.append("egui_overlay must preserve its historical native-window behavior")
for clock_manifest in (ROOT / "vetrace_core/Cargo.toml", ROOT / "vetrace_render/Cargo.toml"):
    if 'web-time = "0.2.4"' not in clock_manifest.read_text(encoding="utf-8"):
        errors.append(f"{clock_manifest.relative_to(ROOT)} is missing the browser monotonic clock")

web_renderer = (ROOT / "vetrace_web/src/wasm/renderer.rs").read_text(encoding="utf-8")
if "WgpuRenderer::from_surface" not in web_renderer:
    errors.append("browser adapter is not constructing the shared WgpuRenderer")
if "web_shader.wgsl" in web_renderer or "create_render_pipeline" in web_renderer:
    errors.append("browser adapter must not own a separate rendering pipeline")
web_target = (ROOT / "vetrace_web/src/wasm/target.rs").read_text(encoding="utf-8")
if "impl RenderTarget for WebRenderTarget" not in web_target:
    errors.append("browser canvas adapter is not wired through the normal RenderTarget boundary")
if "RenderPlugin::with_backend" not in web_target:
    errors.append("browser adapter does not expose a normal RenderPlugin")
web_app_runner = (ROOT / "vetrace_web/src/wasm/app_runner.rs").read_text(encoding="utf-8")
if "AppRunner" not in web_app_runner or "request_animation_frame" not in web_app_runner:
    errors.append("browser runtime is not driving the normal AppRunner")
wasm_facade = (ROOT / "vetrace_web/src/wasm.rs").read_text(encoding="utf-8")
if re.search(r"^\s*#\[wasm_bindgen\(start\)\]", wasm_facade, re.MULTILINE):
    errors.append("vetrace_web must not auto-start the gallery inside dependent web games")
play_js = (ROOT / "website/assets/play.js").read_text(encoding="utf-8")
if "start_example" not in play_js:
    errors.append("examples page must explicitly start the selected runtime")
for required_loader_token in ("verifyWebGpu", "verifyRuntimeFile", "window.isSecureContext", "cache: \"no-store\""):
    if required_loader_token not in play_js:
        errors.append(f"examples loader is missing diagnostic/runtime guard: {required_loader_token}")
serve_script = (ROOT / "scripts/serve_web.sh").read_text(encoding="utf-8")
if "build_web.sh" not in serve_script or "serve_web.py" not in serve_script:
    errors.append("serve_web.sh must build a missing runtime and use the no-cache development server")

for stale_api in ("Transform::from_translation", "Shape::cube", "Shape::sphere", "engine.resource::<"):
    if stale_api in js_text:
        errors.append(f"website example source uses a nonexistent or stale API: {stale_api}")

cargo_toml = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
if '"vetrace_web"' not in cargo_toml:
    errors.append("vetrace_web is not a workspace member")

if errors:
    print("Vetrace web validation failed:", file=sys.stderr)
    for error in errors:
        print(f"  - {error}", file=sys.stderr)
    raise SystemExit(1)

print(f"Vetrace web validation passed: {len(js_slugs)} live examples")
