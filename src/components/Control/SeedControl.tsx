import { useState } from "react";

interface SeedControlProps {
	seed: string;
	onSeedChange: (seed: string) => void;
	onRegenerate: () => void;
	loading: boolean;
}

export function SeedControl({
	seed,
	onSeedChange,
	onRegenerate,
	loading,
}: SeedControlProps) {
	const [showSeedInfo, setShowSeedInfo] = useState(false);

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
						placeholder="5"
					/>
				</label>
				<div className="seed-actions">
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
								<strong>profil kształtu</strong> — <code>seed % 5</code> (0
								Radial, 1 Ellipse, 2 SquareBump, 3 Irregular, 4 Archipelago)
							</li>
							<li>
								<strong>wysokość terenu</strong> — pochodna seeda (elevation)
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
							<li>
								<strong>archipelag</strong> — liczba i pozycje wysp z pochodnych{" "}
								<code>arch</code> / <code>island:N</code>
							</li>
						</ul>
						<p>Ten sam seed zawsze daje ten sam świat.</p>
					</div>
				)}
			</div>
		</div>
	);
}
