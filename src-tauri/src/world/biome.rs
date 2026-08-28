use crate::world::types::TileType;

const DEEP_WATER: f32 = 0.28;
const WATER: f32 = 0.35;
const COAST: f32 = 0.42;
const UPLAND: f32 = 0.55;
const HIGH: f32 = 0.70;
const SNOW_LINE: f32 = 0.85;

const DRY: f32 = 0.15;
const ARID: f32 = 0.25;
const HUMID: f32 = 0.30;
const WET: f32 = 0.50;
const SWAMP: f32 = 0.55;

pub fn biome(elevation: f32, moisture: f32) -> TileType {
    if elevation < DEEP_WATER {
        return TileType::DeepWater;
    }
    if elevation < WATER {
        return TileType::Water;
    }
    if elevation < COAST {
        if moisture < ARID {
            return TileType::RockyShore;
        }
        return TileType::Sand;
    }
    if elevation >= SNOW_LINE {
        return TileType::Snow;
    }
    if elevation >= HIGH {
        if moisture < ARID {
            return TileType::Tundra;
        }
        return TileType::Mountain;
    }
    if elevation >= UPLAND {
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

    #[test]
    fn water_and_coast() {
        assert_eq!(biome(0.10, 0.50), TileType::DeepWater);
        assert_eq!(biome(0.32, 0.50), TileType::Water);
        assert_eq!(biome(0.38, 0.50), TileType::Sand);
        assert_eq!(biome(0.38, 0.10), TileType::RockyShore);
    }

    #[test]
    fn lowland_moisture_bands() {
        assert_eq!(biome(0.45, 0.10), TileType::Desert);
        assert_eq!(biome(0.45, 0.20), TileType::Savanna);
        assert_eq!(biome(0.45, 0.40), TileType::Grass);
        assert_eq!(biome(0.45, 0.60), TileType::Swamp);
    }

    #[test]
    fn upland_and_high() {
        assert_eq!(biome(0.60, 0.10), TileType::Shrubland);
        assert_eq!(biome(0.60, 0.40), TileType::Forest);
        assert_eq!(biome(0.60, 0.70), TileType::Rainforest);
        assert_eq!(biome(0.75, 0.10), TileType::Tundra);
        assert_eq!(biome(0.75, 0.40), TileType::Mountain);
        assert_eq!(biome(0.90, 0.40), TileType::Snow);
    }
}
