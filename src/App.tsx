import { useState } from "react";
import type { RegionId } from "./api/world";
import { SeedControl } from "./components/Control/SeedControl";
import { GlobalMapView } from "./components/Map/GlobalMapView";
import { RegionMapView } from "./components/Map/RegionMapView";
import { useRegionMap } from "./hooks/useRegionMap";
import { useWorldMap } from "./hooks/useWorldMap";
import "./styles/MapScreen.css";
import "./styles/App.css";

function App() {
	const [seed, setSeed] = useState(5);
	const [draftSeed, setDraftSeed] = useState("5");
	const [selectedRegion, setSelectedRegion] = useState<RegionId | null>(null);
	const [seedError, setSeedError] = useState<string | null>(null);

	const { grid, loading, error } = useWorldMap(seed);
	const regionMap = useRegionMap(seed, selectedRegion);

	function applySeed() {
		const parsedSeed = Number(draftSeed);
		if (!Number.isSafeInteger(parsedSeed) || parsedSeed < 0) {
			setSeedError("Seed musi być nieujemną liczbą całkowitą.");
			return;
		}

		setSeed(parsedSeed);
		setSeedError(null);
		setSelectedRegion(null);
	}

	return (
		<main className="map-screen">
			<div className="map-area">
				{selectedRegion ? (
					<RegionMapView
						region={selectedRegion}
						grid={regionMap.grid}
						onBack={() => setSelectedRegion(null)}
					/>
				) : (
					<GlobalMapView grid={grid} onRegionSelect={setSelectedRegion} />
				)}
			</div>

			<aside className="side-panel">
				<SeedControl
					seed={draftSeed}
					onSeedChange={setDraftSeed}
					onRegenerate={applySeed}
					loading={loading || regionMap.loading}
				/>
				{seedError && <p className="map-error">{seedError}</p>}
				{error && <p className="map-error">{error}</p>}
				{regionMap.error && <p className="map-error">{regionMap.error}</p>}
				{(loading || regionMap.loading) && (
					<p className="map-loading">Generowanie mapy…</p>
				)}
			</aside>
		</main>
	);
}

export default App;
