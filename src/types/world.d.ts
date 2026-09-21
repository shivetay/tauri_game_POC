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

export type SettlementKind = "Hamlet" | "Village" | "Town" | "City";

export type DistrictKind =
	| "Center"
	| "Market"
	| "Craft"
	| "Port"
	| "Temple"
	| "Noble"
	| "Forest"
	| "Residential"
	| "Outskirts";

export interface District {
	kind: DistrictKind;
	name: string;
	inner: number;
	outer: number;
	a0: number;
	span: number;
}

export interface Settlement {
	x: number;
	y: number;
	kind: SettlementKind;
	population: number;
	radius: number;
	name: string;
	coreDx: number;
	coreDy: number;
	districts: District[];
}

export interface WorldBounds {
	x0: number;
	y0: number;
	span: number;
}
