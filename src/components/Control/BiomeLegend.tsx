import { BIOME_LEGEND_ORDER, BIOMES } from "../../map/colors";

export function BiomeLegend() {
	return (
		<div className="biome-legend">
			<h2>Biomy</h2>
			<ul>
				{BIOME_LEGEND_ORDER.map((biome) => (
					<li key={biome}>
						<span
							className="biome-swatch"
							style={{
								backgroundColor: `rgb(${BIOMES[biome].rgb.join(", ")})`,
							}}
						/>
						{BIOMES[biome].label}
					</li>
				))}
			</ul>
		</div>
	);
}
