# Vetrace Map Builder compatibility launcher

`vetrace_map_builder` is retained only for compatibility with existing scripts
and desktop shortcuts. It launches **Vetrace Studio**, whose scene editing
implementation lives in the shared `vetrace_editor` crate.

```bash
cargo run -p vetrace_map_builder
```

New integrations should launch Studio directly:

```bash
cargo run -p vetrace_studio
```

The old duplicate picking, selection, gizmo, history, scene I/O, and editor UI
implementation has been removed from this crate.
