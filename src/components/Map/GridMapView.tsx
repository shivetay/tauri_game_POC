import { type MouseEvent, type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import type { GridCell, Road, Settlement, TerrainGrid, WorldBounds } from "../../api/world";
import { canvasClientToPixel, pixelToCell } from "../../api/world";
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
}: GridMapViewProps) {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const baseCanvasRef = useRef<HTMLCanvasElement | null>(null);
	const hoverRef = useRef<GridCell | null>(null);
	const settlementHoverRef = useRef<SettlementHit | null>(null);
	const [hoveredCell, setHoveredCell] = useState<GridCell | null>(null);
	const [hoveredHit, setHoveredHit] = useState<SettlementHit | null>(null);

	const paint = useCallback(
		(hover: GridCell | null, settlementHover: SettlementHit | null) => {
			const canvas = canvasRef.current;
			const base = baseCanvasRef.current;
			if (!canvas || !base) return;

			const ctx = canvas.getContext("2d");
			if (!ctx) return;

			ctx.drawImage(base, 0, 0);
			if (showGrid) {
				drawRegionGrid(ctx, canvas.width, canvas.height, cellSize);
			}
			if (hover && trackCells) {
				drawCellHighlight(ctx, hover.cx, hover.cy, cellSize);
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
					);
				}
				if (settlements.length > 0) {
					drawSettlements(
						ctx,
						canvas.width,
						canvas.height,
						settlements,
						worldBounds,
						settlementHover,
						settlementStyle,
						grid,
					);
				}
			}
		},
		[cellSize, grid, roadDetail, roads, settlements, settlementStyle, showGrid, trackCells, worldBounds],
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
		hoverRef.current = null;
		settlementHoverRef.current = null;
		setHoveredCell(null);
		setHoveredHit(null);
		paint(null, null);
	}, [grid, paint]);

	function pointerInfo(e: MouseEvent<HTMLCanvasElement>): {
		cell: GridCell | null;
		hit: SettlementHit | null;
	} {
		const canvas = canvasRef.current;
		if (!canvas) return { cell: null, hit: null };

		const pixel = canvasClientToPixel(e.clientX, e.clientY, canvas);
		if (!pixel) return { cell: null, hit: null };

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

		return { cell, hit };
	}

	function clearHover() {
		if (!hoverRef.current && !settlementHoverRef.current) return;

		hoverRef.current = null;
		settlementHoverRef.current = null;
		setHoveredCell(null);
		setHoveredHit(null);
		paint(null, null);
	}

	function handleMouseMove(e: MouseEvent<HTMLCanvasElement>) {
		const { cell, hit } = pointerInfo(e);
		if (!cell && !hit) {
			clearHover();
			return;
		}

		const prevCell = hoverRef.current;
		const prevHit = settlementHoverRef.current;
		const sameCell =
			(prevCell?.cx === cell?.cx && prevCell?.cy === cell?.cy) ||
			(!prevCell && !cell);
		const sameHit =
			prevHit?.settlement.x === hit?.settlement.x &&
			prevHit?.settlement.y === hit?.settlement.y &&
			prevHit?.settlement.kind === hit?.settlement.kind &&
			prevHit?.district?.name === hit?.district?.name;

		if (sameCell && sameHit) return;

		hoverRef.current = cell;
		settlementHoverRef.current = hit;
		setHoveredCell(cell);
		setHoveredHit(hit);
		paint(cell, hit);
	}

	function handleMouseLeave() {
		clearHover();
	}

	function handleClick(e: MouseEvent<HTMLCanvasElement>) {
		const { cell } = pointerInfo(e);
		if (cell) onCellSelect?.(cell);
	}

	const status = hoveredHit
		? formatSettlementLabel(hoveredHit.settlement, hoveredHit.district)
		: hoveredCell
			? hoverLabel(hoveredCell)
			: idleLabel;

	return (
		<div className="map-view">
			<div className="map-view-header">
				{header}
				<p>{status}</p>
			</div>
			<canvas
				ref={canvasRef}
				className="map-canvas"
				onClick={onCellSelect ? handleClick : undefined}
				onMouseMove={handleMouseMove}
				onMouseLeave={handleMouseLeave}
				style={{ cursor: onCellSelect ? "crosshair" : "default" }}
			/>
		</div>
	);
}
