# Vetrace Editor

This crate provides the runtime editor plugin for Vetrace.

## Use

```rust
use vetrace_core::AppBuilder;
use vetrace_render::RenderPlugin;
use vetrace_editor::EditorPlugin;

AppBuilder::new()
    .add_plugin(RenderPlugin::new())
    .add_plugin(EditorPlugin::new())
    .run_until_stopped(MyApp, None, 1.0 / 60.0)?;
```

## Current controls

- Left click: pick/select render entity
- Tab / Shift+Tab: cycle selection
- Escape: clear selection
- Delete / Backspace: delete selected entity
- G or 1: translate mode
- R or 2: rotate mode
- F or 3: scale mode
- T or 4: combined transform mode
- X: switch between global and local axes
- P: switch the multi-selection pivot
- Arrow keys or WASD: edit selected entity on X/Z
- Q / E: edit selected entity down/up
- Shift: faster editing
