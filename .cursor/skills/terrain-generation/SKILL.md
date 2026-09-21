---
name: terrain-generation
description: >-
  Change Rust terrain generation without breaking seed determinism. Use when
  editing world/ sampler, grid, noise, biome, shape, elevation, ridge, prng,
  config, seed, LOD, or Tauri generate_* commands.
---

# Terrain generation

Rust is the only source of terrain. Do not move generation logic to JS.

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
  grid.rs       generate_global / region / chunk
  delta.rs      stub for future terrain edits
src-tauri/src/lib.rs   Tauri commands
```

## Determinism

- Input: `seed: u64` + `TerrainGenParams`. The grid must be identical for the same inputs.
- Profile: `ShapeProfile::from_index((seed % 6) as u32)`.
- Moisture and other layers: derived via `prng` (salts like `"moisture"`), not a new global RNG.
- Baseline: seed `6` → Ellipse. After a shape/biome/noise change, indexes `0..5` must still map to the same profiles.

## Where to add what

| Want | Where | Mirror |
| --- | --- | --- |
| Tauri command | `lib.rs` + `invoke_handler` | `src/api/world.ts` |
| Generator param | `TerrainGenParams` + `apply_to` | `src/types/terrainParams.ts` (camelCase) |
| Biome | `biome.rs` (`TileType`) | `src/map/colors.ts`, `src/types/world.d.ts` |
| Shape profile | `shape.rs` + `from_index` `% N` | update docs/rules if N changes |
| Noise helper | `noise.rs` — do not duplicate | — |

Types sent to the frontend: `serde::Serialize`. Grids: `rayon` as in `grid.rs`.

## Tests

`shape.rs` has unit tests for profiles. Run them after shape/prng/config changes:

```bash
cd src-tauri && cargo test --lib
```

Do not weaken existing assertions to make tests pass — that is a determinism regression.
