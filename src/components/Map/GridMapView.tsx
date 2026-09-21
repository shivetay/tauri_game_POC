import { type MouseEvent, type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import type { GridCell, Settlement, TerrainGrid, WorldBounds } from "../../api/world";
import { canvasClientToPixel, pixelToCell } from "../../api/world";
import {
	drawCellHighlight,
	drawRegionGrid,
	terrainGridToImageData,
} from "../../map/render";
import {
	drawSettlements,
	formatSettlementLabel,
	hitSettlement,
	type SettlementDrawStyle,
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
	worldBounds,
	settlementStyle = "plan",
}: GridMapViewProps) {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const baseCanvasRef = useRef<HTMLCanvasElement | null>(null);
	const hoverRef = useRef<GridCell | null>(null);
	const settlementHoverRef = useRef<Settlement | null>(null);
	const [hoveredCell, setHoveredCell] = useState<GridCell | null>(null);
	const [hoveredSettlement, setHoveredSettlement] = useState<Settlement | null>(
		null,
	);

	const paint = useCallback(
		(hover: GridCell | null, settlementHover: Settlement | null) => {
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
			if (worldBounds && settlements.length > 0) {
				drawSettlements(
					ctx,
					canvas.width,
					canvas.height,
					settlements,
					worldBounds,
					settlementHover,
					settlementStyle,
				);
			}
		},
		[cellSize, settlements, settlementStyle, showGrid, trackCells, worldBounds],
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
		setHoveredSettlement(null);
		paint(null, null);
	}, [grid, paint]);

	function pointerInfo(e: MouseEvent<HTMLCanvasElement>): {
		cell: GridCell | null;
		settlement: Settlement | null;
	} {
		const canvas = canvasRef.current;
		if (!canvas) return { cell: null, settlement: null };

		const pixel = canvasClientToPixel(e.clientX, e.clientY, canvas);
		if (!pixel) return { cell: null, settlement: null };

		const settlement =
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

		return { cell, settlement };
	}

	function clearHover() {
		if (!hoverRef.current && !settlementHoverRef.current) return;

		hoverRef.current = null;
		settlementHoverRef.current = null;
		setHoveredCell(null);
		setHoveredSettlement(null);
		paint(null, null);
	}

	function handleMouseMove(e: MouseEvent<HTMLCanvasElement>) {
		const { cell, settlement } = pointerInfo(e);
		if (!cell && !settlement) {
			clearHover();
			return;
		}

		const prevCell = hoverRef.current;
		const prevSettlement = settlementHoverRef.current;
		const sameCell =
			(prevCell?.cx === cell?.cx && prevCell?.cy === cell?.cy) ||
			(!prevCell && !cell);
		const sameSettlement =
			prevSettlement?.x === settlement?.x &&
			prevSettlement?.y === settlement?.y &&
			prevSettlement?.kind === settlement?.kind;

		if (sameCell && sameSettlement) return;

		hoverRef.current = cell;
		settlementHoverRef.current = settlement;
		setHoveredCell(cell);
		setHoveredSettlement(settlement);
		paint(cell, settlement);
	}

	function handleMouseLeave() {
		clearHover();
	}

	function handleClick(e: MouseEvent<HTMLCanvasElement>) {
		const { cell } = pointerInfo(e);
		if (cell) onCellSelect?.(cell);
	}

	const status = hoveredSettlement
		? formatSettlementLabel(hoveredSettlement)
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
