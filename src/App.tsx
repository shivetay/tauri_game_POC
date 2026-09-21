import { useState } from "react";
import type { ChunkId, RegionId } from "./api/world";
import {
	DEFAULT_TERRAIN_PARAMS,
	type TerrainParams,
} from "./api/world";
import { SeedControl } from "./components/Control/SeedControl";
import { BiomeLegend } from "./components/Control/BiomeLegend";
import { SettlementLegend } from "./components/Control/SettlementLegend";
import { ChunkMapView } from "./components/Map/ChunkMapView";
import { GlobalMapView } from "./components/Map/GlobalMapView";
import { MapLoadingOverlay } from "./components/Map/MapLoadingOverlay";
import { RegionMapView } from "./components/Map/RegionMapView";
import { useChunkMap } from "./hooks/useChunkMap";
import { useRegionMap } from "./hooks/useRegionMap";
import { useSettlements } from "./hooks/useSettlements";
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
	const settlementMap = useSettlements(seed, terrainParams);

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

	const isLoading =
		loading || regionMap.loading || chunkMap.loading || settlementMap.loading;
	const mapLoading =
		selectedRegion && selectedChunk
			? chunkMap.loading
			: selectedRegion
				? regionMap.loading || settlementMap.loading
				: loading || settlementMap.loading;
	const mapLoadingLabel =
		selectedRegion && selectedChunk
			? "Generowanie obszaru…"
			: selectedRegion
				? "Generowanie regionu…"
				: "Generowanie mapy świata…";

	return (
		<main className="map-screen">
			<div className="map-area">
				{mapLoading && <MapLoadingOverlay label={mapLoadingLabel} />}
				{selectedRegion && selectedChunk ? (
					<ChunkMapView
						region={selectedRegion}
						chunk={selectedChunk}
						grid={chunkMap.grid}
						onBack={() => setSelectedChunk(null)}
						settlements={settlementMap.settlements}
					/>
				) : selectedRegion ? (
					<RegionMapView
						region={selectedRegion}
						grid={regionMap.grid}
						onBack={() => setSelectedRegion(null)}
						onChunkSelect={setSelectedChunk}
						settlements={settlementMap.settlements}
					/>
				) : (
					<GlobalMapView
						grid={grid}
						onRegionSelect={selectRegion}
						settlements={settlementMap.settlements}
					/>
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
				{settlementMap.error && (
					<p className="map-error">{settlementMap.error}</p>
				)}
				{isLoading && <p className="map-loading">Generowanie mapy…</p>}
				<SettlementLegend settlements={settlementMap.settlements} />
				<BiomeLegend />
			</aside>
		</main>
	);
}

export default App;
