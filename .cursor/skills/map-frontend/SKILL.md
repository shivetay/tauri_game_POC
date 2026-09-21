---
name: map-frontend
description: >-
  Change the React map UI, canvas rendering, Tauri wrappers, or LOD views.
  Use when editing src/ components, hooks, api/world.ts, map/render, colors,
  SeedControl, or global/region/chunk map flow.
---

# Map frontend

The frontend does not generate terrain. Only `invoke` + drawing.

## Layers

```
src/api/world.ts          invoke wrappers + pixel → region/chunk mapping
src/hooks/                useWorldMap, useRegionMap, useChunkMap
src/components/Map/       GlobalMapView, RegionMapView, ChunkMapView, GridMapView
src/components/Control/   SeedControl, BiomeLegend
src/map/render.ts         TerrainGrid → ImageData
src/map/colors.ts         biome colors = TileType from Rust
src/types/world.d.ts      grid, biome, ids
src/types/terrainParams.ts  mirror of TerrainGenParams
src/utils/constants.ts    REGION_SIZE, CHUNK_SIZE, resolutions
```

## UI flow

`App.tsx` holds seed (`number`) + draft (input string) + `TerrainParams` + selected region/chunk.

1. No region → `GlobalMapView` (`generate_global`)
2. Region → `RegionMapView` (`generate_region`)
3. Region + chunk → `ChunkMapView` (`generate_chunk`)

A new LOD level = a new hook in `src/hooks/` + a view in `src/components/Map/` + a command that already exists or is added on the Rust side.

## Conventions

- Tauri calls only through `src/api/world.ts`. Call `assertTauri()` first.
- One hook per feature. Do not `invoke` from a component.
- Functional components, TypeScript strict, no `any`.
- Keep LOD constants in `constants.ts` in sync with `WorldConfig` defaults (512 / 512 / 256, region 64, chunk 8).
- Colors: every `TileType` must have an entry in `BIOMES`.
- The app runs only in the Tauri window (`pnpm tauri dev`). Do not test as Vite-only in the browser.

## Seed in the UI

- Generator state: `number` (non-negative safe integer).
- The input may stay a string (`draftSeed`) until Regenerate.
- Regenerating clears the selected region and chunk.
