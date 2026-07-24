SIMPLE SHOOTER MAPS

Save .scene.json maps from vetrace_map_builder into this folder.
Simple Shooter discovers every JSON scene here at startup. These files are the
only playable maps shown in the offline and hosted map selectors.

When hosting, the selected scene and its scene-local textures are transferred
to joining players automatically. Joining players do not need a local copy.

Every map must contain enough Spawn Point objects for its players. Add them
with the Spawn Point button (or the 6 key) in vetrace_map_builder and place
them above collidable ground. Green markers are grounded and red markers are
invalid; both colors are editor-only. The game
raycasts downward, snaps valid points to the ground, rejects points over the
void, and refuses to start a match when there are too few safe points.
