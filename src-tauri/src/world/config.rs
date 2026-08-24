use serde::{Deserialize, Serialize};

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
}

impl ShapeProfile {
  pub fn from_index(index: u32) -> Self {
    match index % 5 {
      0 => Self::Radial,
      1 => Self::Ellipse,
      2 => Self::SquareBump,
      3 => Self::Irregular,
      _ => Self::Archipelago,
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
}

impl Default for WorldConfig {
  fn default() -> Self {
    Self {
      seed: 5,
      world_width: 512,
      world_height: 512,
      region_size: 64,
      macro_resolution: 512,
      micro_resolution: 256,
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
          ShapeProfile::from_index((self.seed % 5) as u32)
      })
  }
}
