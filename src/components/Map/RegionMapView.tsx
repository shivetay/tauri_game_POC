import { useEffect, useRef } from "react";
import type { RegionId, TerrainGrid } from "../../api/world";
import { terrainGridToImageData } from "../../map/render";

interface RegionMapViewProps {
	region: RegionId;
	grid: TerrainGrid | null;
	onBack: () => void;
}

export function RegionMapView({ region, grid, onBack }: RegionMapViewProps) {
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

	return (
		<div className="map-view">
			<div className="map-view-header">
				<button type="button" onClick={onBack}>
					← Back to world
				</button>
				<p>
					Region ({region.rx}, {region.ry}) — więcej detali
				</p>
			</div>
			<canvas ref={canvasRef} className="map-canvas" />
		</div>
	);
}
