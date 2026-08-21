mod world;

use std::sync::Mutex;
use tauri::State;
use world::config::WorldConfig;
use world::grid::{generate_global_grid, generate_region_grid};
use world::types::{RegionId, TerrainGrid};

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

#[tauri::command]
fn generate_global(state: State<'_, Mutex<WorldState>>, seed: String) -> TerrainGrid {
	let mut guard = state.lock().unwrap();
	guard.config.seed = seed;
	let grid = generate_global_grid(guard.config.clone());
	guard.global_cache = Some(grid.clone());
	grid
}

#[tauri::command]
fn generate_region(
	state: State<'_, Mutex<WorldState>>,
	seed: String,
	rx: u32,
	ry: u32,
) -> TerrainGrid {
	let guard = state.lock().unwrap();
	let mut config = guard.config.clone();
	config.seed = seed;
	generate_region_grid(config, RegionId { rx, ry })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.manage(Mutex::new(WorldState::default()))
		.invoke_handler(tauri::generate_handler![generate_global, generate_region])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
