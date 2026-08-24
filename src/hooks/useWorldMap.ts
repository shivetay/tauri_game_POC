import { useCallback, useEffect, useState } from "react";
import { generateGlobal, type TerrainGrid } from "../api/world";

export function useWorldMap(seed: number) {
	const [grid, setGrid] = useState<TerrainGrid | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const reload = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			const result = await generateGlobal(seed);
			setGrid(result);
		} catch (e) {
			setError(String(e));
		} finally {
			setLoading(false);
		}
	}, [seed]);

	useEffect(() => {
		reload();
	}, [reload]);

	return { grid, loading, error, reload };
}
