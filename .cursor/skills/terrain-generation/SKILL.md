---
name: terrain-generation
description: >-
  Change Rust terrain generation without breaking seed determinism. Use when
  editing world/ sampler, grid, noise, biome, shape, elevation, ridge, prng,
  config, seed, LOD, or grid generate_* APIs.
---

# Terrain generation

Rust is the only source of terrain. Do not move generation logic into the UI.

## Module map

```
src-tauri/src/world/
  config.rs     WorldConfig, ShapeProfile, TerrainGenParams, LOD
  types.rs      TileType, TerrainCell, TerrainGrid, RegionId, ChunkId
  prng.rs       hash / derive seed (u64)
  noise.rs      fBm, clamp, lerp, redistribution
  shape.rs      6 land profiles
  elevation.rs  relief belts / orogeny
  ridge.rs      ridges
  biome.rs      TileType classification
  sampler.rs    sample a point
  grid.rs       generate_global / region / chunk / view
  delta.rs      stub for future terrain edits
src-tauri/src/app.rs              egui UI (calls world::*)
src-tauri/src/colors.rs           biome colors
src-tauri/src/render.rs           compose ColorImage
src-tauri/src/settlements_draw.rs settlements + roads overlay
src-tauri/src/ecology_draw.rs     habitat wash + life summary
```

## Determinism

- Input: `seed: u64` + `TerrainGenParams`. The grid must be identical for the same inputs.
- Profile: `ShapeProfile::from_index((seed % 6) as u32)`.
- Moisture and other layers: derived via `prng` (salts like `"moisture"`), not a new global RNG.
- Baseline: seed `6` → Ellipse. After a shape/biome/noise change, indexes `0..5` must still map to the same profiles.

## Where to add what

| Want | Where | Mirror |
| --- | --- | --- |
| Generator param | `TerrainGenParams` + `apply_to` | egui sliders in `app.rs` if exposed |
| Biome | `biome.rs` (`TileType`) | `colors.rs` |
| Shape profile | `shape.rs` + `from_index` `% N` | update docs/rules if N changes |
| Noise helper | `noise.rs` — do not duplicate | — |

Grids: `rayon` as in `grid.rs`.

## Tests

```bash
cd src-tauri && cargo test --lib
```

Do not weaken existing assertions to make tests pass — that is a determinism regression.
