use noise::SuperSimplex;

use crate::world::noise::{fbm, smoothstep};
use crate::world::prng::derive_seed;

const LAND_UPLIFT: f64 = 0.42;
const OCEAN_UPLIFT: f64 = 0.07;
const DEPRESSION_SCALE: f64 = 0.30;

/// Weak relief below this is discarded — avoids thin smear-like belts.
const MIN_UPLIFT_BELT: f64 = 0.18;
const MIN_DEPRESSION_BELT: f64 = 0.16;

fn seed_offset(seed: u64, salt: &str) -> f64 {
    (derive_seed(seed, salt) as f64) * 0.00137
}

/// Zero out weak fringe; remap the rest to [0, 1].
fn belt_strength(raw: f64, min: f64) -> f64 {
    if raw <= min {
        0.0
    } else {
        (raw - min) / (1.0 - min)
    }
}

/// Broad uplift/depression zones — no ridged line geometry.
fn relief_at(
    ridge_noise: &SuperSimplex,
    warp_noise: &SuperSimplex,
    seed: u64,
    world_x: f64,
    world_y: f64,
) -> (f64, f64) {
    let freq = 0.0024;
    let x = world_x * freq;
    let y = world_y * freq;

    let warp_x = (fbm(warp_noise, x + seed_offset(seed, "owx"), y, 2, 0.5, 2.0) - 0.5) * 3.0;
    let warp_y = (fbm(
        warp_noise,
        x + 41.0,
        y + 17.0 + seed_offset(seed, "owy"),
        2,
        0.5,
        2.0,
    ) - 0.5)
        * 3.0;
    let wx = x + warp_x;
    let wy = y + warp_y;

    let zone = fbm(
        ridge_noise,
        wx * 0.5 + seed_offset(seed, "zone"),
        wy * 0.5,
        3,
        0.5,
        2.0,
    );

    let uplift_raw = smoothstep(0.54, 0.74, zone);
    let depression_raw = smoothstep(0.54, 0.74, 1.0 - zone);

    let interior = fbm(
        ridge_noise,
        wx * 0.85 + seed_offset(seed, "interior"),
        wy * 0.85,
        2,
        0.5,
        2.0,
    );
    let uplift = belt_strength(uplift_raw, MIN_UPLIFT_BELT) * (0.72 + interior * 0.38);

    let basin = fbm(
        warp_noise,
        wx * 1.0 + seed_offset(seed, "basin") + 90.0,
        wy * 1.0,
        2,
        0.5,
        2.0,
    );
    let basin_core = belt_strength(smoothstep(0.55, 0.80, basin), MIN_DEPRESSION_BELT);
    let depression = belt_strength(depression_raw, MIN_DEPRESSION_BELT) * (0.7 + basin_core * 0.3);

    (uplift, depression)
}

/// Relief delta from seed-driven zones; scale slider only controls intensity (0 = flat).
pub fn orogeny_delta(
    ridge_noise: &SuperSimplex,
    warp_noise: &SuperSimplex,
    seed: u64,
    world_x: f64,
    world_y: f64,
    _world_width: f64,
    _world_height: f64,
    land_factor: f64,
    scale: f64,
) -> f64 {
    if scale.abs() < f64::EPSILON {
        return 0.0;
    }

    let (uplift, depression) =
        relief_at(ridge_noise, warp_noise, seed, world_x, world_y);

    let land = smoothstep(0.0, 1.0, land_factor);
    let uplift_delta = uplift * land * LAND_UPLIFT;
    let depression_delta =
        depression * (land * DEPRESSION_SCALE + (1.0 - land) * OCEAN_UPLIFT * 0.5);
    (uplift_delta - depression_delta) * scale
}

#[cfg(test)]
mod tests {
    use noise::SuperSimplex;

    use super::{belt_strength, orogeny_delta, MIN_DEPRESSION_BELT, MIN_UPLIFT_BELT};
    use crate::world::prng::derive_seed;

    fn noises(seed: u64) -> (SuperSimplex, SuperSimplex) {
        (
            SuperSimplex::new(derive_seed(seed, "ridge")),
            SuperSimplex::new(derive_seed(seed, "ridge-warp")),
        )
    }

    #[test]
    fn belt_strength_zeros_weak_values() {
        assert_eq!(belt_strength(0.1, MIN_UPLIFT_BELT), 0.0);
        assert!(belt_strength(0.5, MIN_UPLIFT_BELT) > 0.2);
    }

    #[test]
    fn same_seed_same_orogeny() {
        let (ridge, warp) = noises(42);
        let a = orogeny_delta(&ridge, &warp, 42, 120.0, 200.0, 512.0, 512.0, 1.0, 1.0);
        let b = orogeny_delta(&ridge, &warp, 42, 120.0, 200.0, 512.0, 512.0, 1.0, 1.0);
        assert!((a - b).abs() < 1e-12);
    }

    #[test]
    fn seed_produces_both_uplift_and_depression() {
        let (ridge, warp) = noises(6);
        let mut uplift = 0;
        let mut depression = 0;
        for y in (0..512).step_by(8) {
            for x in (0..512).step_by(8) {
                let d = orogeny_delta(
                    &ridge,
                    &warp,
                    6,
                    x as f64,
                    y as f64,
                    512.0,
                    512.0,
                    1.0,
                    1.0,
                );
                if d > 0.04 {
                    uplift += 1;
                }
                if d < -0.02 {
                    depression += 1;
                }
            }
        }
        assert!(uplift >= 12, "expected uplift zones; uplift={uplift}");
        assert!(
            depression >= 10,
            "expected seed-driven depressions; depression={depression}"
        );
    }

    #[test]
    fn weak_orogeny_is_suppressed() {
        let (ridge, warp) = noises(6);
        let mut weak = 0;
        let mut strong = 0;
        for y in (0..512).step_by(4) {
            for x in (0..512).step_by(4) {
                let d = orogeny_delta(
                    &ridge,
                    &warp,
                    6,
                    x as f64,
                    y as f64,
                    512.0,
                    512.0,
                    1.0,
                    1.0,
                )
                .abs();
                if d > 0.0 && d < 0.012 {
                    weak += 1;
                }
                if d >= 0.04 {
                    strong += 1;
                }
            }
        }
        assert!(
            weak < strong / 4,
            "too many weak smear values; weak={weak} strong={strong}"
        );
    }

    #[test]
    fn relief_is_not_thin_directional_streaks() {
        use crate::world::config::{LodLevel, WorldConfig};
        use crate::world::elevation::HIGH_M;
        use crate::world::sampler::TerrainSampler;

        let sampler = TerrainSampler::new(WorldConfig::default());
        let threshold = HIGH_M;
        let mut max_row_run = 0;
        let mut max_col_run = 0;

        for y in 0..512 {
            let mut run = 0;
            for x in 0..512 {
                let e = sampler.elevation_at(x as f64, y as f64, LodLevel::Macro);
                if e > threshold {
                    run += 1;
                    max_row_run = max_row_run.max(run);
                } else {
                    run = 0;
                }
            }
        }
        for x in 0..512 {
            let mut run = 0;
            for y in 0..512 {
                let e = sampler.elevation_at(x as f64, y as f64, LodLevel::Macro);
                if e > threshold {
                    run += 1;
                    max_col_run = max_col_run.max(run);
                } else {
                    run = 0;
                }
            }
        }

        assert!(
            max_row_run < 55 && max_col_run < 55,
            "thin streaks detected; max_row_run={max_row_run} max_col_run={max_col_run}"
        );
    }

    #[test]
    fn zero_orogeny_has_no_mountains() {
        use crate::world::biome::biome;
        use crate::world::config::{LodLevel, WorldConfig};
        use crate::world::sampler::TerrainSampler;
        use crate::world::types::TileType;

        let mut config = WorldConfig::default();
        config.orogeny_strength = 0.0;
        let sampler = TerrainSampler::new(config);
        let mut mountains = 0;
        for y in (0..512).step_by(4) {
            for x in (0..512).step_by(4) {
                let e = sampler.elevation_at(x as f64, y as f64, LodLevel::Macro);
                let m = sampler.moisture_at(x as f64, y as f64);
                let b = biome(e, m);
                if matches!(b, TileType::Mountain | TileType::Snow) {
                    mountains += 1;
                }
            }
        }
        assert_eq!(mountains, 0, "orogeny_strength=0 should produce no mountains");
    }

    #[test]
    fn mountains_are_spread_not_a_single_peak() {
        use crate::world::biome::biome;
        use crate::world::config::{LodLevel, WorldConfig};
        use crate::world::sampler::TerrainSampler;
        use crate::world::types::TileType;

        for seed in [6u64, 12, 18, 24, 30] {
            let sampler = TerrainSampler::new(WorldConfig::default().with_seed(seed));
            let mut high_tiles = Vec::new();
            for y in (0..512).step_by(4) {
                for x in (0..512).step_by(4) {
                    let e = sampler.elevation_at(x as f64, y as f64, LodLevel::Macro);
                    let m = sampler.moisture_at(x as f64, y as f64);
                    let b = biome(e, m);
                    if matches!(b, TileType::Mountain | TileType::Snow) {
                        high_tiles.push((x, y));
                    }
                }
            }
            if high_tiles.len() < 8 {
                continue;
            }
            let mut xs: Vec<i32> = high_tiles.iter().map(|(x, _)| *x).collect();
            xs.sort_unstable();
            let spread = xs.last().copied().unwrap_or(0) - xs.first().copied().unwrap_or(0);
            assert!(spread > 40, "seed {seed}: spread={spread}");
            return;
        }
        panic!("no seed produced spread mountain belts");
    }

    #[test]
    fn mountain_heights_vary_and_cap_at_ten_km() {
        use crate::world::biome::biome;
        use crate::world::config::{LodLevel, WorldConfig};
        use crate::world::elevation::MAX_ELEVATION_M;
        use crate::world::sampler::TerrainSampler;
        use crate::world::types::TileType;

        let sampler = TerrainSampler::new(WorldConfig::default());
        let mut min_m = f32::MAX;
        let mut max_m = f32::MIN;
        let mut count = 0;
        for y in (0..512).step_by(4) {
            for x in (0..512).step_by(4) {
                let e = sampler.elevation_at(x as f64, y as f64, LodLevel::Macro);
                let m = sampler.moisture_at(x as f64, y as f64);
                let b = biome(e, m);
                if matches!(b, TileType::Mountain | TileType::Snow) {
                    min_m = min_m.min(e);
                    max_m = max_m.max(e);
                    count += 1;
                }
            }
        }
        if count >= 2 {
            assert!(max_m <= MAX_ELEVATION_M + 1.0, "peak above cap; max_m={max_m}");
            assert!(
                max_m - min_m > 300.0,
                "mountains should vary in height; min={min_m} max={max_m}"
            );
        }
    }
}
