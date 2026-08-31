/// Sea level in normalized elevation space (must match generation).
pub const SEA_LEVEL_NORM: f32 = 0.35;

/// Maximum land elevation in meters.
pub const MAX_ELEVATION_M: f32 = 10_000.0;

/// Maximum ocean depth below sea level in meters.
pub const MAX_DEPTH_M: f32 = 4_000.0;

/// Convert internal normalized elevation [0, 1] to meters (sea level = 0 m).
pub const fn norm_to_meters(norm: f32) -> f32 {
    if norm <= SEA_LEVEL_NORM {
        let t = norm / SEA_LEVEL_NORM;
        t * MAX_DEPTH_M - MAX_DEPTH_M
    } else {
        let t = (norm - SEA_LEVEL_NORM) / (1.0 - SEA_LEVEL_NORM);
        t * MAX_ELEVATION_M
    }
}

/// Biome thresholds in meters (equivalent to legacy normalized cutoffs).
pub const DEEP_WATER_M: f32 = norm_to_meters(0.28);
pub const WATER_M: f32 = 0.0;
pub const COAST_M: f32 = norm_to_meters(0.42);
pub const UPLAND_M: f32 = norm_to_meters(0.55);
pub const HIGH_M: f32 = norm_to_meters(0.70);
pub const SNOW_LINE_M: f32 = norm_to_meters(0.85);

/// Keep zoomed elevation on the same side of sea level as macro.
pub fn preserve_macro_sea_class(macro_norm: f32, norm: f32) -> f32 {
    if macro_norm < SEA_LEVEL_NORM {
        norm.min(SEA_LEVEL_NORM - 0.02)
    } else {
        norm.max(SEA_LEVEL_NORM + 0.02)
    }
}

pub fn is_water_biome(elevation_m: f32) -> bool {
    elevation_m < WATER_M
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sea_level_is_zero_meters() {
        assert!((norm_to_meters(SEA_LEVEL_NORM)).abs() < 1e-4);
    }

    #[test]
    fn max_peak_is_ten_km() {
        assert!((norm_to_meters(1.0) - MAX_ELEVATION_M).abs() < 1e-3);
    }
}
