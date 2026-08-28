use crate::world::types::TileType;

pub fn biome(elevation: f32, moisture: f32) -> TileType {
    if elevation < 0.30 {
        return TileType::DeepWater;
    }
    if elevation < 0.35 {
        return TileType::Water;
    }
    if elevation < 0.40 {
        return TileType::Sand;
    }
    if elevation >= 0.85 {
        return TileType::Snow;
    }
    if elevation >= 0.70 {
        return TileType::Mountain;
    }
    if moisture >= 0.30 {
        TileType::Forest
    } else {
        TileType::Grass
    }
}
