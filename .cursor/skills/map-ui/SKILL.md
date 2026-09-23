---
name: map-ui
description: >-
  Change the egui map UI, textures, biome colors, settlement/ecology overlays,
  or LOD views. Use when editing app.rs, colors.rs, render.rs,
  settlements_draw.rs, ecology_draw.rs, or map controls.
---

# Map UI (egui)

The UI does not generate terrain. It calls `world::*` and paints.

## Layers

```
src-tauri/src/app.rs               egui shell, seed/params, LOD, legends, clicks
src-tauri/src/render.rs            compose ColorImage (terrain + overlays)
src-tauri/src/colors.rs            biome colors = TileType
src-tauri/src/settlements_draw.rs  roads + settlement markers/plans (RGBA)
src-tauri/src/ecology_draw.rs      habitat wash + life_area_summary
src-tauri/src/world/               generation (source of truth)
```

## UI flow

`MapApp` holds seed + draft params + LOD (`Global` / `Region` / `Chunk`).

| LOD | Terrain | Settlements | Ecology | Draw style |
| --- | --- | --- | --- | --- |
| Global | `generate_global_grid` | City/Town markers, Highway roads | — | marker + main roads |
| Region | `generate_region_grid` | all plans + roads | `generate_region_ecology` | plan + region roads |
| Chunk | `generate_view_grid` | all plans + roads | `generate_chunk_ecology` | plan + close roads; click = biome/species |

Generation runs on a **background thread**; results apply when `request_id` matches. Settlements regenerate only when seed/params change; LOD zoom reuses them.

## Click rules (parity with former React UI)

1. Hit settlement → show label, do **not** zoom.
2. Else if Global/Region → zoom into region/chunk.
3. Else (Chunk) → biome + nearby flora/fauna summary.

## Conventions

- Heavy work off the UI thread
- Every `TileType` has an entry in `BIOMES`
- Settlement/road/district legend colors live in `settlements_draw.rs`
- LOD sizes follow `WorldConfig` defaults (512 / 512 / 256, region 64, chunk 8)
- Run: `cargo run` in `src-tauri`

## Seed in the UI

- Generator state: `u64`
- Draft seed may stay a string until Regenerate
- Regenerating resets LOD to global
