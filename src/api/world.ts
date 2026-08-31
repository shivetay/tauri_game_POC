import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ChunkId, RegionId, TerrainGrid } from "../types/world";
import type { TerrainParams } from "../types/terrainParams";
import {
	CHUNKS_PER_REGION,
} from "../utils/constants";

export type { ChunkId, RegionId, TerrainCell, TerrainGrid, TileType } from "../types/world";
export type { TerrainParams } from "../types/terrainParams";
export { DEFAULT_TERRAIN_PARAMS } from "../types/terrainParams";
export {
	CHUNK_SIZE,
	CHUNKS_PER_REGION,
	CHUNK_RESOLUTION,
	MACRO_RESOLUTION,
	MICRO_RESOLUTION,
	REGION_SIZE,
} from "../utils/constants";

function assertTauri() {
	if (!isTauri()) {
		throw new Error(
			"Aplikacja wymaga okna Tauri. Uruchom: pnpm tauri dev (nie otwieraj localhost:1420 w przeglądarce).",
		);
	}
}

export function generateGlobal(seed: number, params: TerrainParams) {
	assertTauri();
	return invoke<TerrainGrid>("generate_global", { seed, params });
}

export function generateRegion(
	seed: number,
	params: TerrainParams,
	rx: number,
	ry: number,
) {
	assertTauri();
	return invoke<TerrainGrid>("generate_region", { seed, params, rx, ry });
}

export function generateChunk(
	seed: number,
	params: TerrainParams,
	rx: number,
	ry: number,
	cx: number,
	cy: number,
) {
	assertTauri();
	return invoke<TerrainGrid>("generate_chunk", {
		seed,
		params,
		rx,
		ry,
		cx,
		cy,
	});
}

export interface GridCell {
	cx: number;
	cy: number;
}

export function pixelToCell(
	px: number,
	py: number,
	canvasSize: number,
	cellSize: number,
): GridCell {
	return {
		cx: Math.min(Math.floor(px / cellSize), canvasSize / cellSize - 1),
		cy: Math.min(Math.floor(py / cellSize), canvasSize / cellSize - 1),
	};
}

export function pixelToRegion(
	px: number,
	py: number,
	canvasSize: number,
): RegionId {
	const cell = pixelToCell(px, py, canvasSize, canvasSize / CHUNKS_PER_REGION);
	return { rx: cell.cx, ry: cell.cy };
}

export function pixelToChunk(px: number, py: number, canvasSize: number): ChunkId {
	const cell = pixelToCell(px, py, canvasSize, canvasSize / CHUNKS_PER_REGION);
	return { cx: cell.cx, cy: cell.cy };
}
