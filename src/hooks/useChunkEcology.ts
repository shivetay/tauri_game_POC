import { useCallback, useEffect, useState } from "react";
import {
	generateChunkEcology,
	type ChunkEcologyMap,
	type ChunkId,
	type RegionId,
	type TerrainParams,
} from "../api/world";

export function useChunkEcology(
	seed: number,
	params: TerrainParams,
	region: RegionId | null,
	chunk: ChunkId | null,
) {
	const [life, setLife] = useState<ChunkEcologyMap | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const reload = useCallback(async () => {
		if (!region || !chunk) {
			setLife(null);
			return;
		}
		setLoading(true);
		setError(null);
		setLife(null);
		try {
			const result = await generateChunkEcology(
				seed,
				params,
				region.rx,
				region.ry,
				chunk.cx,
				chunk.cy,
			);
			setLife(result);
		} catch (e) {
			setError(String(e));
			setLife(null);
		} finally {
			setLoading(false);
		}
	}, [seed, params, region, chunk]);

	useEffect(() => {
		reload();
	}, [reload]);

	return { life, loading, error, reload };
}
