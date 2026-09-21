import {
	CHUNKS_PER_REGION,
	globalWorldBounds,
	type RegionId,
	type Settlement,
	type TerrainGrid,
} from "../../api/world";
import { GridMapView } from "./GridMapView";

interface GlobalMapViewProps {
	grid: TerrainGrid | null;
	onRegionSelect: (region: RegionId) => void;
	settlements: Settlement[];
}

export function GlobalMapView({
	grid,
	onRegionSelect,
	settlements,
}: GlobalMapViewProps) {
	const cellSize = grid ? grid.width / CHUNKS_PER_REGION : 64;
	const overview = settlements.filter(
		(settlement) =>
			settlement.kind === "City" || settlement.kind === "Town",
	);

	return (
		<GridMapView
			grid={grid}
			cellSize={cellSize}
			idleLabel={`Mapa świata — siatka ${CHUNKS_PER_REGION}×${CHUNKS_PER_REGION}`}
			hoverLabel={(cell) =>
				`Region (${cell.cx}, ${cell.cy}) — kliknij, aby powiększyć`
			}
			onCellSelect={(cell) => onRegionSelect({ rx: cell.cx, ry: cell.cy })}
			settlements={overview}
			worldBounds={globalWorldBounds()}
			settlementStyle="marker"
		/>
	);
}
