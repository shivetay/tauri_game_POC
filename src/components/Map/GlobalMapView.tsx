import { type MouseEvent, useEffect, useRef } from "react";
import {
	MACRO_RESOLUTION,
	pixelToRegion,
	type RegionId,
	type TerrainGrid,
} from "../../api/world";
import { terrainGridToImageData } from "../../map/render";

interface GlobalMapViewProps {
	grid: TerrainGrid | null;
	onRegionSelect: (region: RegionId) => void;
}

export function GlobalMapView({ grid, onRegionSelect }: GlobalMapViewProps) {
	const canvasRef = useRef<HTMLCanvasElement>(null);

	useEffect(() => {
		if (!grid || !canvasRef.current) return;
		const ctx = canvasRef.current.getContext("2d");
		if (!ctx) return;
		const imageData = terrainGridToImageData(grid);
		canvasRef.current.width = grid.width;
		canvasRef.current.height = grid.height;
		ctx.putImageData(imageData, 0, 0);
	}, [grid]);

	function handleClick(e: MouseEvent<HTMLCanvasElement>) {
		const canvas = canvasRef.current;
		if (!canvas) return;
		const rect = canvas.getBoundingClientRect();
		const px = ((e.clientX - rect.left) / rect.width) * canvas.width;
		const py = ((e.clientY - rect.top) / rect.height) * canvas.height;
		onRegionSelect(pixelToRegion(px, py, MACRO_RESOLUTION));
	}

	return (
		<div className="map-view">
			<canvas
				ref={canvasRef}
				className="map-canvas"
				onClick={handleClick}
				title="Kliknij region, aby powiększyć"
			/>
		</div>
	);
}
