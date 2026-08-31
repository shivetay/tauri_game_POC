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

export interface CanvasPixel {
	px: number;
	py: number;
}

/** Map client coords to canvas bitmap pixels (accounts for object-fit: contain letterboxing). */
export function canvasClientToPixel(
	clientX: number,
	clientY: number,
	canvas: HTMLCanvasElement,
): CanvasPixel | null {
	const rect = canvas.getBoundingClientRect();
	const bitmapW = canvas.width;
	const bitmapH = canvas.height;
	if (bitmapW === 0 || bitmapH === 0 || rect.width === 0 || rect.height === 0) {
		return null;
	}

	const localX = clientX - rect.left;
	const localY = clientY - rect.top;
	const scale = Math.min(rect.width / bitmapW, rect.height / bitmapH);
	const renderedW = bitmapW * scale;
	const renderedH = bitmapH * scale;
	const offsetX = (rect.width - renderedW) / 2;
	const offsetY = (rect.height - renderedH) / 2;

	if (
		localX < offsetX ||
		localX > offsetX + renderedW ||
		localY < offsetY ||
		localY > offsetY + renderedH
	) {
		return null;
	}

	return {
		px: ((localX - offsetX) / renderedW) * bitmapW,
		py: ((localY - offsetY) / renderedH) * bitmapH,
	};
}

export function pixelToCell(
	px: number,
	py: number,
	canvasWidth: number,
	canvasHeight: number,
	cellSize: number,
): GridCell {
	const cols = canvasWidth / cellSize;
	const rows = canvasHeight / cellSize;
	return {
		cx: Math.min(Math.max(0, Math.floor(px / cellSize)), cols - 1),
		cy: Math.min(Math.max(0, Math.floor(py / cellSize)), rows - 1),
	};
}

export function pixelToRegion(
	px: number,
	py: number,
	canvasSize: number,
): RegionId {
	const cellSize = canvasSize / CHUNKS_PER_REGION;
	const cell = pixelToCell(px, py, canvasSize, canvasSize, cellSize);
	return { rx: cell.cx, ry: cell.cy };
}

export function pixelToChunk(px: number, py: number, canvasSize: number): ChunkId {
	const cellSize = canvasSize / CHUNKS_PER_REGION;
	const cell = pixelToCell(px, py, canvasSize, canvasSize, cellSize);
	return { cx: cell.cx, cy: cell.cy };
}
