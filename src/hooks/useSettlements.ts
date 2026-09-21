import { useCallback, useEffect, useState } from "react";
import {
	generateSettlements,
	type Settlement,
	type TerrainParams,
} from "../api/world";

export function useSettlements(seed: number, params: TerrainParams) {
	const [settlements, setSettlements] = useState<Settlement[]>([]);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const reload = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			const result = await generateSettlements(seed, params);
			setSettlements(result);
		} catch (e) {
			setError(String(e));
			setSettlements([]);
		} finally {
			setLoading(false);
		}
	}, [seed, params]);

	useEffect(() => {
		reload();
	}, [reload]);

	return { settlements, loading, error, reload };
}
