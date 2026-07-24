# Runtime UI

`vetrace_ui` contains renderer-neutral widget data. Games compose these
components with `vetrace_render::ScreenSpaceRect` for placement and keep page,
navigation, and gameplay policy on the game side.

## Styling

Attach `UIVisualStyle` beside a `UIPanel`, `UIButton`, `UITextEditor`, or
`ColorRect` to customize corner radius, border, shadow, text, and hover/pressed
response. The component is optional, so existing UI keeps sensible defaults.

```rust
let mut style = vetrace_ui::UIVisualStyle::rounded(14.0)
    .with_border(1.0, Vec3::new(0.2, 0.5, 0.9), 0.8)
    .with_shadow(Vec2::new(0.0, 6.0), Vec3::ZERO, 0.5);
style.font_size = 20.0;
```

## Input geometry

`screen_rect_bounds`, `screen_rect_contains`, and `pointer_interaction` provide
the same normalized-anchor/pixel-offset geometry used by screen UI rendering.
They do not impose an input system or callback model: the game reads its input,
uses these helpers, then updates `UIButton::hovered` and `UIButton::pressed` or
dispatches its own typed action.

## Simple Shooter example

The Simple Shooter front end demonstrates:

- a responsive top navigation and bottom-right Play/Maps actions;
- a LAN server browser with Host, Refresh, selection, and Join actions;
- an authoritative combat lobby with host-only rules and a kills leaderboard;
- styled pages for maps, mods, and settings;
- an invisible UI hit target over a rotating 3D actor;
- game-side gradient material randomization;
- graphics and post-processing controls, including volumetric fog and vignette;
- a clean transition from menu to offline play or the server lobby flow.

Run it with `cargo run -p simple_shooter`.
