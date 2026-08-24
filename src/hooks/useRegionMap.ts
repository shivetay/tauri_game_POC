import { useCallback, useEffect, useState } from "react";
import { generateRegion, type RegionId, type TerrainGrid } from "../api/world";

export function useRegionMap(seed: number, region: RegionId | null) {
	const [grid, setGrid] = useState<TerrainGrid | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const reload = useCallback(async () => {
		if (!region) return;
		setLoading(true);
		setError(null);
		try {
			const result = await generateRegion(seed, region.rx, region.ry);
			setGrid(result);
		} catch (e) {
			setError(String(e));
		} finally {
			setLoading(false);
		}
	}, [seed, region]);

	useEffect(() => {
		reload();
	}, [reload]);

	return { grid, loading, error, reload };
}
