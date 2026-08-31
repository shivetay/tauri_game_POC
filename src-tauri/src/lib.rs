mod world;

use std::sync::Mutex;
use tauri::State;
use world::config::{TerrainGenParams, WorldConfig};
use world::grid::{generate_chunk_grid, generate_global_grid, generate_region_grid};
use world::types::{ChunkId, RegionId, TerrainGrid};

struct WorldState {
    config: WorldConfig,
    global_cache: Option<TerrainGrid>,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            config: WorldConfig::default(),
            global_cache: None,
        }
    }
}

fn config_with(seed: u64, params: TerrainGenParams) -> WorldConfig {
    let mut config = WorldConfig::default();
    config.seed = seed;
    params.apply_to(&mut config);
    config
}

#[tauri::command]
fn generate_global(
    state: State<'_, Mutex<WorldState>>,
    seed: u64,
    params: TerrainGenParams,
) -> TerrainGrid {
    let mut guard = state.lock().unwrap();
    let config = config_with(seed, params);
    guard.config = config.clone();
    let grid = generate_global_grid(config);
    guard.global_cache = Some(grid.clone());
    grid
}

#[tauri::command]
fn generate_region(
    _state: State<'_, Mutex<WorldState>>,
    seed: u64,
    params: TerrainGenParams,
    rx: u32,
    ry: u32,
) -> TerrainGrid {
    generate_region_grid(config_with(seed, params), RegionId { rx, ry })
}

#[tauri::command]
fn generate_chunk(
    _state: State<'_, Mutex<WorldState>>,
    seed: u64,
    params: TerrainGenParams,
    rx: u32,
    ry: u32,
    cx: u32,
    cy: u32,
) -> TerrainGrid {
    generate_chunk_grid(
        config_with(seed, params),
        RegionId { rx, ry },
        ChunkId { cx, cy },
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(WorldState::default()))
        .invoke_handler(tauri::generate_handler![generate_global, generate_region, generate_chunk])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
