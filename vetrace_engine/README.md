# 🛠️ Vetrace Engine (`vetrace_engine`)

An experimental **raytracing-capable game engine** written in Rust, powered by:

- `egui` for fast & native GUI rendering
- `wgpu` compute shaders for raycasting and post processing
- A hybrid rasterization/raytracing pipeline using WGSL shaders
- `sdl2` for windowing, input, and graphics context
- An Entity-Component-System (ECS) core
- Custom macro support via `#[export]` procedural macros

---

## 🚀 Features

- ✅ Real-time raytracing using wgpu compute shaders
- 🖼️ `Sprite3D` converts textures into textured quad meshes for full raytraced lighting
- 🧠 Modular ECS design
- 🎛️ Integrated GUI using `egui`
- 🖱️ SDL2 input and window management
- 🔧 Extensible via component factories and behaviors
- 📝 Lua and Rust behaviour scripting
- 🛠️ 3D physics and collision detection (Rapier)
- 🧩 Built-in scene loading (`ron`-based)
- 🧰 Easy to integrate into your own game project

---

## 📦 Usage

Add this to your `Cargo.toml` (after it's published):

```toml
[dependencies]
vetrace_vengine = "0.1"

```

Examples expect a `tree.png` texture under `assets/textures`. This file is not
included in the repository, so create the directory and provide your own image
before running the sprite example. Textures are loaded with the `image` crate
and converted to RGBA internally, so formats like PNG or JPEG will work.
If the file is missing the engine will panic with an `IoError` (`No such file or
directory`). A corrupt or unsupported image instead produces a `DecodingError`.
The sprite example shows a simple textured quad rendered through the standard
mesh pipeline so it receives lighting, shadows and global illumination like any
other object.

See [ENGINE_GUIDE.md](ENGINE_GUIDE.md) for detailed documentation on how the engine works and how to extend it.

### glTF PBR Example

A minimal example demonstrating the PBR asset pipeline is provided in
`examples/pbr_cat.rs`. Place `cat.gltf` (and its accompanying resources)
inside the `assets/` directory before running the example. It will load that
file, upload its textures and meshes to the GPU, and render the model with its
base-color material.

Run it with:

```sh
cargo run --example pbr_cat
```

The example uses `wgpu` and requires a GPU with WebGPU support.
