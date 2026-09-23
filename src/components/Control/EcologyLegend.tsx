import { FAUNA_GROUP_INFO, VEGETATION_RGB } from "../../map/ecology";

interface EcologyLegendProps {
	mode: "region" | "chunk" | "hidden";
}

export function EcologyLegend({ mode }: EcologyLegendProps) {
	if (mode === "hidden") return null;

	return (
		<div className="biome-legend">
			<h2>{mode === "region" ? "Siedliska (region)" : "Życie (obszar)"}</h2>
			<ul>
				<li>
					<span
						className="biome-swatch"
						style={{
							backgroundColor: `rgb(${VEGETATION_RGB.join(", ")})`,
						}}
					/>
					{mode === "region" ? "Potencjał biomu" : "Roślinność"}
				</li>
				{(Object.keys(FAUNA_GROUP_INFO) as Array<keyof typeof FAUNA_GROUP_INFO>).map(
					(key) => (
						<li key={key}>
							<span
								className="biome-swatch"
								style={{
									backgroundColor: `rgb(${FAUNA_GROUP_INFO[key].rgb.join(", ")})`,
								}}
							/>
							{FAUNA_GROUP_INFO[key].label}
						</li>
					),
				)}
			</ul>
			{mode === "chunk" && (
				<p className="map-hint">Kliknij obszar, by zobaczyć biom i gatunki.</p>
			)}
			{mode === "region" && (
				<p className="map-hint">
					Kliknij obszar, by powiększyć · kliknij miasto po szczegóły.
				</p>
			)}
		</div>
	);
}
