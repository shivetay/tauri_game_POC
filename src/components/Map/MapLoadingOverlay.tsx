interface MapLoadingOverlayProps {
	label?: string;
}

export function MapLoadingOverlay({
	label = "Generowanie mapy…",
}: MapLoadingOverlayProps) {
	return (
		<div className="map-loading-overlay" role="status" aria-live="polite">
			<div className="map-loading-spinner" aria-hidden="true" />
			<p>{label}</p>
		</div>
	);
}
