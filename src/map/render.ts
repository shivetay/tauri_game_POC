import type { RegionId, TerrainGrid } from "../types/world";
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

export function drawRegionGrid(
	ctx: CanvasRenderingContext2D,
	width: number,
	height: number,
	regionSize: number,
) {
	ctx.save();
	ctx.strokeStyle = "rgba(255, 255, 255, 0.5)";
	ctx.lineWidth = 1;

	for (let x = regionSize; x < width; x += regionSize) {
		ctx.beginPath();
		ctx.moveTo(x + 0.5, 0);
		ctx.lineTo(x + 0.5, height);
		ctx.stroke();
	}

	for (let y = regionSize; y < height; y += regionSize) {
		ctx.beginPath();
		ctx.moveTo(0, y + 0.5);
		ctx.lineTo(width, y + 0.5);
		ctx.stroke();
	}

	ctx.restore();
}

export function drawCellHighlight(
	ctx: CanvasRenderingContext2D,
	cx: number,
	cy: number,
	cellSize: number,
) {
	const x = cx * cellSize;
	const y = cy * cellSize;

	ctx.save();
	ctx.fillStyle = "rgba(255, 255, 255, 0.22)";
	ctx.fillRect(x, y, cellSize, cellSize);
	ctx.strokeStyle = "rgba(255, 255, 255, 0.95)";
	ctx.lineWidth = 2;
	ctx.strokeRect(x + 1, y + 1, cellSize - 2, cellSize - 2);
	ctx.restore();
}

export function drawRegionHighlight(
	ctx: CanvasRenderingContext2D,
	region: RegionId,
	regionSize: number,
) {
	drawCellHighlight(ctx, region.rx, region.ry, regionSize);
}
