import {
	CHUNK_RESOLUTION,
	chunkWorldBounds,
	type ChunkId,
	type RegionId,
	type Settlement,
	type TerrainGrid,
} from "../../api/world";
import { GridMapView } from "./GridMapView";

interface ChunkMapViewProps {
	region: RegionId;
	chunk: ChunkId;
	grid: TerrainGrid | null;
	onBack: () => void;
	settlements: Settlement[];
}

export function ChunkMapView({
	region,
	chunk,
	grid,
	onBack,
	settlements,
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
			worldBounds={chunkWorldBounds(region, chunk)}
		/>
	);
}
