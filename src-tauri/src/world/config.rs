use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainGenParams {
    pub orogeny_strength: f32,
    pub moisture_strength: f32,
    pub detail_strength: f32,
    pub terrain_roughness: f32,
    pub land_size: f32,
    pub coast_distortion: f32,
}

impl Default for TerrainGenParams {
    fn default() -> Self {
        Self {
            orogeny_strength: 1.0,
            moisture_strength: 1.0,
            detail_strength: 1.0,
            terrain_roughness: 1.0,
            land_size: 1.0,
            coast_distortion: 1.0,
        }
    }
}

impl TerrainGenParams {
    pub fn apply_to(self, config: &mut WorldConfig) {
        config.orogeny_strength = self.orogeny_strength.clamp(0.0, 1.0);
        config.moisture_strength = self.moisture_strength.clamp(0.0, 1.0);
        config.detail_strength = self.detail_strength.clamp(0.0, 1.0) * 0.08;
        config.terrain_roughness = self.terrain_roughness.clamp(0.0, 1.0);
        config.land_size = self.land_size.clamp(0.0, 1.0);
        config.coast_distortion = self.coast_distortion.clamp(0.0, 1.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LodLevel {
  Macro,
  Micro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ShapeProfile {
    Radial,
    Ellipse,
    SquareBump,
    Irregular,
    Archipelago,
    Continents,
}

impl ShapeProfile {
  pub fn from_index(index: u32) -> Self {
    match index % 6 {
      0 => Self::Radial,
      1 => Self::Ellipse,
      2 => Self::SquareBump,
      3 => Self::Irregular,
      4 => Self::Archipelago,
      _ => Self::Continents,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldConfig {
    pub seed: u64,
    pub world_width: u32,
    pub world_height: u32,
    pub region_size: u32,
    pub macro_resolution: u32,
    pub micro_resolution: u32,
    pub chunk_size: u32,
    pub chunk_resolution: u32,
    pub scale: f32,
    pub octaves: u32,
    pub persistence: f32,
    pub lacunarity: f32,
    pub island_mix: f32,
    pub redistribution_exponent: f32,
    pub redistribution_fudge: f32,
    pub detail_octaves: u32,
    pub detail_strength: f32,
    pub shape_profile: Option<ShapeProfile>,
    /// 0 = flat relief, 1 = full seed-driven belts (uplift + depressions).
    pub orogeny_strength: f32,
    /// 0 = uniform moisture, 1 = full variation.
    pub moisture_strength: f32,
    /// 0 = smooth base terrain, 1 = full hills/valleys.
    pub terrain_roughness: f32,
    /// 0 = less land, 1 = more land.
    pub land_size: f32,
    /// 0 = smooth coasts, 1 = full coastline warp.
    pub coast_distortion: f32,
}

impl Default for WorldConfig {
  fn default() -> Self {
    Self {
      seed: 6,
      world_width: 512,
      world_height: 512,
      region_size: 64,
      macro_resolution: 512,
      micro_resolution: 512,
      chunk_size: 8,
      chunk_resolution: 256,
      scale: 1.8,
      octaves: 6,
      persistence: 0.5,
      lacunarity: 2.0,
      island_mix: 0.75,
      redistribution_exponent: 2.2,
      redistribution_fudge: 1.1,
      detail_octaves: 4,
      detail_strength: 0.08,
      shape_profile: None,
      orogeny_strength: 1.0,
      moisture_strength: 1.0,
      terrain_roughness: 1.0,
      land_size: 1.0,
      coast_distortion: 1.0,
    }
  }
}

impl WorldConfig {
  pub fn with_seed(mut self, seed: u64) -> Self {
      self.seed = seed;
      self
  }

  pub fn resolved_shape_profile(&self) -> ShapeProfile {
      self.shape_profile.unwrap_or_else(|| {
          ShapeProfile::from_index((self.seed % 6) as u32)
      })
  }
}
