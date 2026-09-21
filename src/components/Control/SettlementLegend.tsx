import type { Settlement } from "../../api/world";
import {
	SETTLEMENT_INFO,
	SETTLEMENT_KIND_ORDER,
} from "../../map/settlements";

interface SettlementLegendProps {
	settlements: Settlement[];
}

function formatRange(min: number, max: number): string {
	return `${min.toLocaleString("pl-PL")}–${max.toLocaleString("pl-PL")}`;
}

export function SettlementLegend({ settlements }: SettlementLegendProps) {
	const counts = {
		City: 0,
		Town: 0,
		Village: 0,
		Hamlet: 0,
	};
	for (const settlement of settlements) {
		counts[settlement.kind] += 1;
	}

	return (
		<div className="biome-legend">
			<h2>Osady (mieszkańcy)</h2>
			<ul>
				{SETTLEMENT_KIND_ORDER.map((kind) => {
					const info = SETTLEMENT_INFO[kind];
					return (
						<li key={kind}>
							<span
								className="biome-swatch settlement-swatch"
								style={{ backgroundColor: info.fill }}
							/>
							<span>
								{info.label} · {formatRange(info.popMin, info.popMax)} ·{" "}
								{counts[kind]}
							</span>
						</li>
					);
				})}
			</ul>
		</div>
	);
}
