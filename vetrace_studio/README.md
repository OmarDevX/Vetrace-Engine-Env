# Vetrace Studio

`vetrace_studio` is the installed editor application for project-driven Vetrace games.
It is an additional workflow beside native Rust games; it does not replace direct use
of `vetrace_core`, `vetrace_render`, `vetrace_physics`, or the other subsystem crates.

## Run

Open the project manager:

```bash
cargo run -p vetrace_studio
```

Open a project directly:

```bash
cargo run -p vetrace_studio -- examples/lua_runtime_project
```

The current Studio foundation provides:

- project manager with recent-project persistence;
- validated Empty, 3D Starter, and Lua Starter project templates;
- project/main-scene loading without executing project Lua scripts;
- reusable `vetrace_editor` picking, selection outlines, and transform gizmos;
- hierarchy/scene tree;
- fully reflected generic component inspector;
- generic add/remove component operations;
- primitive creation and entity deletion;
- scene saving and reloading;
- project asset browser and console, including captured game stdout/stderr;
- external play/stop through `vetrace-player`;
- generic enum dropdowns sourced from component reflection metadata;
- bounded authored-scene undo/redo (`Ctrl+Z`, `Ctrl+Shift+Z`);
- right-mouse fly camera (`WASDQE`, mouse wheel speed, Shift boost).

The first release intentionally launches both play mode and project switching as
separate processes. This keeps WGPU/winit ownership simple and prevents gameplay
state from corrupting the editor world. Embedded play-in-editor, dock persistence,
and import thumbnails are later phases.
