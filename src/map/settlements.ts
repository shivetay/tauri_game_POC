import type { Settlement, SettlementKind, WorldBounds } from "../types/world";

export interface SettlementInfo {
	label: string;
	popMin: number;
	popMax: number;
	fill: string;
	stroke: string;
	minPx: number;
}

/** Keep population ranges in sync with `src-tauri/src/world/settlement.rs`. */
export const SETTLEMENT_KIND_ORDER: SettlementKind[] = [
	"City",
	"Town",
	"Village",
	"Hamlet",
];

export const SETTLEMENT_INFO: Record<SettlementKind, SettlementInfo> = {
	City: {
		label: "Miasto",
		popMin: 8_000,
		popMax: 50_000,
		fill: "#c9a36a",
		stroke: "#3a2a08",
		minPx: 22,
	},
	Town: {
		label: "Miasteczko",
		popMin: 1_200,
		popMax: 8_000,
		fill: "#d4a070",
		stroke: "#3a220c",
		minPx: 14,
	},
	Village: {
		label: "Wieś",
		popMin: 150,
		popMax: 1_200,
		fill: "#e2d0a8",
		stroke: "#2a2418",
		minPx: 9,
	},
	Hamlet: {
		label: "Wioska",
		popMin: 20,
		popMax: 150,
		fill: "#d8d0c0",
		stroke: "#222222",
		minPx: 6,
	},
};

const DRAW_ORDER: SettlementKind[] = ["Hamlet", "Village", "Town", "City"];

const OUTLINE_VERTS: Record<SettlementKind, number> = {
	Hamlet: 6,
	Village: 7,
	Town: 8,
	City: 10,
};

function u32hash(n: number): number {
	let x = Math.imul(n | 0, 1597334677);
	x = (x ^ (x >>> 16)) >>> 0;
	x = Math.imul(x, 2246822519);
	return x >>> 0;
}

function unit(x: number, y: number, i: number): number {
	const xi = (x * 1000) | 0;
	const yi = (y * 1000) | 0;
	return u32hash(u32hash(xi) + Math.imul(u32hash(yi), 3) + i) / 4294967296;
}

export function settlementFootprintRadius(
	settlement: Settlement,
	pxPerWorld: number,
): number {
	return Math.max(
		SETTLEMENT_INFO[settlement.kind].minPx,
		settlement.radius * pxPerWorld,
	);
}

export type SettlementDrawStyle = "marker" | "plan";

const MARKER_SIZE: Record<"City" | "Town", { half: number; hit: number }> = {
	City: { half: 4.5, hit: 8 },
	Town: { half: 2.4, hit: 6 },
};

function markerSize(kind: SettlementKind): { half: number; hit: number } {
	return kind === "City" ? MARKER_SIZE.City : MARKER_SIZE.Town;
}

export function hitSettlement(
	px: number,
	py: number,
	canvasWidth: number,
	canvasHeight: number,
	settlements: Settlement[],
	bounds: WorldBounds,
	style: SettlementDrawStyle = "plan",
): Settlement | null {
	const pxPerWorld = canvasWidth / bounds.span;
	let best: Settlement | null = null;
	let bestD = Infinity;

	for (const settlement of settlements) {
		const sx = ((settlement.x - bounds.x0) / bounds.span) * canvasWidth;
		const sy = ((settlement.y - bounds.y0) / bounds.span) * canvasHeight;
		if (style === "marker") {
			const d = Math.hypot(px - sx, py - sy);
			if (d <= markerSize(settlement.kind).hit && d < bestD) {
				best = settlement;
				bestD = d;
			}
			continue;
		}
		const stretch = 0.72 + unit(settlement.x, settlement.y, 1) * 0.45;
		const r = settlementFootprintRadius(settlement, pxPerWorld) + 3;
		const dx = px - sx;
		const dy = py - sy;
		const rot = (unit(settlement.x, settlement.y, 0) - 0.5) * 0.9;
		const c = Math.cos(-rot);
		const s = Math.sin(-rot);
		const lx = dx * c - dy * s;
		const ly = dx * s + dy * c;
		const nx = lx / (r * stretch);
		const ny = ly / r;
		const d = nx * nx + ny * ny;
		if (d <= 1 && d < bestD) {
			best = settlement;
			bestD = d;
		}
	}

	return best;
}

export function formatSettlementLabel(settlement: Settlement): string {
	const info = SETTLEMENT_INFO[settlement.kind];
	return `${info.label} · ${settlement.population.toLocaleString("pl-PL")} mieszk.`;
}

function traceFootprint(
	ctx: CanvasRenderingContext2D,
	settlement: Settlement,
	R: number,
	stretch: number,
) {
	const verts = OUTLINE_VERTS[settlement.kind];
	ctx.beginPath();
	for (let i = 0; i < verts; i++) {
		const a = (i / verts) * Math.PI * 2;
		const jitter = 0.68 + unit(settlement.x, settlement.y, 10 + i) * 0.42;
		const x = Math.cos(a) * R * stretch * jitter;
		const y = Math.sin(a) * R * jitter;
		if (i === 0) ctx.moveTo(x, y);
		else ctx.lineTo(x, y);
	}
	ctx.closePath();
}

function drawFootprint(
	ctx: CanvasRenderingContext2D,
	sx: number,
	sy: number,
	settlement: Settlement,
	pxPerWorld: number,
	hovered: boolean,
) {
	const info = SETTLEMENT_INFO[settlement.kind];
	const R = settlementFootprintRadius(settlement, pxPerWorld);
	const stretch = 0.72 + unit(settlement.x, settlement.y, 1) * 0.45;
	const rot = (unit(settlement.x, settlement.y, 0) - 0.5) * 0.9;

	ctx.save();
	ctx.translate(sx, sy);
	ctx.rotate(rot);
	traceFootprint(ctx, settlement, R, stretch);
	ctx.globalAlpha = 0.62;
	ctx.fillStyle = info.fill;
	ctx.fill();
	ctx.globalAlpha = 1;
	ctx.strokeStyle = hovered ? "rgba(255, 255, 255, 0.95)" : info.stroke;
	ctx.lineWidth = hovered ? 2 : 1.2;
	ctx.stroke();
	ctx.restore();
}

function drawCityMarker(
	ctx: CanvasRenderingContext2D,
	sx: number,
	sy: number,
	kind: SettlementKind,
	hovered: boolean,
) {
	const info = SETTLEMENT_INFO[kind];
	const half = markerSize(kind).half;
	ctx.save();
	ctx.translate(sx, sy);
	ctx.rotate(Math.PI / 4);
	ctx.beginPath();
	ctx.rect(-half, -half, half * 2, half * 2);
	ctx.fillStyle = info.fill;
	ctx.fill();
	ctx.strokeStyle = info.stroke;
	ctx.lineWidth = kind === "City" ? 1.4 : 1;
	ctx.stroke();
	if (hovered) {
		ctx.strokeStyle = "rgba(255, 255, 255, 0.95)";
		ctx.lineWidth = 2;
		ctx.strokeRect(-half - 2, -half - 2, half * 2 + 4, half * 2 + 4);
	}
	ctx.restore();
}

export function drawSettlements(
	ctx: CanvasRenderingContext2D,
	canvasWidth: number,
	canvasHeight: number,
	settlements: Settlement[],
	bounds: WorldBounds,
	hovered: Settlement | null,
	style: SettlementDrawStyle = "plan",
) {
	const pxPerWorld = canvasWidth / bounds.span;
	const pad = 28;

	for (const kind of DRAW_ORDER) {
		for (const settlement of settlements) {
			if (settlement.kind !== kind) continue;
			const sx = ((settlement.x - bounds.x0) / bounds.span) * canvasWidth;
			const sy = ((settlement.y - bounds.y0) / bounds.span) * canvasHeight;
			if (
				sx < -pad ||
				sy < -pad ||
				sx > canvasWidth + pad ||
				sy > canvasHeight + pad
			) {
				continue;
			}

			const isHovered =
				hovered !== null &&
				hovered.x === settlement.x &&
				hovered.y === settlement.y &&
				hovered.kind === settlement.kind;
			if (style === "marker") {
				drawCityMarker(ctx, sx, sy, settlement.kind, isHovered);
			} else {
				drawFootprint(ctx, sx, sy, settlement, pxPerWorld, isHovered);
			}
		}
	}
}
