import { CHUNKS_PER_REGION, type RegionId, type TerrainGrid } from "../../api/world";
import { GridMapView } from "./GridMapView";

interface GlobalMapViewProps {
	grid: TerrainGrid | null;
	onRegionSelect: (region: RegionId) => void;
}

export function GlobalMapView({ grid, onRegionSelect }: GlobalMapViewProps) {
	const cellSize = grid ? grid.width / CHUNKS_PER_REGION : 64;

	return (
		<GridMapView
			grid={grid}
			cellSize={cellSize}
			idleLabel={`Mapa świata — siatka ${CHUNKS_PER_REGION}×${CHUNKS_PER_REGION}`}
			hoverLabel={(cell) =>
				`Region (${cell.cx}, ${cell.cy}) — kliknij, aby powiększyć`
			}
			onCellSelect={(cell) => onRegionSelect({ rx: cell.cx, ry: cell.cy })}
		/>
	);
}
