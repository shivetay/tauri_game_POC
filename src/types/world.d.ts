export type TileType =
	| "DeepWater"
	| "Water"
	| "Sand"
	| "Grass"
	| "Forest"
	| "Mountain"
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

export interface RegionId {
	rx: number;
	ry: number;
}
