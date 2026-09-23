# map_tests — agent contract

Procedural world-map preview. **Desktop** app (Rust + egui). No web frontend.

Stack and directories: `.cursor/rules/project-context.mdc`.
Workflows: `.cursor/skills/`.

## Source of truth

- Terrain is computed **only in** `src-tauri/src/world/`.
- UI lives in `src-tauri/src/app.rs` (egui) and paints textures from `render.rs`.
- Same seed + same `TerrainGenParams` → same world. Always.

## Domain state (current code)

- Seed: `u64`. Default: `6`.
- Shape profile: `seed % 6` → Radial, Ellipse, SquareBump, Irregular, Archipelago, Continents.
- Three LOD levels: global → region → chunk.
- Terrain params: `TerrainGenParams` (serde camelCase) applied via `apply_to` on `WorldConfig`.

## Change flow

1. State assumptions and ambiguities — ask, do not guess.
2. Plan steps + success criteria + file list.
3. Show the plan to the user **before** editing code.
4. Minimal diff. Do not clean adjacent code. Do not add packages unless the task requires a new UI/runtime capability.
5. Clean up your own mess: no unused imports/vars/types/files, no commented-out code, no debug logs.
6. Verify: `cargo test --lib` in `src-tauri`, seed determinism, run the binary (`cargo run` in `src-tauri`).

Skill: `.cursor/skills/change-flow/SKILL.md`.

## Patterns

- New biome: `world/biome.rs` + color in `colors.rs` (`TileType` must match).
- New shape profile: `world/shape.rs` + `ShapeProfile::from_index` (`% N`).
- UI / LOD: `app.rs`. Compose texture: `render.rs`. Settlements/roads: `settlements_draw.rs`. Ecology wash: `ecology_draw.rs`.

Skills: `terrain-generation`, `map-ui`.

## Hard limits

- Prefer existing crates: `noise`, `rayon`, `serde`, `eframe`/`egui`.
- Leave no artifacts: unused vars/imports, dead exports, commented-out code, debug logs, `allow(dead_code)` instead of deletion.
- Code comments: English. Agent files (`AGENTS.md`, `.cursor/rules`, `.cursor/skills`): English. README and `docs/`: Polish.
- After a generation change, baseline seed `6` (Ellipse). Other `seed % 6` values must keep the same profile.
