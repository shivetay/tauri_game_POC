import { useEffect, useRef } from "react";
import type { ChunkId, RegionId, TerrainGrid } from "../../api/world";
import { terrainGridToImageData } from "../../map/render";

interface ChunkMapViewProps {
	region: RegionId;
	chunk: ChunkId;
	grid: TerrainGrid | null;
	onBack: () => void;
}

export function ChunkMapView({ region, chunk, grid, onBack }: ChunkMapViewProps) {
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
					← Region ({region.rx}, {region.ry})
				</button>
				<p>
					Obszar ({chunk.cx}, {chunk.cy}) w regionie ({region.rx}, {region.ry}) — maks.
					przybliżenie
				</p>
			</div>
			<canvas ref={canvasRef} className="map-canvas" />
		</div>
	);
}
