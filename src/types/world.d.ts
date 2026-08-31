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
	/** Elevation in meters; sea level = 0, max peak = 10_000 m. */
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
