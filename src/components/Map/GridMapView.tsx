import {
	type MouseEvent,
	type ReactNode,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import type {
	ChunkEcologyMap,
	GridCell,
	RegionEcologyMap,
	Road,
	Settlement,
	TerrainGrid,
	TileType,
	WorldBounds,
} from "../../api/world";
import { canvasClientToPixel, pixelToCell } from "../../api/world";
import { BIOMES } from "../../map/colors";
import {
	chunkHabitatFromLife,
	drawHabitatOverlay,
	lifeAreaSummary,
	regionTileSummary,
} from "../../map/ecology";
import {
	drawCellHighlight,
	drawRegionGrid,
	terrainGridToImageData,
} from "../../map/render";
import {
	drawRoads,
	drawSettlements,
	formatSettlementLabel,
	hitSettlement,
	type RoadDetail,
	type SettlementDrawStyle,
	type SettlementHit,
} from "../../map/settlements";

interface GridMapViewProps {
	grid: TerrainGrid | null;
	cellSize: number;
	idleLabel: string;
	hoverLabel: (cell: GridCell) => string;
	onCellSelect?: (cell: GridCell) => void;
	header?: ReactNode;
	showGrid?: boolean;
	trackCells?: boolean;
	settlements?: Settlement[];
	roads?: Road[];
	roadDetail?: RoadDetail;
	worldBounds?: WorldBounds;
	settlementStyle?: SettlementDrawStyle;
	/** Region LOD: potential occurrence wash (A+B). */
	habitat?: RegionEcologyMap | null;
	/** Chunk LOD: color wash + species detail on click. */
	life?: ChunkEcologyMap | null;
}

function biomeAtPixel(
	grid: TerrainGrid,
	px: number,
	py: number,
	canvasWidth: number,
	canvasHeight: number,
): TileType | null {
	if (grid.width === 0 || grid.height === 0 || grid.cells.length === 0) {
		return null;
	}
	const gx = Math.min(
		grid.width - 1,
		Math.max(0, Math.floor((px / canvasWidth) * grid.width)),
	);
	const gy = Math.min(
		grid.height - 1,
		Math.max(0, Math.floor((py / canvasHeight) * grid.height)),
	);
	return grid.cells[gy * grid.width + gx]?.biome ?? null;
}

function formatBiomeLabel(biome: TileType): string {
	return `Biom: ${BIOMES[biome].label}`;
}

export function GridMapView({
	grid,
	cellSize,
	idleLabel,
	hoverLabel,
	onCellSelect,
	header,
	showGrid = true,
	trackCells = true,
	settlements = [],
	roads = [],
	roadDetail = "region",
	worldBounds,
	settlementStyle = "plan",
	habitat = null,
	life = null,
}: GridMapViewProps) {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const baseCanvasRef = useRef<HTMLCanvasElement | null>(null);
	const habitatCanvasRef = useRef<HTMLCanvasElement | null>(null);
	const selectedCellRef = useRef<GridCell | null>(null);
	const selectedHitRef = useRef<SettlementHit | null>(null);
	const [selectedInfo, setSelectedInfo] = useState<string | null>(null);

	const paint = useCallback(
		(selectedCell: GridCell | null, selectedHit: SettlementHit | null) => {
			const canvas = canvasRef.current;
			const base = baseCanvasRef.current;
			if (!canvas || !base) return;

			const ctx = canvas.getContext("2d");
			if (!ctx) return;

			ctx.drawImage(base, 0, 0);
			const habitatLayer = habitatCanvasRef.current;
			if (habitatLayer) {
				ctx.drawImage(habitatLayer, 0, 0);
			}
			if (showGrid) {
				drawRegionGrid(ctx, canvas.width, canvas.height, cellSize);
			}
			if (selectedCell && trackCells) {
				drawCellHighlight(ctx, selectedCell.cx, selectedCell.cy, cellSize);
			}
			if (worldBounds) {
				if (roads.length > 0) {
					drawRoads(
						ctx,
						canvas.width,
						canvas.height,
						roads,
						worldBounds,
						roadDetail,
						grid,
					);
				}
				if (settlements.length > 0) {
					drawSettlements(
						ctx,
						canvas.width,
						canvas.height,
						settlements,
						worldBounds,
						selectedHit,
						settlementStyle,
						grid,
					);
				}
			}
		},
		[
			cellSize,
			grid,
			roadDetail,
			roads,
			settlements,
			settlementStyle,
			showGrid,
			trackCells,
			worldBounds,
		],
	);

	useEffect(() => {
		if (!grid || !canvasRef.current) return;

		const canvas = canvasRef.current;
		canvas.width = grid.width;
		canvas.height = grid.height;

		const base = document.createElement("canvas");
		base.width = grid.width;
		base.height = grid.height;
		const baseCtx = base.getContext("2d");
		if (!baseCtx) return;

		baseCtx.putImageData(terrainGridToImageData(grid), 0, 0);
		baseCanvasRef.current = base;

		const wash =
			habitat ??
			(life && life.cells.length > 0 ? chunkHabitatFromLife(life) : null);
		if (wash) {
			const layer = document.createElement("canvas");
			layer.width = grid.width;
			layer.height = grid.height;
			const layerCtx = layer.getContext("2d");
			if (layerCtx) {
				drawHabitatOverlay(layerCtx, grid.width, grid.height, wash);
				habitatCanvasRef.current = layer;
			} else {
				habitatCanvasRef.current = null;
			}
		} else {
			habitatCanvasRef.current = null;
		}

		selectedCellRef.current = null;
		selectedHitRef.current = null;
		setSelectedInfo(null);
		paint(null, null);
	}, [grid, habitat, life, paint]);

	function pointerInfo(e: MouseEvent<HTMLCanvasElement>): {
		cell: GridCell | null;
		hit: SettlementHit | null;
		pixel: { px: number; py: number } | null;
	} {
		const canvas = canvasRef.current;
		if (!canvas) {
			return { cell: null, hit: null, pixel: null };
		}

		const pixel = canvasClientToPixel(e.clientX, e.clientY, canvas);
		if (!pixel) {
			return { cell: null, hit: null, pixel: null };
		}

		const hit =
			worldBounds && settlements.length > 0
				? hitSettlement(
						pixel.px,
						pixel.py,
						canvas.width,
						canvas.height,
						settlements,
						worldBounds,
						settlementStyle,
					)
				: null;
		const cell = trackCells
			? pixelToCell(
					pixel.px,
					pixel.py,
					canvas.width,
					canvas.height,
					cellSize,
				)
			: null;

		return { cell, hit, pixel };
	}

	function buildClickInfo(
		cell: GridCell | null,
		hit: SettlementHit | null,
		pixel: { px: number; py: number },
		canvas: HTMLCanvasElement,
	): string {
		if (hit) {
			return formatSettlementLabel(hit.settlement, hit.district);
		}

		const parts: string[] = [];
		if (grid) {
			const biome = biomeAtPixel(
				grid,
				pixel.px,
				pixel.py,
				canvas.width,
				canvas.height,
			);
			if (biome) {
				parts.push(formatBiomeLabel(biome));
			}
		}

		if (life && worldBounds) {
			const lifeHint = lifeAreaSummary(
				life,
				canvas.width,
				canvas.height,
				pixel.px,
				pixel.py,
				worldBounds,
			);
			if (lifeHint) parts.push(lifeHint);
		} else if (habitat && cell) {
			const tileHint = regionTileSummary(habitat, cell.cx, cell.cy);
			if (tileHint) parts.push(tileHint);
		} else if (cell) {
			parts.push(hoverLabel(cell));
		}

		return parts.length > 0 ? parts.join(" · ") : idleLabel;
	}

	function handleClick(e: MouseEvent<HTMLCanvasElement>) {
		const canvas = canvasRef.current;
		const { cell, hit, pixel } = pointerInfo(e);
		if (!canvas || !pixel) return;

		// Settlement / place: show info, do not zoom.
		if (hit) {
			const info = formatSettlementLabel(hit.settlement, hit.district);
			selectedCellRef.current = null;
			selectedHitRef.current = hit;
			setSelectedInfo(info);
			paint(null, hit);
			return;
		}

		// Navigable LOD: single click zooms (global → region → chunk).
		if (onCellSelect && cell) {
			onCellSelect(cell);
			return;
		}

		// Chunk (no further zoom): biome + flora/fauna on click.
		const info = buildClickInfo(cell, null, pixel, canvas);
		selectedCellRef.current = cell;
		selectedHitRef.current = null;
		setSelectedInfo(info);
		paint(cell, null);
	}

	const status = selectedInfo ?? idleLabel;
	const canNavigate = Boolean(onCellSelect);

	return (
		<div className="map-view">
			<div className="map-view-header">
				{header}
				<p>{status}</p>
			</div>
			<canvas
				ref={canvasRef}
				className="map-canvas"
				onClick={handleClick}
				style={{ cursor: canNavigate ? "crosshair" : "default" }}
			/>
		</div>
	);
}
