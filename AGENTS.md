# map_tests — agent contract

Procedural world-map preview. **Desktop** app (Tauri 2). Do not run it as a plain web app (`localhost:1420`).

Stack and directories: `.cursor/rules/project-context.mdc`.
Workflows: `.cursor/skills/`.

## Source of truth

- Terrain is computed **only in Rust** (`src-tauri/src/world/`).
- Frontend (`src/`) calls Tauri via `src/api/world.ts` and paints a canvas.
- Same seed + same `TerrainParams` → same world. Always.

## Domain state (current code)

- Seed: `number` / `u64`, not a string. Default: `6`.
- Shape profile: `seed % 6` → Radial, Ellipse, SquareBump, Irregular, Archipelago, Continents.
- Three LOD levels: global (`generate_global`) → region → chunk.
- Terrain params: `TerrainParams` (TS, camelCase) ↔ `TerrainGenParams` (Rust, serde camelCase).
- `simplex-noise` in JS is legacy — new generation belongs in Rust (`noise` + `rayon`).

## Change flow

1. State assumptions and ambiguities — ask, do not guess.
2. Plan steps + success criteria + file list.
3. Show the plan to the user **before** editing code.
4. Minimal diff. Do not clean adjacent code. Do not add packages.
5. Clean up your own mess: no unused imports/vars/types/files, no commented-out code, no debug logs.
6. Verify: linter on touched files, seed determinism, existing patterns, UI in the Tauri window (not the browser).

Skill: `.cursor/skills/change-flow/SKILL.md`.

## Patterns

- New Tauri command: `lib.rs` (`invoke_handler`) + wrapper in `src/api/world.ts`.
- New biome: `world/biome.rs` + color in `src/map/colors.ts` (`TileType` must match).
- New shape profile: `world/shape.rs` + `ShapeProfile::from_index` (`% N`).
- One hook per feature: `useWorldMap` / `useRegionMap` / `useChunkMap`.
- Views: `GlobalMapView` → `RegionMapView` → `ChunkMapView`.

Skills: `terrain-generation`, `map-frontend`.

## Hard limits

- Do not add npm/cargo dependencies. Extend what exists: Tauri 2, React 19, `noise`, `rayon`, `serde`.
- Leave no artifacts: unused vars/imports, dead exports, commented-out code, debug logs, `allow(dead_code)` instead of deletion.
- Code comments: English. Agent files (`AGENTS.md`, `.cursor/rules`, `.cursor/skills`): English. README and `docs/`: Polish.
- After a generation change, baseline seed `6` (Ellipse). Other `seed % 6` values must keep the same profile.
