import {
	CHUNK_RESOLUTION,
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
}

export function ChunkMapView({
	region,
	chunk,
	grid,
	onBack,
	settlements,
	roads,
	worldBounds,
}: ChunkMapViewProps) {
	const cellSize = grid?.width ?? CHUNK_RESOLUTION;

	return (
		<GridMapView
			grid={grid}
			cellSize={cellSize}
			showGrid={false}
			trackCells={false}
			idleLabel={`Obszar (${chunk.cx}, ${chunk.cy}) w regionie (${region.rx}, ${region.ry}) — maks. przybliżenie`}
			hoverLabel={() =>
				`Obszar (${chunk.cx}, ${chunk.cy}) w regionie (${region.rx}, ${region.ry}) — maks. przybliżenie`
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
		/>
	);
}
