import { invoke, isTauri } from "@tauri-apps/api/core";
import type { RegionId, TerrainGrid } from "../types/world";
import { MACRO_RESOLUTION, REGION_SIZE } from "../utils/constants";

export type { RegionId, TerrainCell, TerrainGrid, TileType } from "../types/world";
export { MACRO_RESOLUTION, REGION_SIZE } from "../utils/constants";

function assertTauri() {
	if (!isTauri()) {
		throw new Error(
			"Aplikacja wymaga okna Tauri. Uruchom: pnpm tauri dev (nie otwieraj localhost:1420 w przeglądarce).",
		);
	}
}

export function generateGlobal(seed: string) {
	assertTauri();
	return invoke<TerrainGrid>("generate_global", { seed });
}

export function generateRegion(seed: string, rx: number, ry: number) {
	assertTauri();
	return invoke<TerrainGrid>("generate_region", { seed, rx, ry });
}

export function pixelToRegion(
	px: number,
	py: number,
	canvasSize: number,
): RegionId {
	const worldX = (px / canvasSize) * MACRO_RESOLUTION;
	const worldY = (py / canvasSize) * MACRO_RESOLUTION;
	return {
		rx: Math.floor(worldX / REGION_SIZE),
		ry: Math.floor(worldY / REGION_SIZE),
	};
}
