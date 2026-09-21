import type {
	District,
	DistrictKind,
	Settlement,
	SettlementKind,
	TerrainGrid,
	TileType,
	WorldBounds,
} from "../types/world";

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

export const DISTRICT_KIND_ORDER: DistrictKind[] = [
	"Center",
	"Market",
	"Craft",
	"Port",
	"Temple",
	"Noble",
	"Residential",
	"Forest",
	"Outskirts",
];

export const DISTRICT_INFO: Record<DistrictKind, { label: string; fill: string }> =
	{
		Center: { label: "Centrum / rynek", fill: "#7a4e36" },
		Market: { label: "Handlowa", fill: "#c9a24a" },
		Craft: { label: "Rzemieślnicza", fill: "#8c5a3c" },
		Port: { label: "Portowa", fill: "#6a8490" },
		Temple: { label: "Świątynna", fill: "#d4c8ae" },
		Noble: { label: "Zamożna", fill: "#c4a07a" },
		Forest: { label: "Leśna", fill: "#5d7a48" },
		Residential: { label: "Mieszkaniowa", fill: "#cbb89a" },
		Outskirts: { label: "Obrzeża", fill: "#b59a72" },
	};

const DRAW_ORDER: SettlementKind[] = ["Hamlet", "Village", "Town", "City"];

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

export interface SettlementHit {
	settlement: Settlement;
	district: District | null;
}

function footprintFrame(settlement: Settlement): { stretch: number; rot: number } {
	return {
		stretch: 0.92 + unit(settlement.x, settlement.y, 1) * 0.16,
		rot: (unit(settlement.x, settlement.y, 0) - 0.5) * 0.7,
	};
}

function toLocalNorm(
	px: number,
	py: number,
	sx: number,
	sy: number,
	settlement: Settlement,
	R: number,
): { nx: number; ny: number } {
	const { stretch, rot } = footprintFrame(settlement);
	const dx = px - sx;
	const dy = py - sy;
	const c = Math.cos(-rot);
	const s = Math.sin(-rot);
	const lx = (dx * c - dy * s) / stretch;
	const ly = dx * s + dy * c;
	return { nx: lx / R, ny: ly / R };
}

const TAU = Math.PI * 2;

function normAngle(a: number): number {
	let x = a % TAU;
	if (x < 0) x += TAU;
	return x;
}

function outerScale(settlement: Settlement): number {
	let m = 0;
	for (const d of settlement.districts) {
		if (d.outer > m) m = d.outer;
	}
	return m > 0.01 ? m : 1;
}

function isPlanned(settlement: Settlement): boolean {
	return (
		settlement.kind === "City" && unit(settlement.x, settlement.y, 12) < 0.38
	);
}

function roadAxes(settlement: Settlement): number[] {
	const base = unit(settlement.x, settlement.y, 13) * TAU;
	if (settlement.kind === "Hamlet") return [base];
	if (settlement.kind === "Village") {
		if (unit(settlement.x, settlement.y, 14) < 0.5) return [base];
		return [base, base + Math.PI * (0.1 + unit(settlement.x, settlement.y, 15) * 0.2)];
	}
	const axes = [base];
	axes.push(
		base + Math.PI * (0.4 + unit(settlement.x, settlement.y, 14) * 0.32),
	);
	if (
		settlement.kind === "City" &&
		unit(settlement.x, settlement.y, 16) < 0.58
	) {
		axes.push(base + TAU * (0.63 + unit(settlement.x, settlement.y, 17) * 0.1));
	}
	return axes;
}

function corePos(settlement: Settlement): { x: number; y: number } {
	return { x: settlement.coreDx, y: settlement.coreDy };
}

function outlineR(settlement: Settlement, angle: number): number {
	const a = normAngle(angle);
	const axes = roadAxes(settlement);
	const planned = isPlanned(settlement);
	const elongA = axes[0];
	const da = a - elongA;
	const c = Math.cos(da);
	const s = Math.sin(da);
	const elong = planned
		? 1.1 + unit(settlement.x, settlement.y, 18) * 0.12
		: settlement.kind === "Hamlet"
			? 1.2 + unit(settlement.x, settlement.y, 18) * 0.22
			: 1.48 + unit(settlement.x, settlement.y, 18) * 0.5;
	const perp = planned
		? 0.9
		: 0.5 + unit(settlement.x, settlement.y, 19) * 0.22;
	let r = (elong * perp) / Math.sqrt((perp * c) ** 2 + (elong * s) ** 2);
	r *= planned ? 0.76 : 0.68;
	if (planned) {
		const sq = 0.8 / Math.max(Math.abs(c), Math.abs(s), 0.25);
		r = r * 0.38 + Math.min(sq, 1.22) * 0.62;
	} else {
		for (let i = 0; i < axes.length; i++) {
			const k = Math.cos(a - axes[i]);
			if (k > 0.1) {
				const lobe =
					0.16 + unit(settlement.x, settlement.y, 21 + i) * 0.14;
				r += lobe * k * k;
			}
		}
	}
	const aq = ((a / TAU) * 16) | 0;
	r *= 0.9 + unit(settlement.x, settlement.y, 30 + aq) * 0.22;
	return Math.max(0.36, Math.min(1.62, r));
}

function planOutline(settlement: Settlement): { x: number; y: number }[] {
	const n =
		settlement.kind === "City" ? 22 : settlement.kind === "Town" ? 18 : 14;
	const rot = roadAxes(settlement)[0];
	const c = corePos(settlement);
	const pts: { x: number; y: number }[] = [];
	for (let i = 0; i < n; i++) {
		const a = rot + (i / n) * TAU;
		const r = outlineR(settlement, a);
		pts.push({ x: c.x + Math.cos(a) * r, y: c.y + Math.sin(a) * r });
	}
	return pts;
}

function districtSite(
	settlement: Settlement,
	district: District,
): { x: number; y: number } {
	const c = corePos(settlement);
	if (district.span >= TAU - 1e-3) return c;
	const a = district.a0 + district.span * 0.5;
	const rOut = outlineR(settlement, a);
	let t = (district.inner + district.outer) * 0.5 / outerScale(settlement);
	t = Math.min(0.84, Math.max(0.16, t * 0.85));
	if (district.kind === "Forest" || district.kind === "Outskirts") t += 0.1;
	if (district.kind === "Center") t = 0;
	t = Math.min(0.88, t);
	return { x: c.x + Math.cos(a) * rOut * t, y: c.y + Math.sin(a) * rOut * t };
}

function bisectorHit(
	p: { x: number; y: number },
	q: { x: number; y: number },
	a: { x: number; y: number },
	b: { x: number; y: number },
): { x: number; y: number } {
	const mx = (a.x + b.x) * 0.5;
	const my = (a.y + b.y) * 0.5;
	const nx = b.x - a.x;
	const ny = b.y - a.y;
	const dx = q.x - p.x;
	const dy = q.y - p.y;
	const den = dx * nx + dy * ny;
	if (Math.abs(den) < 1e-12) {
		return { x: (p.x + q.x) * 0.5, y: (p.y + q.y) * 0.5 };
	}
	const t = ((mx - p.x) * nx + (my - p.y) * ny) / den;
	return { x: p.x + t * dx, y: p.y + t * dy };
}

function clipHalfPlane(
	poly: { x: number; y: number }[],
	site: { x: number; y: number },
	other: { x: number; y: number },
): { x: number; y: number }[] {
	if (poly.length < 3) return [];
	const ddx = site.x - other.x;
	const ddy = site.y - other.y;
	if (ddx * ddx + ddy * ddy < 1e-8) return poly;
	const out: { x: number; y: number }[] = [];
	const n = poly.length;
	for (let i = 0; i < n; i++) {
		const cur = poly[i];
		const prev = poly[(i + n - 1) % n];
		const curIn =
			(cur.x - site.x) ** 2 + (cur.y - site.y) ** 2 <=
			(cur.x - other.x) ** 2 + (cur.y - other.y) ** 2;
		const prevIn =
			(prev.x - site.x) ** 2 + (prev.y - site.y) ** 2 <=
			(prev.x - other.x) ** 2 + (prev.y - other.y) ** 2;
		if (curIn) {
			if (!prevIn) out.push(bisectorHit(prev, cur, site, other));
			out.push(cur);
		} else if (prevIn) {
			out.push(bisectorHit(prev, cur, site, other));
		}
	}
	return out;
}

function districtPoints(
	settlement: Settlement,
	district: District,
): { x: number; y: number }[] {
	if (settlement.districts.length === 0) return planOutline(settlement);
	const site = districtSite(settlement, district);
	let poly = planOutline(settlement).map((p) => ({ x: p.x, y: p.y }));
	for (const other of settlement.districts) {
		if (other.name === district.name && other.a0 === district.a0) continue;
		poly = clipHalfPlane(poly, site, districtSite(settlement, other));
		if (poly.length < 3) return [];
	}
	return poly;
}

function pointInPoly(
	x: number,
	y: number,
	pts: { x: number; y: number }[],
): boolean {
	let inside = false;
	for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
		const yi = pts[i].y;
		const yj = pts[j].y;
		if ((yi > y) === (yj > y)) continue;
		const xi = pts[i].x;
		const xj = pts[j].x;
		if (x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
	}
	return inside;
}

function districtAt(settlement: Settlement, nx: number, ny: number): District | null {
	for (let i = settlement.districts.length - 1; i >= 0; i--) {
		const d = settlement.districts[i];
		const poly = districtPoints(settlement, d);
		if (poly.length >= 3 && pointInPoly(nx, ny, poly)) return d;
	}
	return settlement.districts[0] ?? null;
}

function tracePoly(
	ctx: CanvasRenderingContext2D,
	pts: { x: number; y: number }[],
	scale: number,
) {
	if (pts.length === 0) return;
	ctx.beginPath();
	ctx.moveTo(pts[0].x * scale, pts[0].y * scale);
	for (let i = 1; i < pts.length; i++) {
		ctx.lineTo(pts[i].x * scale, pts[i].y * scale);
	}
	ctx.closePath();
}

export function hitSettlement(
	px: number,
	py: number,
	canvasWidth: number,
	canvasHeight: number,
	settlements: Settlement[],
	bounds: WorldBounds,
	style: SettlementDrawStyle = "plan",
): SettlementHit | null {
	const pxPerWorld = canvasWidth / bounds.span;
	let best: SettlementHit | null = null;
	let bestD = Infinity;

	for (const settlement of settlements) {
		const sx = ((settlement.x - bounds.x0) / bounds.span) * canvasWidth;
		const sy = ((settlement.y - bounds.y0) / bounds.span) * canvasHeight;
		if (style === "marker") {
			const d = Math.hypot(px - sx, py - sy);
			if (d <= markerSize(settlement.kind).hit && d < bestD) {
				best = { settlement, district: null };
				bestD = d;
			}
			continue;
		}
		const r = settlementFootprintRadius(settlement, pxPerWorld);
		const { nx, ny } = toLocalNorm(px, py, sx, sy, settlement, r);
		const outline = planOutline(settlement);
		if (!pointInPoly(nx, ny, outline)) continue;
		const dx = nx - settlement.coreDx;
		const dy = ny - settlement.coreDy;
		const d = dx * dx + dy * dy;
		if (d < bestD) {
			best = {
				settlement,
				district: districtAt(settlement, nx, ny),
			};
			bestD = d;
		}
	}

	return best;
}

export function formatSettlementLabel(
	settlement: Settlement,
	district: District | null = null,
): string {
	const pop = settlement.population.toLocaleString("pl-PL");
	if (district) {
		return `${settlement.name} · ${district.name} · ${pop} mieszk.`;
	}
	return `${settlement.name} · ${pop} mieszk.`;
}

function isWaterBiome(biome: TileType): boolean {
	return biome === "Water" || biome === "DeepWater";
}

function biomeAtCanvas(
	grid: TerrainGrid,
	canvasX: number,
	canvasY: number,
): TileType | null {
	const gx = Math.floor(canvasX);
	const gy = Math.floor(canvasY);
	if (gx < 0 || gy < 0 || gx >= grid.width || gy >= grid.height) return null;
	return grid.cells[gy * grid.width + gx].biome;
}

function localToCanvas(
	localX: number,
	localY: number,
	sx: number,
	sy: number,
	rot: number,
	stretch: number,
): { x: number; y: number } {
	const lx = localX * stretch;
	const ly = localY;
	const c = Math.cos(rot);
	const s = Math.sin(rot);
	return { x: sx + lx * c - ly * s, y: sy + lx * s + ly * c };
}

function footprintLocalVerts(
	settlement: Settlement,
	R: number,
): { x: number; y: number }[] {
	return planOutline(settlement).map((p) => ({ x: p.x * R, y: p.y * R }));
}

function pullOntoLand(
	verts: { x: number; y: number }[],
	sx: number,
	sy: number,
	rot: number,
	stretch: number,
	grid: TerrainGrid | null,
): { x: number; y: number }[] {
	if (!grid) return verts;
	return verts.map((v) => {
		let x = v.x;
		let y = v.y;
		for (let k = 0; k < 10; k++) {
			const p = localToCanvas(x, y, sx, sy, rot, stretch);
			const biome = biomeAtCanvas(grid, p.x, p.y);
			if (biome !== null && !isWaterBiome(biome)) return { x, y };
			x *= 0.8;
			y *= 0.8;
		}
		return { x, y };
	});
}

function punchWater(
	overlay: CanvasRenderingContext2D,
	ox: number,
	oy: number,
	width: number,
	height: number,
	grid: TerrainGrid,
) {
	const img = overlay.getImageData(0, 0, width, height);
	const data = img.data;
	for (let y = 0; y < height; y++) {
		for (let x = 0; x < width; x++) {
			const i = (y * width + x) * 4;
			if (data[i + 3] === 0) continue;
			const biome = biomeAtCanvas(grid, ox + x, oy + y);
			if (biome === null || isWaterBiome(biome)) data[i + 3] = 0;
		}
	}
	overlay.putImageData(img, 0, 0);
}

function traceVerts(
	ctx: CanvasRenderingContext2D,
	verts: { x: number; y: number }[],
) {
	if (verts.length === 0) return;
	ctx.beginPath();
	ctx.moveTo(verts[0].x, verts[0].y);
	for (let i = 1; i < verts.length; i++) {
		ctx.lineTo(verts[i].x, verts[i].y);
	}
	ctx.closePath();
}

function sameSettlement(a: Settlement, b: Settlement): boolean {
	return a.x === b.x && a.y === b.y && a.kind === b.kind;
}

function drawFootprint(
	ctx: CanvasRenderingContext2D,
	sx: number,
	sy: number,
	settlement: Settlement,
	pxPerWorld: number,
	hovered: boolean,
	hoveredDistrict: District | null,
	grid: TerrainGrid | null,
) {
	const info = SETTLEMENT_INFO[settlement.kind];
	const R = settlementFootprintRadius(settlement, pxPerWorld);
	const { stretch, rot } = footprintFrame(settlement);
	const verts = pullOntoLand(
		footprintLocalVerts(settlement, R),
		sx,
		sy,
		rot,
		stretch,
		grid,
	);
	const pad = Math.ceil(R * Math.max(stretch, 1) * 2.15 + 10);
	const size = pad * 2;
	const overlay = document.createElement("canvas");
	overlay.width = size;
	overlay.height = size;
	const octx = overlay.getContext("2d");
	if (!octx) return;

	octx.translate(pad, pad);
	octx.rotate(rot);
	octx.scale(stretch, 1);
	traceVerts(octx, verts);
	octx.save();
	octx.clip();
	if (settlement.districts.length > 0) {
		octx.fillStyle = DISTRICT_INFO.Residential.fill;
		octx.fill();
		for (const district of settlement.districts) {
			const pts = districtPoints(settlement, district);
			if (pts.length < 3) continue;
			tracePoly(octx, pts, R);
			octx.fillStyle = DISTRICT_INFO[district.kind].fill;
			octx.fill();
		}
	} else {
		octx.fillStyle = info.fill;
		octx.fill();
	}
	octx.restore();
	octx.setTransform(1, 0, 0, 1, 0, 0);
	if (grid) punchWater(octx, sx - pad, sy - pad, size, size, grid);

	ctx.drawImage(overlay, sx - pad, sy - pad);

	ctx.save();
	ctx.translate(sx, sy);
	ctx.rotate(rot);
	ctx.scale(stretch, 1);
	traceVerts(ctx, verts);
	ctx.strokeStyle = "rgba(48, 32, 18, 0.4)";
	ctx.lineWidth = 1;
	ctx.stroke();
	if (hovered) {
		ctx.save();
		traceVerts(ctx, verts);
		ctx.clip();
		if (hoveredDistrict) {
			tracePoly(ctx, districtPoints(settlement, hoveredDistrict), R);
		} else {
			traceVerts(ctx, verts);
		}
		ctx.strokeStyle = "rgba(255, 255, 255, 0.85)";
		ctx.lineWidth = 1.4;
		ctx.stroke();
		ctx.restore();
	}
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
	hovered: SettlementHit | null,
	style: SettlementDrawStyle = "plan",
	grid: TerrainGrid | null = null,
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
				hovered !== null && sameSettlement(hovered.settlement, settlement);
			if (style === "marker") {
				drawCityMarker(ctx, sx, sy, settlement.kind, isHovered);
			} else {
				drawFootprint(
					ctx,
					sx,
					sy,
					settlement,
					pxPerWorld,
					isHovered,
					isHovered ? hovered.district : null,
					grid,
				);
			}
		}
	}
}
