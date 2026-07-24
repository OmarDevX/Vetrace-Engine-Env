SIMPLE SHOOTER WEAPON MODDING

Every JSON file in this directory is loaded into the weapon registry. Each file
must have a unique id. Players store an EquippedWeapon id and multiplayer
snapshots replicate that id. Use equip_weapon(engine, player, "compact_pistol")
from game-side loadout, pickup, or mod code. The game validates unsafe values,
skips invalid files, and always provides a built-in rifle fallback.

gameplay
  damage, cooldown_seconds, range
  aim_mode:
    crosshair_converge - camera selects the target, then the real shot travels
                        from the gun muzzle to that point
    barrel_forward    - shot follows the gun's local -Z axis exactly

attachment
  position            - gun root from the eye in gun-local coordinates
  muzzle              - physical bullet/tracer/sound/flash origin
  rotation_degrees    - aim-relative gun and barrel rotation
  first_person_position changes only the camera view model. It cannot move the
                        authoritative muzzle or alter hit detection.

model
  path                 - GLB/GLTF path relative to simple shooter/assets, or
                         null for the built-in procedural rifle
  position/rotation/scale transform either model type
  body/barrel/stock/grip sizes and PBR colors tune the procedural rifle
  Custom models require building Simple Shooter with --features gltf.
  Model nodes are render-only and never receive gameplay or physics components.

tracer, muzzle_flash, sound
  Control each shot's visible 3D path, muzzle burst, lifetime, colors, emissive
  strength, sound asset, volume, and spatial range independently.
  light_intensity/light_range make the emissive effect illuminate nearby PBR
  geometry. Tracer light_samples distributes 1..4 cheap point-light samples
  along the visible shot path; two is the recommended default.

Coordinate convention: +X right, +Y up, -Z forward. Keep the muzzle near the
visible barrel tip. In multiplayer the server remains authoritative for damage,
range, and fire rate; shot snapshots carry the actual muzzle-to-impact path so
every client renders the same origin and endpoint.

MULTIPLAYER COMPATIBILITY

The registry computes a deterministic hash from weapon ids plus damage, range,
cooldown, and aim mode. Clients with different gameplay hashes are rejected
with a clear message. Models, colors, tracers, flashes, and sounds are excluded
from this hash, so client-side visual replacements remain possible.
