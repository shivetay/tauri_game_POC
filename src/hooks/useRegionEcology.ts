import { useCallback, useEffect, useState } from "react";
import {
	generateRegionEcology,
	type RegionEcologyMap,
	type RegionId,
	type TerrainParams,
} from "../api/world";

export function useRegionEcology(
	seed: number,
	params: TerrainParams,
	region: RegionId | null,
) {
	const [habitat, setHabitat] = useState<RegionEcologyMap | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const reload = useCallback(async () => {
		if (!region) {
			setHabitat(null);
			return;
		}
		setLoading(true);
		setError(null);
		try {
			const result = await generateRegionEcology(
				seed,
				params,
				region.rx,
				region.ry,
			);
			setHabitat(result);
		} catch (e) {
			setError(String(e));
			setHabitat(null);
		} finally {
			setLoading(false);
		}
	}, [seed, params, region]);

	useEffect(() => {
		reload();
	}, [reload]);

	return { habitat, loading, error, reload };
}
