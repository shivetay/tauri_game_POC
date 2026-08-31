use crate::world::elevation::{
    COAST_M, DEEP_WATER_M, HIGH_M, SNOW_LINE_M, UPLAND_M, WATER_M,
};
use crate::world::types::TileType;

const DRY: f32 = 0.15;
const ARID: f32 = 0.25;
const HUMID: f32 = 0.30;
const WET: f32 = 0.50;
const SWAMP: f32 = 0.55;

pub fn biome(elevation_m: f32, moisture: f32) -> TileType {
    if elevation_m < DEEP_WATER_M {
        return TileType::DeepWater;
    }
    if elevation_m < WATER_M {
        return TileType::Water;
    }
    if elevation_m < COAST_M {
        if moisture < ARID {
            return TileType::RockyShore;
        }
        return TileType::Sand;
    }
    if elevation_m >= SNOW_LINE_M {
        return TileType::Snow;
    }
    if elevation_m >= HIGH_M {
        if moisture < ARID {
            return TileType::Tundra;
        }
        return TileType::Mountain;
    }
    if elevation_m >= UPLAND_M {
        if moisture < ARID {
            return TileType::Shrubland;
        }
        if moisture < WET {
            return TileType::Forest;
        }
        return TileType::Rainforest;
    }
    if moisture < DRY {
        return TileType::Desert;
    }
    if moisture < HUMID {
        return TileType::Savanna;
    }
    if moisture < SWAMP {
        return TileType::Grass;
    }
    TileType::Swamp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::elevation::norm_to_meters;

    #[test]
    fn water_and_coast() {
        assert_eq!(biome(norm_to_meters(0.10), 0.50), TileType::DeepWater);
        assert_eq!(biome(norm_to_meters(0.32), 0.50), TileType::Water);
        assert_eq!(biome(norm_to_meters(0.38), 0.50), TileType::Sand);
        assert_eq!(biome(norm_to_meters(0.38), 0.10), TileType::RockyShore);
    }

    #[test]
    fn lowland_moisture_bands() {
        assert_eq!(biome(norm_to_meters(0.45), 0.10), TileType::Desert);
        assert_eq!(biome(norm_to_meters(0.45), 0.20), TileType::Savanna);
        assert_eq!(biome(norm_to_meters(0.45), 0.40), TileType::Grass);
        assert_eq!(biome(norm_to_meters(0.45), 0.60), TileType::Swamp);
    }

    #[test]
    fn upland_and_high() {
        assert_eq!(biome(norm_to_meters(0.60), 0.10), TileType::Shrubland);
        assert_eq!(biome(norm_to_meters(0.60), 0.40), TileType::Forest);
        assert_eq!(biome(norm_to_meters(0.60), 0.70), TileType::Rainforest);
        assert_eq!(biome(norm_to_meters(0.75), 0.10), TileType::Tundra);
        assert_eq!(biome(norm_to_meters(0.75), 0.40), TileType::Mountain);
        assert_eq!(biome(norm_to_meters(0.90), 0.40), TileType::Snow);
    }
}
