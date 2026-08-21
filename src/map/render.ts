import type { TerrainGrid } from "../types/world";
import { biomeToRgb } from "./colors";

export function terrainGridToImageData(grid: TerrainGrid): ImageData {
	const { width, height, cells } = grid;
	const pixels = new Uint8ClampedArray(width * height * 4);

	for (let i = 0; i < cells.length; i++) {
		const [r, g, b] = biomeToRgb(cells[i].biome);
		const o = i * 4;
		pixels[o] = r;
		pixels[o + 1] = g;
		pixels[o + 2] = b;
		pixels[o + 3] = 255;
	}

	return new ImageData(pixels, width, height);
}
