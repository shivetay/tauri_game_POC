import {
	CHUNK_RESOLUTION,
	type ChunkEcologyMap,
	type ChunkId,
	type RegionId,
	type Road,
	type Settlement,
	type TerrainGrid,
	type WorldBounds,
} from "../../api/world";
import { GridMapView } from "./GridMapView";

interface ChunkMapViewProps {
	region: RegionId;
	chunk: ChunkId;
	grid: TerrainGrid | null;
	onBack: () => void;
	settlements: Settlement[];
	roads: Road[];
	worldBounds: WorldBounds;
	life: ChunkEcologyMap | null;
}

export function ChunkMapView({
	region,
	chunk,
	grid,
	onBack,
	settlements,
	roads,
	worldBounds,
	life,
}: ChunkMapViewProps) {
	const cellSize = grid?.width ?? CHUNK_RESOLUTION;

	return (
		<GridMapView
			grid={grid}
			cellSize={cellSize}
			showGrid={false}
			trackCells={false}
			idleLabel={`Obszar (${chunk.cx}, ${chunk.cy}) — kliknij biom/miasto lub gatunki`}
			hoverLabel={() =>
				`Obszar (${chunk.cx}, ${chunk.cy}) w regionie (${region.rx}, ${region.ry})`
			}
			header={
				<button type="button" onClick={onBack}>
					← Region ({region.rx}, {region.ry})
				</button>
			}
			settlements={settlements}
			roads={roads}
			roadDetail="close"
			worldBounds={worldBounds}
			life={life}
		/>
	);
}
