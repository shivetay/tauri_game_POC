export type TileType =
	| "DeepWater"
	| "Water"
	| "Sand"
	| "RockyShore"
	| "Desert"
	| "Savanna"
	| "Grass"
	| "Swamp"
	| "Shrubland"
	| "Forest"
	| "Rainforest"
	| "Mountain"
	| "Tundra"
	| "Snow";

export interface TerrainCell {
	elevation: number;
	moisture: number;
	biome: TileType;
}

export interface TerrainGrid {
	width: number;
	height: number;
	cells: TerrainCell[];
}

export interface ChunkId {
	cx: number;
	cy: number;
}

export interface RegionId {
	rx: number;
	ry: number;
}
