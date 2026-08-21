use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TileType {
  DeepWater = 0,
  Water = 1,
  Sand = 2,
  Grass = 3,
  Forest = 4,
  Mountain = 5,
  Snow = 6,
  Rock = 7,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainCell {
  pub elevation: f32,
  pub moisture: f32,
  pub biome: TileType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainGrid {
  pub width: u32,
  pub height: u32,
  pub cells: Vec<TerrainCell>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RegionId {
    pub rx: u32,
    pub ry: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct RegionBounds {
    pub world_x0: f32,
    pub world_y0: f32,
    pub world_x1: f32,
    pub world_y1: f32,
}

impl RegionBounds {
  pub fn from_region(id: RegionId, region_size: u32) -> Self {
    let x0 = id.rx as f32 * region_size as f32;
    let y0 = id.ry as f32 * region_size as f32;
    Self {
      world_x0: x0,
      world_y0: y0,
      world_x1: x0 + region_size as f32,
      world_y1: y0 + region_size as f32,
    }
  }
}