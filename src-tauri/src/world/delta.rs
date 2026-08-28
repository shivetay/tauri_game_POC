use serde::{Deserialize, Serialize};

use crate::world::types::TileType;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainDelta {
    pub world_x: u32,
    pub world_y: u32,
    pub elevation_override: Option<f32>,
    pub biome_override: Option<TileType>,
    pub source_event_id: String,
}

pub fn apply_delta_elevation(base: f32, deltas: &[TerrainDelta], x: u32, y: u32) -> f32 {
    deltas
        .iter()
        .find(|delta| delta.world_x == x && delta.world_y == y)
        .and_then(|delta| delta.elevation_override)
        .unwrap_or(base)
}
