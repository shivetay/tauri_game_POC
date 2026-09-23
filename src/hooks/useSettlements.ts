import { useCallback, useEffect, useState } from "react";
import {
	generateSettlements,
	type Road,
	type Settlement,
	type TerrainParams,
} from "../api/world";

export function useSettlements(seed: number, params: TerrainParams) {
	const [settlements, setSettlements] = useState<Settlement[]>([]);
	const [roads, setRoads] = useState<Road[]>([]);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const reload = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			const result = await generateSettlements(seed, params);
			setSettlements(result.settlements);
			setRoads(result.roads);
		} catch (e) {
			setError(String(e));
			setSettlements([]);
			setRoads([]);
		} finally {
			setLoading(false);
		}
	}, [seed, params]);

	useEffect(() => {
		reload();
	}, [reload]);

	return { settlements, roads, loading, error, reload };
}
