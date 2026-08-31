import { useCallback, useEffect, useState } from "react";
import {
	DEFAULT_TERRAIN_PARAMS,
	generateGlobal,
	type TerrainGrid,
	type TerrainParams,
} from "../api/world";

export function useWorldMap(seed: number, params: TerrainParams) {
	const [grid, setGrid] = useState<TerrainGrid | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const reload = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			const result = await generateGlobal(seed, params);
			setGrid(result);
		} catch (e) {
			setError(String(e));
		} finally {
			setLoading(false);
		}
	}, [seed, params]);

	useEffect(() => {
		reload();
	}, [reload]);

	return { grid, loading, error, reload };
}

export { DEFAULT_TERRAIN_PARAMS };
