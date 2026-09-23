import type {
	District,
	DistrictKind,
	Road,
	RoadKind,
	RoadSurface,
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
		Forest: { label: "Leśna / park", fill: "#5d7a48" },
		Residential: { label: "Mieszkaniowa", fill: "#cbb89a" },
		Outskirts: { label: "Obrzeża", fill: "#b59a72" },
	};

export type RoadDetail = "main" | "region" | "close";

export const ROAD_KIND_ORDER: RoadKind[] = ["Highway", "Secondary", "Local"];

export const ROAD_INFO: Record<RoadKind, { label: string; stroke: string }> = {
	Highway: { label: "Szlak główny", stroke: "#c4a36a" },
	Secondary: { label: "Droga boczna", stroke: "#8a6a48" },
	Local: { label: "Droga poboczna", stroke: "#6a5340" },
};

const STREET_BED = "#ead9b0";

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

function settlementWorldRadius(settlement: Settlement): number {
	let maxR = 1;
	for (const p of planOutline(settlement)) {
		const r = Math.hypot(p.x, p.y);
		if (r > maxR) maxR = r;
	}
	return settlement.radius * maxR * 1.06;
}

function settlementIntersectsBounds(
	settlement: Settlement,
	bounds: WorldBounds,
): boolean {
	const r = settlementWorldRadius(settlement);
	return !(
		settlement.x + r < bounds.x0 ||
		settlement.x - r > bounds.x0 + bounds.span ||
		settlement.y + r < bounds.y0 ||
		settlement.y - r > bounds.y0 + bounds.span
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

function footprintFrame(): { stretch: number; rot: number } {
	return { stretch: 1, rot: 0 };
}

function toLocalNorm(
	px: number,
	py: number,
	sx: number,
	sy: number,
	R: number,
): { nx: number; ny: number } {
	const { stretch, rot } = footprintFrame();
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

function roadApproaches(settlement: Settlement): { angle: number; kind: RoadKind }[] {
	if (settlement.roadApproaches.length > 0) {
		return settlement.roadApproaches;
	}
	return roadAxesFallback(settlement).map((angle) => ({
		angle,
		kind: "Secondary" as const,
	}));
}

function roadAxesFallback(settlement: Settlement): number[] {
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

function roadAxes(settlement: Settlement): number[] {
	const hw = roadApproaches(settlement).filter((a) => a.kind === "Highway");
	if (hw.length > 0) return hw.map((a) => a.angle);
	return roadApproaches(settlement).map((a) => a.angle);
}

function lobeWeight(kind: RoadKind): number {
	if (kind === "Highway") return 1.55;
	if (kind === "Secondary") return 1.05;
	return 0.55;
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
		const approaches = roadApproaches(settlement);
		for (let i = 0; i < approaches.length; i++) {
			const k = Math.cos(a - approaches[i].angle);
			if (k > 0.2) {
				r += 0.08 * lobeWeight(approaches[i].kind) * k * k;
			}
		}
	} else {
		const approaches = roadApproaches(settlement);
		for (let i = 0; i < approaches.length; i++) {
			const k = Math.cos(a - approaches[i].angle);
			if (k > 0.1) {
				const lobe =
					(0.16 + unit(settlement.x, settlement.y, 21 + i) * 0.14) *
					lobeWeight(approaches[i].kind);
				r += lobe * k * k;
			}
		}
	}
	const aq = ((a / TAU) * 16) | 0;
	let jitter = 0.9 + unit(settlement.x, settlement.y, 30 + aq) * 0.22;
	for (const approach of roadApproaches(settlement)) {
		const k = Math.cos(a - approach.angle);
		if (k > 0.72) {
			const w = (k - 0.72) / 0.28;
			jitter = jitter * (1 - w) + w;
		}
	}
	r *= jitter;
	return Math.max(0.36, Math.min(1.72, r));
}

function planOutline(settlement: Settlement): { x: number; y: number }[] {
	const n =
		settlement.kind === "City" ? 22 : settlement.kind === "Town" ? 18 : 14;
	const rot = roadAxes(settlement)[0];
	const pts: { x: number; y: number }[] = [];
	for (let i = 0; i < n; i++) {
		const a = rot + (i / n) * TAU;
		const r = outlineR(settlement, a);
		pts.push({ x: Math.cos(a) * r, y: Math.sin(a) * r });
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

type Pt = { x: number; y: number };

interface CityBlock {
	poly: Pt[];
	district: District | null;
}

function polyArea(poly: Pt[]): number {
	let a = 0;
	for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
		a += poly[j].x * poly[i].y - poly[i].x * poly[j].y;
	}
	return a * 0.5;
}

function centroid(poly: Pt[]): Pt {
	let x = 0;
	let y = 0;
	let a = 0;
	for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
		const c = poly[j].x * poly[i].y - poly[i].x * poly[j].y;
		x += (poly[j].x + poly[i].x) * c;
		y += (poly[j].y + poly[i].y) * c;
		a += c;
	}
	if (Math.abs(a) < 1e-10) return poly[0];
	return { x: x / (3 * a), y: y / (3 * a) };
}

function splitPolyByLine(
	poly: Pt[],
	ax: number,
	ay: number,
	bx: number,
	by: number,
): [Pt[], Pt[]] {
	const nx = -(by - ay);
	const ny = bx - ax;
	const side = (p: Pt) => nx * (p.x - ax) + ny * (p.y - ay);
	const hit = (p: Pt, q: Pt) => {
		const s1 = side(p);
		const s2 = side(q);
		const t = s1 / (s1 - s2 || 1e-9);
		return { x: p.x + (q.x - p.x) * t, y: p.y + (q.y - p.y) * t };
	};
	const left: Pt[] = [];
	const right: Pt[] = [];
	const n = poly.length;
	for (let i = 0; i < n; i++) {
		const cur = poly[i];
		const prev = poly[(i + n - 1) % n];
		const cin = side(cur) >= 0;
		const pin = side(prev) >= 0;
		if (cin) {
			if (!pin) left.push(hit(prev, cur));
			left.push(cur);
		} else if (pin) {
			left.push(hit(prev, cur));
		}
		if (!cin) {
			if (pin) right.push(hit(prev, cur));
			right.push(cur);
		} else if (!pin) {
			right.push(hit(prev, cur));
		}
	}
	return [left, right];
}

function insetPoly(poly: Pt[], amt: number): Pt[] {
	const c = centroid(poly);
	return poly.map((p) => {
		const dx = p.x - c.x;
		const dy = p.y - c.y;
		const d = Math.hypot(dx, dy);
		if (d < 1e-6) return p;
		const k = Math.max(0.15, (d - amt) / d);
		return { x: c.x + dx * k, y: c.y + dy * k };
	});
}

function streetSplits(settlement: Settlement): { ax: number; ay: number; bx: number; by: number }[] {
	const planned = isPlanned(settlement);
	const approaches = roadApproaches(settlement);
	const main = approaches[0]?.angle ?? roadAxes(settlement)[0] ?? 0;
	const lines: { ax: number; ay: number; bx: number; by: number }[] = [];
	const addLine = (angle: number, ox: number, oy: number) => {
		const ux = Math.cos(angle);
		const uy = Math.sin(angle);
		lines.push({
			ax: ox - ux * 2.4,
			ay: oy - uy * 2.4,
			bx: ox + ux * 2.4,
			by: oy + uy * 2.4,
		});
	};
	for (const ap of approaches) {
		addLine(ap.angle, 0, 0);
	}
	if (approaches.length === 0) addLine(main, 0, 0);

	let extraPara = 0;
	let extraPerp = 1;
	if (settlement.kind === "City") {
		extraPara = 2;
		extraPerp = 2;
	} else if (settlement.kind === "Town") {
		extraPara = 1;
		extraPerp = 2;
	} else if (settlement.kind === "Village") {
		extraPara = 1;
		extraPerp = 1;
	}
	const jitter = planned ? 0.04 : 0.16;
	const addOffsets = (base: number, count: number, salt: number) => {
		for (let i = 0; i < count; i++) {
			const sign = i % 2 === 0 ? 1 : -1;
			const rank = Math.floor(i / 2) + 1;
			const off =
				sign * rank * (0.26 + unit(settlement.x, settlement.y, salt + i) * 0.12);
			const ang =
				base + (unit(settlement.x, settlement.y, salt + 20 + i) - 0.5) * jitter;
			const px = -Math.sin(base);
			const py = Math.cos(base);
			addLine(ang, px * off, py * off);
		}
	};
	addOffsets(main, extraPara, 80);
	addOffsets(main + Math.PI / 2, extraPerp, 120);
	return lines;
}

function settlementBlocks(settlement: Settlement): CityBlock[] {
	const splits = streetSplits(settlement);
	let polys: Pt[][] = [planOutline(settlement)];
	for (const line of splits) {
		const next: Pt[][] = [];
		for (const poly of polys) {
			if (poly.length < 3) continue;
			const [a, b] = splitPolyByLine(poly, line.ax, line.ay, line.bx, line.by);
			if (a.length >= 3 && Math.abs(polyArea(a)) > 0.01) next.push(a);
			if (b.length >= 3 && Math.abs(polyArea(b)) > 0.01) next.push(b);
		}
		if (next.length > 0) polys = next;
	}
	const insetAmt =
		settlement.kind === "City"
			? 0.042
			: settlement.kind === "Town"
				? 0.048
				: 0.055;
	const centerDistrict =
		settlement.districts.find((d) => d.kind === "Center") ?? null;
	const blocks: CityBlock[] = [];
	let centerIdx = -1;
	let centerBest = Infinity;
	for (const poly of polys) {
		if (Math.abs(polyArea(poly)) < 0.014) continue;
		const c = centroid(poly);
		const inset = insetPoly(poly, insetAmt);
		if (inset.length < 3 || Math.abs(polyArea(inset)) < 0.006) continue;
		let district: District | null = null;
		let best = Infinity;
		for (const d of settlement.districts) {
			if (d.kind === "Center") continue;
			const site = districtSite(settlement, d);
			const dist = (site.x - c.x) ** 2 + (site.y - c.y) ** 2;
			if (dist < best) {
				best = dist;
				district = d;
			}
		}
		const d0 = c.x * c.x + c.y * c.y;
		if (centerDistrict && d0 < centerBest) {
			centerBest = d0;
			centerIdx = blocks.length;
		}
		blocks.push({ poly: inset, district });
	}
	if (centerIdx >= 0 && centerDistrict && centerBest < 0.14) {
		blocks[centerIdx].district = centerDistrict;
	}
	return blocks;
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
	for (const block of settlementBlocks(settlement)) {
		if (pointInPoly(nx, ny, block.poly)) {
			return block.district;
		}
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
		const { nx, ny } = toLocalNorm(px, py, sx, sy, r);
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
			if (biome === null || !isWaterBiome(biome)) return { x, y };
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
			if (biome !== null && isWaterBiome(biome)) data[i + 3] = 0;
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
	const { stretch, rot } = footprintFrame();
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
	octx.fillStyle = STREET_BED;
	octx.fill();
	const blocks = settlementBlocks(settlement);
	for (const block of blocks) {
		if (block.poly.length < 3) continue;
		tracePoly(octx, block.poly, R);
		octx.fillStyle = block.district
			? DISTRICT_INFO[block.district.kind].fill
			: info.fill;
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
			for (const block of settlementBlocks(settlement)) {
				if (block.district?.name !== hoveredDistrict.name) continue;
				tracePoly(ctx, block.poly, R);
				ctx.strokeStyle = "rgba(255, 255, 255, 0.85)";
				ctx.lineWidth = 1.4;
				ctx.stroke();
			}
		} else {
			traceVerts(ctx, verts);
			ctx.strokeStyle = "rgba(255, 255, 255, 0.85)";
			ctx.lineWidth = 1.4;
			ctx.stroke();
		}
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

function roadInView(
	points: { x: number; y: number }[],
	bounds: WorldBounds,
): boolean {
	if (points.length === 0) return false;
	let minX = points[0].x;
	let maxX = points[0].x;
	let minY = points[0].y;
	let maxY = points[0].y;
	for (let i = 1; i < points.length; i++) {
		const p = points[i];
		if (p.x < minX) minX = p.x;
		if (p.x > maxX) maxX = p.x;
		if (p.y < minY) minY = p.y;
		if (p.y > maxY) maxY = p.y;
	}
	const pad = bounds.span * 0.08;
	return !(
		maxX < bounds.x0 - pad ||
		minX > bounds.x0 + bounds.span + pad ||
		maxY < bounds.y0 - pad ||
		minY > bounds.y0 + bounds.span + pad
	);
}

function strokeRoadPath(
	ctx: CanvasRenderingContext2D,
	points: { x: number; y: number }[],
	bounds: WorldBounds,
	canvasWidth: number,
	canvasHeight: number,
) {
	ctx.beginPath();
	for (let i = 0; i < points.length; i++) {
		const sx = ((points[i].x - bounds.x0) / bounds.span) * canvasWidth;
		const sy = ((points[i].y - bounds.y0) / bounds.span) * canvasHeight;
		if (i === 0) ctx.moveTo(sx, sy);
		else ctx.lineTo(sx, sy);
	}
	ctx.stroke();
}

function roadLayerWidth(
	kind: RoadKind,
	surface: RoadSurface,
	detail: RoadDetail,
): number {
	const base =
		kind === "Highway" ? 1 : kind === "Secondary" ? 0.62 : 0.38;
	const scale =
		detail === "main" ? 2.3 : detail === "region" ? 2.5 : 4.2;
	const surfaceMul =
		surface === "Paved" ? 1 : surface === "Packed" ? 0.88 : 0.72;
	return Math.max(0.7, scale * base * surfaceMul);
}

function paintRoadStroke(
	ctx: CanvasRenderingContext2D,
	road: Road,
	bounds: WorldBounds,
	canvasWidth: number,
	canvasHeight: number,
	detail: RoadDetail,
) {
	const width = roadLayerWidth(road.kind, road.surface, detail);
	ctx.setLineDash([]);
	if (road.surface === "Paved") {
		ctx.strokeStyle = "rgba(42, 28, 14, 0.72)";
		ctx.lineWidth = width + (detail === "close" ? 1.6 : 0.9);
		strokeRoadPath(ctx, road.points, bounds, canvasWidth, canvasHeight);
		ctx.strokeStyle =
			detail === "close" ? "rgba(214, 176, 110, 0.95)" : "rgba(196, 163, 106, 0.88)";
		ctx.lineWidth = width;
		strokeRoadPath(ctx, road.points, bounds, canvasWidth, canvasHeight);
		return;
	}
	if (road.surface === "Packed") {
		ctx.strokeStyle =
			detail === "close" ? "rgba(122, 90, 54, 0.88)" : "rgba(110, 82, 50, 0.78)";
		ctx.lineWidth = width;
		strokeRoadPath(ctx, road.points, bounds, canvasWidth, canvasHeight);
		return;
	}
	ctx.strokeStyle =
		detail === "close" ? "rgba(86, 68, 46, 0.8)" : "rgba(86, 68, 46, 0.62)";
	ctx.lineWidth = width;
	strokeRoadPath(ctx, road.points, bounds, canvasWidth, canvasHeight);
}

export function drawRoads(
	ctx: CanvasRenderingContext2D,
	canvasWidth: number,
	canvasHeight: number,
	roads: Road[],
	bounds: WorldBounds,
	detail: RoadDetail,
) {
	ctx.save();
	ctx.lineCap = "round";
	ctx.lineJoin = "round";
	const order: RoadKind[] = ["Local", "Secondary", "Highway"];
	for (const kind of order) {
		for (const road of roads) {
			if (road.kind !== kind) continue;
			if (detail === "main" && road.kind !== "Highway") continue;
			if (road.points.length < 2 || !roadInView(road.points, bounds)) continue;
			paintRoadStroke(ctx, road, bounds, canvasWidth, canvasHeight, detail);
		}
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

	for (const kind of DRAW_ORDER) {
		for (const settlement of settlements) {
			if (settlement.kind !== kind) continue;
			if (!settlementIntersectsBounds(settlement, bounds)) continue;
			const sx = ((settlement.x - bounds.x0) / bounds.span) * canvasWidth;
			const sy = ((settlement.y - bounds.y0) / bounds.span) * canvasHeight;

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
