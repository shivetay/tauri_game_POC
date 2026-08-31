import { useState } from "react";
import type { ChunkId, RegionId } from "./api/world";
import {
	DEFAULT_TERRAIN_PARAMS,
	type TerrainParams,
} from "./api/world";
import { SeedControl } from "./components/Control/SeedControl";
import { BiomeLegend } from "./components/Control/BiomeLegend";
import { ChunkMapView } from "./components/Map/ChunkMapView";
import { GlobalMapView } from "./components/Map/GlobalMapView";
import { RegionMapView } from "./components/Map/RegionMapView";
import { useChunkMap } from "./hooks/useChunkMap";
import { useRegionMap } from "./hooks/useRegionMap";
import { useWorldMap } from "./hooks/useWorldMap";
import "./styles/MapScreen.css";
import "./styles/App.css";

function App() {
	const [seed, setSeed] = useState(6);
	const [draftSeed, setDraftSeed] = useState("6");
	const [terrainParams, setTerrainParams] =
		useState<TerrainParams>(DEFAULT_TERRAIN_PARAMS);
	const [draftParams, setDraftParams] =
		useState<TerrainParams>(DEFAULT_TERRAIN_PARAMS);
	const [selectedRegion, setSelectedRegion] = useState<RegionId | null>(null);
	const [selectedChunk, setSelectedChunk] = useState<ChunkId | null>(null);
	const [seedError, setSeedError] = useState<string | null>(null);

	const { grid, loading, error } = useWorldMap(seed, terrainParams);
	const regionMap = useRegionMap(seed, terrainParams, selectedRegion);
	const chunkMap = useChunkMap(
		seed,
		terrainParams,
		selectedRegion,
		selectedChunk,
	);

	function applySeedValue(nextSeed: number) {
		setDraftSeed(String(nextSeed));
		setSeed(nextSeed);
		setSeedError(null);
		setSelectedRegion(null);
		setSelectedChunk(null);
	}

	function applySeed() {
		const parsedSeed = Number(draftSeed);
		if (!Number.isSafeInteger(parsedSeed) || parsedSeed < 0) {
			setSeedError("Seed musi być nieujemną liczbą całkowitą.");
			return;
		}

		setTerrainParams(draftParams);
		applySeedValue(parsedSeed);
	}

	function createRandomSeed() {
		const bytes = new Uint32Array(1);
		crypto.getRandomValues(bytes);
		applySeedValue(bytes[0]);
	}

	function selectRegion(region: RegionId) {
		setSelectedRegion(region);
		setSelectedChunk(null);
	}

	const isLoading = loading || regionMap.loading || chunkMap.loading;

	return (
		<main className="map-screen">
			<div className="map-area">
				{selectedRegion && selectedChunk ? (
					<ChunkMapView
						region={selectedRegion}
						chunk={selectedChunk}
						grid={chunkMap.grid}
						onBack={() => setSelectedChunk(null)}
					/>
				) : selectedRegion ? (
					<RegionMapView
						region={selectedRegion}
						grid={regionMap.grid}
						onBack={() => setSelectedRegion(null)}
						onChunkSelect={setSelectedChunk}
					/>
				) : (
					<GlobalMapView grid={grid} onRegionSelect={selectRegion} />
				)}
			</div>

			<aside className="side-panel">
				<SeedControl
					seed={draftSeed}
					onSeedChange={setDraftSeed}
					params={draftParams}
					onParamsChange={setDraftParams}
					onRegenerate={applySeed}
					onRandomSeed={createRandomSeed}
					loading={isLoading}
				/>
				{seedError && <p className="map-error">{seedError}</p>}
				{error && <p className="map-error">{error}</p>}
				{regionMap.error && <p className="map-error">{regionMap.error}</p>}
				{chunkMap.error && <p className="map-error">{chunkMap.error}</p>}
				{isLoading && <p className="map-loading">Generowanie mapy…</p>}
				<BiomeLegend />
			</aside>
		</main>
	);
}

export default App;
