import { useCallback, useEffect, useState } from "react";
import {
	generateView,
	type ChunkId,
	type RegionId,
	type TerrainGrid,
	type TerrainParams,
	type WorldBounds,
} from "../api/world";

export function useChunkMap(
	seed: number,
	params: TerrainParams,
	region: RegionId | null,
	chunk: ChunkId | null,
	worldBounds: WorldBounds | null,
) {
	const [grid, setGrid] = useState<TerrainGrid | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const x0 = worldBounds?.x0 ?? null;
	const y0 = worldBounds?.y0 ?? null;
	const span = worldBounds?.span ?? null;

	const reload = useCallback(async () => {
		if (!region || !chunk || x0 === null || y0 === null || span === null) return;
		setLoading(true);
		setError(null);
		setGrid(null);
		try {
			const result = await generateView(
				seed,
				params,
				region.rx,
				region.ry,
				x0,
				y0,
				span,
			);
			setGrid(result);
		} catch (e) {
			setError(String(e));
		} finally {
			setLoading(false);
		}
	}, [seed, params, region, chunk, x0, y0, span]);

	useEffect(() => {
		reload();
	}, [reload]);

	return { grid, loading, error, reload };
}
