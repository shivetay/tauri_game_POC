import { useState } from "react";
import type { TerrainParams } from "../../types/terrainParams";

interface SeedControlProps {
	seed: string;
	onSeedChange: (seed: string) => void;
	params: TerrainParams;
	onParamsChange: (params: TerrainParams) => void;
	onRegenerate: () => void;
	onRandomSeed: () => void;
	loading: boolean;
}

const PARAM_SLIDERS: {
	key: keyof TerrainParams;
	label: string;
	hint: string;
}[] = [
	{
		key: "orogenyStrength",
		label: "Ukształtowanie",
		hint: "0 = płasko, 1 = pełne pasma (góry i depresje z seeda)",
	},
	{
		key: "moistureStrength",
		label: "Wilgotność",
		hint: "0 = jednolita, 1 = pełny zasięg biomów",
	},
	{
		key: "detailStrength",
		label: "Detal micro",
		hint: "0 = gładko, 1 = pełny detal w zbliżeniu",
	},
	{
		key: "terrainRoughness",
		label: "Falistość terenu",
		hint: "0 = równiny, 1 = pełne wzniesienia bazowe",
	},
	{
		key: "landSize",
		label: "Powierzchnia lądu",
		hint: "0 = mniej lądu, 1 = więcej lądu",
	},
	{
		key: "coastDistortion",
		label: "Nieregularność brzegu",
		hint: "0 = gładkie wybrzeże, 1 = pełne zatoki i półwyspy",
	},
];

export function SeedControl({
	seed,
	onSeedChange,
	params,
	onParamsChange,
	onRegenerate,
	onRandomSeed,
	loading,
}: SeedControlProps) {
	const [showSeedInfo, setShowSeedInfo] = useState(false);

	function setParam(key: keyof TerrainParams, value: number) {
		onParamsChange({ ...params, [key]: value });
	}

	return (
		<div className="seed-control-container">
			<h1>World Map Generator</h1>
			<div className="seed-control">
				<label>
					Seed:{" "}
					<input
						type="number"
						min="0"
						step="1"
						value={seed}
						onChange={(e) => onSeedChange(e.target.value)}
						placeholder="6"
					/>
				</label>
				{PARAM_SLIDERS.map(({ key, label, hint }) => (
					<label key={key} className="param-slider" title={hint}>
						{label}:{" "}
						<input
							type="range"
							min="0"
							max="1"
							step="0.1"
							value={params[key]}
							onChange={(e) => setParam(key, Number(e.target.value))}
						/>
						<span className="param-value">{params[key].toFixed(1)}</span>
					</label>
				))}
				<div className="seed-actions">
					<button type="button" onClick={onRandomSeed} disabled={loading}>
						Losuj seed
					</button>
					<button type="button" onClick={onRegenerate} disabled={loading}>
						{loading ? "Generowanie…" : "Regenerate"}
					</button>
					<button
						type="button"
						className="seed-info-toggle"
						onClick={() => setShowSeedInfo((open) => !open)}
					>
						{showSeedInfo ? "Ukryj" : "Pokaż"} opis
					</button>
				</div>
				{showSeedInfo && (
					<div className="seed-info">
						<p>
							Seed to nieujemna liczba całkowita. Z niej generator buduje cały
							świat:
						</p>
						<ul>
							<li>
								<strong>profil kształtu</strong> — <code>seed % 6</code> (0
								Radial, 1 Ellipse, 2 SquareBump, 3 Irregular, 4 Archipelago, 5
								Continents)
							</li>
							<li>
								<strong>pasma górskie i depresje</strong> — losowe z seeda;
								suwak ukształtowania tylko skaluje siłę efektu
							</li>
							<li>
								<strong>wilgotność</strong> — pochodna <code>seed + moisture</code>
							</li>
							<li>
								<strong>detal micro</strong> — pochodna <code>seed + detail</code>
							</li>
							<li>
								<strong>wybrzeże</strong> — pochodna <code>seed + coast</code>
							</li>
						</ul>
						<p>
							Suwaki skalują intensywność (0 = wyłączone, 1 = domyślnie). Ten
							sam seed i te same suwaki zawsze dają ten sam świat.
						</p>
					</div>
				)}
			</div>
		</div>
	);
}
