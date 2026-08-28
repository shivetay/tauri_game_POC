import type { TileType } from "../types/world";

export interface BiomeInfo {
	label: string;
	rgb: [number, number, number];
}

export const BIOMES: Record<TileType, BiomeInfo> = {
	DeepWater: { label: "Głęboka woda", rgb: [0x1a, 0x3a, 0x5c] },
	Water: { label: "Woda", rgb: [0x2b, 0x6c, 0xb0] },
	Sand: { label: "Piasek", rgb: [0xe8, 0xd5, 0xa3] },
	RockyShore: { label: "Skaliste wybrzeże", rgb: [0x9a, 0x9a, 0x8a] },
	Desert: { label: "Pustynia", rgb: [0xd4, 0xb4, 0x83] },
	Savanna: { label: "Sawanna", rgb: [0xb8, 0xa8, 0x4a] },
	Grass: { label: "Trawa", rgb: [0x5a, 0xa4, 0x4a] },
	Swamp: { label: "Bagno", rgb: [0x3d, 0x5c, 0x3a] },
	Shrubland: { label: "Krzewiasta stepa", rgb: [0x8a, 0x9a, 0x4a] },
	Forest: { label: "Las", rgb: [0x2d, 0x5a, 0x27] },
	Rainforest: { label: "Las deszczowy", rgb: [0x1a, 0x4a, 0x20] },
	Mountain: { label: "Góry", rgb: [0x6b, 0x6b, 0x6b] },
	Tundra: { label: "Tundra", rgb: [0xb8, 0xc4, 0xb8] },
	Snow: { label: "Śnieg", rgb: [0xf0, 0xf0, 0xf5] },
};

/** Display order for the legend (grouped by terrain type). */
export const BIOME_LEGEND_ORDER: TileType[] = [
	"DeepWater",
	"Water",
	"Sand",
	"RockyShore",
	"Desert",
	"Savanna",
	"Grass",
	"Swamp",
	"Shrubland",
	"Forest",
	"Rainforest",
	"Mountain",
	"Tundra",
	"Snow",
];

export function biomeToRgb(biome: TileType): [number, number, number] {
	return BIOMES[biome].rgb;
}

export function biomeToHex(biome: TileType): string {
	const [r, g, b] = BIOMES[biome].rgb;
	return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
}
