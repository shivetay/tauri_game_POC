import type { RegionId, TerrainGrid, TileType } from "../types/world";
import { HIGH_ELEVATION_M, MAX_ELEVATION_M } from "../utils/constants";
import { biomeToRgb } from "./colors";

function lerpByte(a: number, b: number, t: number): number {
	return Math.round(a + (b - a) * t);
}

function shadeHighElevation(
	biome: TileType,
	elevationM: number,
	base: [number, number, number],
): [number, number, number] {
	if (
		biome !== "Mountain" &&
		biome !== "Snow" &&
		biome !== "Tundra"
	) {
		return base;
	}

	const t = Math.min(
		1,
		Math.max(0, (elevationM - HIGH_ELEVATION_M) / (MAX_ELEVATION_M - HIGH_ELEVATION_M)),
	);
	const [r, g, b] = base;

	if (biome === "Snow") {
		return [
			lerpByte(r, 0xf8, t),
			lerpByte(g, 0xf8, t),
			lerpByte(b, 0xff, t),
		];
	}

	return [
		lerpByte(r, 0x9a, t),
		lerpByte(g, 0x9a, t),
		lerpByte(b, 0xa8, t),
	];
}

export function terrainGridToImageData(grid: TerrainGrid): ImageData {
	const { width, height, cells } = grid;
	const pixels = new Uint8ClampedArray(width * height * 4);

	for (let i = 0; i < cells.length; i++) {
		const cell = cells[i];
		const [r, g, b] = shadeHighElevation(
			cell.biome,
			cell.elevation,
			biomeToRgb(cell.biome),
		);
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
