# Render-to-texture cameras

Vetrace exposes generic secondary raster cameras to game-side custom materials.
The renderer does not contain mirror, portal, minimap, scope, or security-camera
logic. It only:

1. Rasterizes enabled `RenderTextureCamera` entities into named GPU textures.
2. Makes up to four named textures available to each `CustomShaderMaterial`.
3. Prevents a surface from sampling the same texture currently used as its
   render attachment.
4. Applies optional render-layer filtering.

This does not use a BVH or hardware ray tracing. Each enabled camera performs an
additional raster scene pass every frame.

## Create a secondary camera

The camera position and orientation come from the entity's `Transform`. Local
`-Z` is forward and local `+Y` is up.

```rust
engine
    .spawn_actor("Mirror camera")
    .with(Transform {
        translation: reflected_position,
        rotation: reflected_rotation,
        scale: Vec3::ONE,
    })
    .with(RenderTextureCamera {
        target_name: "mirror_view".into(),
        width: 1024,
        height: 1024,
        layer_mask: u32::MAX & !MIRROR_SURFACE_LAYER,
        ..RenderTextureCamera::default()
    })
    .build();
```

Lower `order` values render first. This matters when one render texture samples
another. Cyclic dependencies intentionally see a previous-frame result.

## Bind a view to a custom material

```rust
CustomShaderMaterial {
    shader_id: "game/planar_mirror".into(),
    wgsl_source: Some(MIRROR_WGSL.into()),
    render_textures: vec!["mirror_view".into()],
    ..CustomShaderMaterial::default()
}
```

The fixed slots are:

| Material index | WGSL binding |
|---|---:|
| `render_textures[0]` | group 0, binding 11 |
| `render_textures[1]` | group 0, binding 12 |
| `render_textures[2]` | group 0, binding 13 |
| `render_textures[3]` | group 0, binding 14 |

They reuse the normal material filtering sampler at group 0, binding 2. Missing names
are bound to a black fallback texture.

```wgsl
@group(0) @binding(2)
var render_texture_sampler: sampler;

@group(0) @binding(11)
var mirror_view: texture_2d<f32>;

let reflected_color = textureSample(mirror_view, render_texture_sampler, mirror_uv);
```

The receiver is automatically omitted from a camera pass when its
`render_textures` list contains that camera's `target_name`. This prevents an
illegal texture read/write feedback loop and normally removes the mirror or
portal surface from its own image.

## Render layers

Entities without `RenderLayers` use all bits. A secondary camera draws an object
when:

```text
object_layers & camera_layer_mask != 0
```

```rust
mirror_actor.with(RenderLayers { mask: MIRROR_SURFACE_LAYER });
```

Render layers are generic and can also hide first-person weapons, editor-only
helpers, world-space UI, or other objects from a secondary view.

## Planar mirror game logic

For a mirror plane with point `plane_point` and unit normal `n`, reflect the main
camera position and its orientation vectors in game code:

```rust
fn reflect_point(point: Vec3, plane_point: Vec3, n: Vec3) -> Vec3 {
    point - 2.0 * (point - plane_point).dot(n) * n
}

fn reflect_vector(vector: Vec3, n: Vec3) -> Vec3 {
    vector - 2.0 * vector.dot(n) * n
}
```

Update the `RenderTextureCamera` entity transform each frame, then let the mirror
custom shader project and sample the named image. Oblique near-plane clipping is
not built in yet; place the secondary camera slightly behind the mirror plane or
perform clipping in the mirror shader when needed.

## Current scope

- Scene meshes, PBR textures, glTF assets, baked GI, dynamic lights, and the
  existing directional shadow data are available in render-texture passes.
- Screen overlays, egui, SSAO, and custom full-screen post-process chains are not
  rendered into secondary targets.
- Secondary views currently reuse the main view's directional shadow cascades.
- There is no recursion limit because direct self-sampling is excluded. Multiple
  portals can be ordered, while true recursive portals require ping-pong targets
  or an explicit recursion policy.

See `examples/render_texture_portal.rs` and
`examples/render_texture_portal.wgsl` for a complete game-side example.

## Custom shader vertex interface

Render-texture materials normally need mesh UVs. Existing custom shaders remain
backward compatible and receive only locations 0 and 1. Opt into the full shared
vertex interface explicitly:

```rust
CustomShaderMaterial {
    vertex_interface: CustomShaderVertexInterface::Textured,
    ..Default::default()
}
```

The full fragment input contract is:

```wgsl
@location(0) world_position: vec3<f32>,
@location(1) normal: vec3<f32>,
@location(2) uv: vec2<f32>,
@location(3) color: vec4<f32>,
@location(4) tangent: vec4<f32>,
@location(5) lightmap_uv: vec2<f32>,
```


### Custom shader vertex interfaces

WGPU 0.20 requires the vertex outputs and fragment inputs to match exactly.
Choose the smallest interface your shader consumes:

- `Legacy`: locations 0–1 (`world_position`, `normal`)
- `Textured`: locations 0–2 (adds `uv`)
- `Full`: locations 0–5 (adds `color`, `tangent`, `lightmap_uv`)

A `Full` fragment shader must declare all six locations even when some fields
are unused. Portal, screen, and planar-mirror materials normally use `Textured`.
