import { useCallback, useEffect, useState } from "react";
import {
	generateChunk,
	type ChunkId,
	type RegionId,
	type TerrainGrid,
	type TerrainParams,
} from "../api/world";

export function useChunkMap(
	seed: number,
	params: TerrainParams,
	region: RegionId | null,
	chunk: ChunkId | null,
) {
	const [grid, setGrid] = useState<TerrainGrid | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const reload = useCallback(async () => {
		if (!region || !chunk) return;
		setLoading(true);
		setError(null);
		try {
			const result = await generateChunk(
				seed,
				params,
				region.rx,
				region.ry,
				chunk.cx,
				chunk.cy,
			);
			setGrid(result);
		} catch (e) {
			setError(String(e));
		} finally {
			setLoading(false);
		}
	}, [seed, params, region, chunk]);

	useEffect(() => {
		reload();
	}, [reload]);

	return { grid, loading, error, reload };
}
