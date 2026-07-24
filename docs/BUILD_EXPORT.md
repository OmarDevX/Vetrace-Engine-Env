# Build and export

Vetrace exports project-driven games without compiling Rust. The exporter copies a prebuilt `vetrace-player` template and packages the project into a versioned `game.vpak` archive.

## Studio

Open the **Build** tab, choose an export preset and player template, then click **Export project**. Studio validates the scene and asset database before starting an asynchronous export.

The default output is:

```text
builds/desktop/
├── <GameName>[.exe]
├── game.vpak
├── build-report.json
└── licenses/          # copied when present beside the player template
```

The exported player automatically discovers a single sidecar `.vpak`, `game.vpak`, or a package matching its executable name.

## Command line

Build `vetrace-player` once, then export without invoking Cargo:

```bash
vetrace-build \
  --player-template target/release/vetrace-player \
  --preset Desktop \
  /path/to/project
```

`VETRACE_PLAYER_TEMPLATE` may be used instead of `--player-template`.

Run a package directly:

```bash
vetrace-player --package builds/desktop/game.vpak
```

## Export presets

Presets are stored in `export.vetrace.toml` at the project root. Output directories must remain beneath `builds/`.

```toml
format_version = 1
default_preset = "Desktop"

[[presets]]
name = "Desktop"
target = "host"
output_directory = "builds/desktop"
executable_name = ""
package_name = "game.vpak"
compression = "deflate"
include_asset_database = true
```

Cross-platform export is template-driven: supply a prebuilt player for the target platform. The exporter does not compile or verify foreign binaries.

## Package integrity

A `.vpak` contains the project manifest, source assets, optional stable asset database, and `vpak.json`. Every declared file has a size and BLAKE3 digest. Package mounting rejects traversal paths, undeclared files, duplicates, missing files, invalid hashes, and oversized metadata before loading the project.
