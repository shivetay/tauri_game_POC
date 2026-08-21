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
	return (
		<div className="seed-control-container">
			<h1>World Map Generator</h1>
			<div className="seed-control">
				<label>
					Seed:{" "}
					<input
						value={seed}
						onChange={(e) => onSeedChange(e.target.value)}
						placeholder="eons-world-1"
					/>
				</label>
				<button type="button" onClick={onRegenerate} disabled={loading}>
					{loading ? "Generowanie…" : "Regenerate"}
				</button>
			</div>
		</div>
	);
}
