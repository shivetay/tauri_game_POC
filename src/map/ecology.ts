import type {
	ChunkEcologyMap,
	FaunaGroup,
	LifeInstance,
	RegionEcologyMap,
	WorldBounds,
} from "../types/world";

export const FAUNA_GROUP_INFO: Record<
	FaunaGroup,
	{ label: string; rgb: [number, number, number] }
> = {
	LargeMammal: { label: "Duże ssaki", rgb: [0xc4, 0x7a, 0x2c] },
	Bird: { label: "Ptaki", rgb: [0x3a, 0x7c, 0xb8] },
	SmallFauna: { label: "Mała fauna", rgb: [0x8a, 0x4a, 0x9a] },
};

/** Same green as region biome-potential wash (vegetation). */
export const VEGETATION_RGB: [number, number, number] = [0x4a, 0x8a, 0x5a];

const HABITAT_THRESHOLD = 0.28;
const GROUP_THRESHOLD = 0.38;

function paintHabitatPixels(
	data: Uint8ClampedArray,
	canvasWidth: number,
	canvasHeight: number,
	habitat: RegionEcologyMap,
): void {
	const { width, height, cells } = habitat;
	const scaleX = canvasWidth / width;
	const scaleY = canvasHeight / height;

	for (let py = 0; py < canvasHeight; py++) {
		const gy = Math.min(height - 1, Math.floor(py / scaleY));
		for (let px = 0; px < canvasWidth; px++) {
			const gx = Math.min(width - 1, Math.floor(px / scaleX));
			const cell = cells[gy * width + gx];
			if (!cell || cell.potential < HABITAT_THRESHOLD) continue;

			let r = VEGETATION_RGB[0];
			let g = VEGETATION_RGB[1];
			let b = VEGETATION_RGB[2];
			let a = Math.floor(cell.potential * 55);

			const groups: { v: number; rgb: [number, number, number] }[] = [
				{ v: cell.largeMammals, rgb: FAUNA_GROUP_INFO.LargeMammal.rgb },
				{ v: cell.birds, rgb: FAUNA_GROUP_INFO.Bird.rgb },
				{ v: cell.smallFauna, rgb: FAUNA_GROUP_INFO.SmallFauna.rgb },
			];
			groups.sort((x, y) => y.v - x.v);
			const top = groups[0];
			if (top && top.v >= GROUP_THRESHOLD) {
				const t = Math.min(1, (top.v - GROUP_THRESHOLD) / 0.45);
				r = Math.round(r * (1 - t) + top.rgb[0] * t);
				g = Math.round(g * (1 - t) + top.rgb[1] * t);
				b = Math.round(b * (1 - t) + top.rgb[2] * t);
				a = Math.max(a, Math.floor(40 + top.v * 70));
				const second = groups[1];
				if (
					second &&
					second.v >= GROUP_THRESHOLD &&
					(px + py) % 6 < 2
				) {
					r = Math.round(r * 0.55 + second.rgb[0] * 0.45);
					g = Math.round(g * 0.55 + second.rgb[1] * 0.45);
					b = Math.round(b * 0.55 + second.rgb[2] * 0.45);
				}
			}

			const i = (py * canvasWidth + px) * 4;
			data[i] = r;
			data[i + 1] = g;
			data[i + 2] = b;
			data[i + 3] = a;
		}
	}
}

/** Soft biome wash (A) + fauna-group tints (B). Used on region and chunk. */
export function drawHabitatOverlay(
	ctx: CanvasRenderingContext2D,
	canvasWidth: number,
	canvasHeight: number,
	habitat: RegionEcologyMap,
): void {
	const { width, height, cells } = habitat;
	if (width === 0 || height === 0 || cells.length === 0) return;

	const off = document.createElement("canvas");
	off.width = canvasWidth;
	off.height = canvasHeight;
	const offCtx = off.getContext("2d");
	if (!offCtx) return;

	const image = offCtx.createImageData(canvasWidth, canvasHeight);
	paintHabitatPixels(image.data, canvasWidth, canvasHeight, habitat);
	offCtx.putImageData(image, 0, 0);
	ctx.drawImage(off, 0, 0);
}

function canvasToWorld(
	px: number,
	py: number,
	canvasWidth: number,
	canvasHeight: number,
	bounds: WorldBounds,
): { x: number; y: number } {
	return {
		x: bounds.x0 + (px / canvasWidth) * bounds.span,
		y: bounds.y0 + (py / canvasHeight) * bounds.span,
	};
}

/** Detailed flora + fauna list for the hovered area on chunk LOD. */
export function lifeAreaSummary(
	life: ChunkEcologyMap,
	canvasWidth: number,
	canvasHeight: number,
	px: number,
	py: number,
	bounds: WorldBounds,
): string | null {
	const { x, y } = canvasToWorld(px, py, canvasWidth, canvasHeight, bounds);
	const radius = bounds.span * 0.12;
	const r2 = radius * radius;

	const floraNearby = life.flora.filter((inst) => {
		const dx = inst.x - x;
		const dy = inst.y - y;
		return dx * dx + dy * dy <= r2;
	});
	const faunaNearby = life.fauna.filter((inst) => {
		const dx = inst.x - x;
		const dy = inst.y - y;
		return dx * dx + dy * dy <= r2;
	});

	if (floraNearby.length === 0 && faunaNearby.length === 0) {
		const gx = Math.min(
			life.width - 1,
			Math.max(0, Math.floor((px / canvasWidth) * life.width)),
		);
		const gy = Math.min(
			life.height - 1,
			Math.max(0, Math.floor((py / canvasHeight) * life.height)),
		);
		const cell = life.cells[gy * life.width + gx];
		if (!cell || cell.potential < HABITAT_THRESHOLD) return null;
		return "Obszar życia — brak zidentyfikowanych gatunków w zasięgu";
	}

	const uniq = (items: LifeInstance[]) => {
		const seen = new Set<string>();
		const out: string[] = [];
		for (const inst of items) {
			const key = `${inst.species}|${inst.subspecies}`;
			if (seen.has(key)) continue;
			seen.add(key);
			out.push(`${inst.species} (${inst.subspecies})`);
			if (out.length >= 4) break;
		}
		return out;
	};

	const floraPart = uniq(floraNearby);
	const faunaPart = uniq(faunaNearby);
	const parts: string[] = [];
	if (floraPart.length > 0) parts.push(`Flora: ${floraPart.join(", ")}`);
	if (faunaPart.length > 0) parts.push(`Fauna: ${faunaPart.join(", ")}`);
	return parts.join(" · ");
}

export function regionTileSummary(
	habitat: RegionEcologyMap,
	cx: number,
	cy: number,
): string | null {
	const across = habitat.tilesAcross;
	if (!across || habitat.tiles.length === 0) return null;
	if (cx < 0 || cy < 0 || cx >= across || cy >= across) return null;

	const tile = habitat.tiles[cy * across + cx];
	if (!tile) return null;

	const parts: string[] = [];
	if (tile.flora.length > 0) {
		parts.push(`Flora: ${tile.flora.join(", ")}`);
	}
	if (tile.fauna.length > 0) {
		parts.push(`Fauna: ${tile.fauna.join(", ")}`);
	}
	if (parts.length === 0) {
		return `Kafel (${cx}, ${cy}) — brak spodziewanego życia`;
	}
	return `Kafel (${cx}, ${cy}) — ${parts.join(" · ")}`;
}

export function chunkHabitatFromLife(life: ChunkEcologyMap): RegionEcologyMap {
	return {
		width: life.width,
		height: life.height,
		cells: life.cells,
		tilesAcross: 0,
		tiles: [],
	};
}
