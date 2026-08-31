import { type MouseEvent, type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import type { GridCell, TerrainGrid } from "../../api/world";
import { canvasClientToPixel, pixelToCell } from "../../api/world";
import {
	drawCellHighlight,
	drawRegionGrid,
	terrainGridToImageData,
} from "../../map/render";

interface GridMapViewProps {
	grid: TerrainGrid | null;
	cellSize: number;
	idleLabel: string;
	hoverLabel: (cell: GridCell) => string;
	onCellSelect?: (cell: GridCell) => void;
	header?: ReactNode;
}

export function GridMapView({
	grid,
	cellSize,
	idleLabel,
	hoverLabel,
	onCellSelect,
	header,
}: GridMapViewProps) {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const baseCanvasRef = useRef<HTMLCanvasElement | null>(null);
	const hoverRef = useRef<GridCell | null>(null);
	const [hoveredCell, setHoveredCell] = useState<GridCell | null>(null);

	const paint = useCallback(
		(hover: GridCell | null) => {
			const canvas = canvasRef.current;
			const base = baseCanvasRef.current;
			if (!canvas || !base) return;

			const ctx = canvas.getContext("2d");
			if (!ctx) return;

			ctx.drawImage(base, 0, 0);
			drawRegionGrid(ctx, canvas.width, canvas.height, cellSize);
			if (hover) {
				drawCellHighlight(ctx, hover.cx, hover.cy, cellSize);
			}
		},
		[cellSize],
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
		setHoveredCell(null);
		paint(null);
	}, [grid, paint]);

	function pointerToCell(e: MouseEvent<HTMLCanvasElement>): GridCell | null {
		const canvas = canvasRef.current;
		if (!canvas) return null;

		const pixel = canvasClientToPixel(e.clientX, e.clientY, canvas);
		if (!pixel) return null;

		return pixelToCell(
			pixel.px,
			pixel.py,
			canvas.width,
			canvas.height,
			cellSize,
		);
	}

	function clearHover() {
		if (!hoverRef.current) return;

		hoverRef.current = null;
		setHoveredCell(null);
		paint(null);
	}

	function handleMouseMove(e: MouseEvent<HTMLCanvasElement>) {
		const cell = pointerToCell(e);
		if (!cell) {
			clearHover();
			return;
		}

		const prev = hoverRef.current;
		if (prev?.cx === cell.cx && prev?.cy === cell.cy) return;

		hoverRef.current = cell;
		setHoveredCell(cell);
		paint(cell);
	}

	function handleMouseLeave() {
		clearHover();
	}

	function handleClick(e: MouseEvent<HTMLCanvasElement>) {
		const cell = pointerToCell(e);
		if (cell) onCellSelect?.(cell);
	}

	return (
		<div className="map-view">
			<div className="map-view-header">
				{header}
				<p>{hoveredCell ? hoverLabel(hoveredCell) : idleLabel}</p>
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
