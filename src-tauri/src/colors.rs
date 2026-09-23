use crate::world::types::TileType;

pub struct BiomeInfo {
    pub label: &'static str,
    pub rgb: [u8; 3],
}

pub const BIOMES: &[(TileType, BiomeInfo)] = &[
    (
        TileType::DeepWater,
        BiomeInfo {
            label: "Głęboka woda",
            rgb: [0x1a, 0x3a, 0x5c],
        },
    ),
    (
        TileType::Water,
        BiomeInfo {
            label: "Woda",
            rgb: [0x2b, 0x6c, 0xb0],
        },
    ),
    (
        TileType::Sand,
        BiomeInfo {
            label: "Piasek",
            rgb: [0xe8, 0xd5, 0xa3],
        },
    ),
    (
        TileType::RockyShore,
        BiomeInfo {
            label: "Skaliste wybrzeże",
            rgb: [0x9a, 0x9a, 0x8a],
        },
    ),
    (
        TileType::Desert,
        BiomeInfo {
            label: "Pustynia",
            rgb: [0xd4, 0xb4, 0x83],
        },
    ),
    (
        TileType::Savanna,
        BiomeInfo {
            label: "Sawanna",
            rgb: [0xb8, 0xa8, 0x4a],
        },
    ),
    (
        TileType::Grass,
        BiomeInfo {
            label: "Trawa",
            rgb: [0x5a, 0xa4, 0x4a],
        },
    ),
    (
        TileType::Swamp,
        BiomeInfo {
            label: "Bagno",
            rgb: [0x3d, 0x5c, 0x3a],
        },
    ),
    (
        TileType::Shrubland,
        BiomeInfo {
            label: "Krzewiasta stepa",
            rgb: [0x8a, 0x9a, 0x4a],
        },
    ),
    (
        TileType::Forest,
        BiomeInfo {
            label: "Las",
            rgb: [0x2d, 0x5a, 0x27],
        },
    ),
    (
        TileType::Rainforest,
        BiomeInfo {
            label: "Las deszczowy",
            rgb: [0x1a, 0x4a, 0x20],
        },
    ),
    (
        TileType::Mountain,
        BiomeInfo {
            label: "Góry",
            rgb: [0x6b, 0x6b, 0x6b],
        },
    ),
    (
        TileType::Tundra,
        BiomeInfo {
            label: "Tundra",
            rgb: [0xb8, 0xc4, 0xb8],
        },
    ),
    (
        TileType::Snow,
        BiomeInfo {
            label: "Śnieg",
            rgb: [0xf0, 0xf0, 0xf5],
        },
    ),
];

pub fn biome_rgb(biome: TileType) -> [u8; 3] {
    for (tile, info) in BIOMES {
        if *tile == biome {
            return info.rgb;
        }
    }
    [0xff, 0x00, 0xff]
}
