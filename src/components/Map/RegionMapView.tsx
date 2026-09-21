import {
	CHUNKS_PER_REGION,
	regionWorldBounds,
	type ChunkId,
	type RegionId,
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
}

export function RegionMapView({
	region,
	grid,
	onBack,
	onChunkSelect,
	settlements,
}: RegionMapViewProps) {
	const cellSize = grid ? grid.width / CHUNKS_PER_REGION : 64;

	return (
		<GridMapView
			grid={grid}
			cellSize={cellSize}
			idleLabel={`Region (${region.rx}, ${region.ry}) — siatka ${CHUNKS_PER_REGION}×${CHUNKS_PER_REGION}, kliknij obszar`}
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
			worldBounds={regionWorldBounds(region)}
		/>
	);
}
