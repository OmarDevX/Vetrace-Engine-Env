# ⚙️ Vetracer Engine Macros (`vetrace_engine_macros`)

Procedural macros for [`vetrace_engine`](https://github.com/yourname/vetrace-engine), a modular Rust-based game engine powered by `egui`, `OpenGL`, and `sdl2`.

These macros simplify and automate boilerplate code for components, behaviors, and runtime registration within the Vetrace engine.

---

## ✨ Features

- ✅ `#[export]` attribute macro to automatically expose components or behaviors
- 🧱 Easy integration with Vetrace’s ECS and scene loading system
- 🧰 Helps reduce repetitive code for component factories and editor bindings

---

## 🔧 Usage

Add this to your `Cargo.toml` (for internal use in `vetrace_engine`):

```toml
[dependencies]
vetrace_engine_macros = { path = "../engine_macros" }
