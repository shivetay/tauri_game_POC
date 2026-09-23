import {
	CHUNKS_PER_REGION,
	regionWorldBounds,
	type ChunkId,
	type RegionEcologyMap,
	type RegionId,
	type Road,
	type Settlement,
	type TerrainGrid,
} from "../../api/world";
import { GridMapView } from "./GridMapView";

interface RegionMapViewProps {
	region: RegionId;
	grid: TerrainGrid | null;
	onBack: () => void;
	onChunkSelect: (chunk: ChunkId) => void;
	settlements: Settlement[];
	roads: Road[];
	habitat: RegionEcologyMap | null;
}

export function RegionMapView({
	region,
	grid,
	onBack,
	onChunkSelect,
	settlements,
	roads,
	habitat,
}: RegionMapViewProps) {
	const cellSize = grid ? grid.width / CHUNKS_PER_REGION : 64;

	return (
		<GridMapView
			grid={grid}
			cellSize={cellSize}
			idleLabel={`Region (${region.rx}, ${region.ry}) — kliknij obszar · kliknij miasto po info`}
			hoverLabel={(cell) =>
				`Obszar (${cell.cx}, ${cell.cy}) — kliknij, aby powiększyć`
			}
			onCellSelect={(cell) => onChunkSelect({ cx: cell.cx, cy: cell.cy })}
			header={
				<button type="button" onClick={onBack}>
					← Mapa świata
				</button>
			}
			settlements={settlements}
			roads={roads}
			roadDetail="region"
			worldBounds={regionWorldBounds(region)}
			habitat={habitat}
		/>
	);
}
