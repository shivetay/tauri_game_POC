export interface TerrainParams {
	orogenyStrength: number;
	moistureStrength: number;
	detailStrength: number;
	terrainRoughness: number;
	landSize: number;
	coastDistortion: number;
}

export const DEFAULT_TERRAIN_PARAMS: TerrainParams = {
	orogenyStrength: 1,
	moistureStrength: 1,
	detailStrength: 1,
	terrainRoughness: 1,
	landSize: 1,
	coastDistortion: 1,
};
