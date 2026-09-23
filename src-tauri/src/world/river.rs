use noise::{NoiseFn, SuperSimplex};

use crate::world::noise::{clamp01, fbm, smoothstep};
use crate::world::prng::derive_seed;

/// Strength of bank fertility boost applied in the sampler.
pub const RIVER_MOISTURE_BOOST: f32 = 0.48;

/// Shallow carved channel elevation (m) — below sea level → Water biome.
pub const RIVER_CHANNEL_ELEV_M: f32 = -35.0;

pub struct RiverSample {
    /// 0 = dry land, 1 = in the river channel.
    pub channel: f32,
    /// 0 = far, 1 = on the bank / near channel (fertility falloff).
    pub proximity: f32,
}

fn seed_offset(seed: u64, salt: &str) -> f64 {
    (derive_seed(seed, salt) as f64) * 0.00137
}

/// Meandering river field from warped noise. Ocean filtering is done by the sampler.
pub fn river_sample(
    river_noise: &SuperSimplex,
    warp_noise: &SuperSimplex,
    seed: u64,
    world_x: f64,
    world_y: f64,
) -> RiverSample {
    let freq = 0.0095;
    let x = world_x * freq;
    let y = world_y * freq;

    let warp_amp = 3.2;
    let wx = x
        + (fbm(
            warp_noise,
            x * 0.55 + seed_offset(seed, "rwx"),
            y * 0.55,
            3,
            0.5,
            2.0,
        ) - 0.5)
            * warp_amp;
    let wy = y
        + (fbm(
            warp_noise,
            x * 0.55 + 31.0,
            y * 0.55 + 19.0 + seed_offset(seed, "rwy"),
            3,
            0.5,
            2.0,
        ) - 0.5)
            * warp_amp;

    // Sparse but visible network — rivers in a meaningful fraction of land.
    let presence = fbm(
        river_noise,
        wx * 0.32 + seed_offset(seed, "rpres"),
        wy * 0.32,
        3,
        0.55,
        2.0,
    );
    let network = smoothstep(0.40, 0.56, presence);
    if network < 0.02 {
        return RiverSample {
            channel: 0.0,
            proximity: 0.0,
        };
    }

    // Zero-crossing of warped noise = channel centerline.
    let n = river_noise.get([
        wx * 1.05 + seed_offset(seed, "rline"),
        wy * 1.05,
    ]);
    let d = n.abs();

    // Channel / bank widths in noise units (wider = more visible on macro LOD).
    let half_w = 0.062;
    let bank_w = 0.24;
    let channel = network * (1.0 - smoothstep(0.0, half_w, d));
    let proximity = network * (1.0 - smoothstep(0.0, bank_w, d));

    RiverSample {
        channel: clamp01(channel) as f32,
        proximity: clamp01(proximity) as f32,
    }
}

/// True when water should block roads (ocean/lake), false for fordable river channels.
pub fn blocks_road(elevation_m: f32, river_channel: f32) -> bool {
    if elevation_m >= 0.0 {
        return false;
    }
    // Shallow carved river — allow fords.
    if river_channel > 0.45 && elevation_m >= RIVER_CHANNEL_ELEV_M - 1.0 {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::prng::derive_seed;
    use noise::SuperSimplex;

    fn noises(seed: u64) -> (SuperSimplex, SuperSimplex) {
        (
            SuperSimplex::new(derive_seed(seed, "river")),
            SuperSimplex::new(derive_seed(seed, "river-warp")),
        )
    }

    #[test]
    fn river_sample_is_deterministic() {
        let (r, w) = noises(6);
        let a = river_sample(&r, &w, 6, 180.0, 220.0);
        let b = river_sample(&r, &w, 6, 180.0, 220.0);
        assert!((a.channel - b.channel).abs() < 1e-6);
        assert!((a.proximity - b.proximity).abs() < 1e-6);
    }

    #[test]
    fn channel_implies_proximity() {
        let (r, w) = noises(6);
        let mut found = false;
        for yi in 0..256 {
            for xi in 0..256 {
                let x = f64::from(xi) * 2.0;
                let y = f64::from(yi) * 2.0;
                let s = river_sample(&r, &w, 6, x, y);
                if s.channel > 0.55 {
                    assert!(
                        s.proximity >= s.channel - 0.05,
                        "bank proximity should cover the channel"
                    );
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "expected at least one channel sample for seed 6");
    }

    #[test]
    fn shallow_river_does_not_block_roads() {
        assert!(!blocks_road(RIVER_CHANNEL_ELEV_M, 0.9));
        assert!(blocks_road(-500.0, 0.0));
        assert!(!blocks_road(10.0, 0.0));
    }
}
