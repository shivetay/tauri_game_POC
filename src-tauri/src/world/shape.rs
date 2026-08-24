use noise::SuperSimplex;

use crate::world::config::{ShapeProfile, WorldConfig};
use crate::world::noise::{fbm, lerp};
use crate::world::prng::{derive_seed, hash_seed, unit_noise};

fn coast_warp(coast_noise: &SuperSimplex, world_x: f64, world_y: f64) -> (f64, f64) {
    let ox = fbm(coast_noise, world_x * 0.012, world_y * 0.012, 4, 0.5, 2.0) - 0.5;
    let oy = fbm(
        coast_noise,
        world_x * 0.012 + 51.0,
        world_y * 0.012 + 17.0,
        4,
        0.5,
        2.0,
    ) - 0.5;
    (ox, oy)
}

fn radial_falloff(dx: f64, dy: f64) -> f64 {
    (1.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0)
}

/// Wartość kształtu: 1 = środek lądu, 0 = krawędź / ocean
pub fn shape_value(
    profile: ShapeProfile,
    world_x: f64,
    world_y: f64,
    config: &WorldConfig,
    coast_noise: &SuperSimplex,
) -> f64 {
    let w = config.world_width as f64;
    let h = config.world_height as f64;
    let cx = w * 0.5;
    let cy = h * 0.5;
    let (ox, oy) = coast_warp(coast_noise, world_x, world_y);

    match profile {
        ShapeProfile::Radial => {
            let dx = (world_x - cx) / (w * 0.5) + ox * 0.45;
            let dy = (world_y - cy) / (h * 0.5) + oy * 0.45;
            radial_falloff(dx, dy)
        }
        ShapeProfile::Ellipse => {
            let seed_h = hash_seed(config.seed) as f64;
            let aspect = 0.6 + (seed_h % 100.0) / 100.0 * 0.8; // 0.6–1.4
            let dx = (world_x - cx) / (w * 0.5) + ox * 0.45;
            let dy = (world_y - cy) / (h * 0.5 * aspect) + oy * 0.45;
            radial_falloff(dx, dy)
        }
        ShapeProfile::SquareBump => {
            let nx = (2.0 * world_x) / w - 1.0 + ox * 0.30;
            let ny = (2.0 * world_y) / h - 1.0 + oy * 0.30;
            (1.0 - (1.0 - nx * nx) * (1.0 - ny * ny)).clamp(0.0, 1.0)
        }
        ShapeProfile::Irregular => {
            let dx = (world_x - cx) / (w * 0.5) + ox * 0.75;
            let dy = (world_y - cy) / (h * 0.5) + oy * 0.75;
            radial_falloff(dx, dy)
        }
        ShapeProfile::Archipelago => {
            let n = 3 + (derive_seed(config.seed, "arch") % 4) as usize;
            let margin = 0.16;
            let mut max_shape = 0.0_f64;
            for i in 0..n {
                let idx = i as u64;
                let ix = (margin + unit_noise(config.seed, 1, idx) * (1.0 - 2.0 * margin)) * w;
                let iy = (margin + unit_noise(config.seed, 2, idx) * (1.0 - 2.0 * margin)) * h;
                let radius =
                    config.region_size as f64 * (0.40 + unit_noise(config.seed, 3, idx) * 0.90);
                let aspect = 0.55 + unit_noise(config.seed, 4, idx) * 0.90;
                let dx = (world_x - ix) / radius + ox * 0.55;
                let dy = (world_y - iy) / (radius * aspect) + oy * 0.55;
                max_shape = max_shape.max(radial_falloff(dx, dy));
            }
            max_shape
        }
    }
}

/// Mieszanka surowej elewacji z kształtem wyspy
pub fn apply_shape(raw_elevation: f64, shape: f64, island_mix: f64) -> f64 {
    lerp(raw_elevation, shape, island_mix as f64)
}

#[cfg(test)]
mod tests {
    use noise::SuperSimplex;

    use super::shape_value;
    use crate::world::config::{ShapeProfile, WorldConfig};
    use crate::world::prng::derive_seed;

    fn sample(profile: ShapeProfile, seed: u64, x: f64, y: f64) -> f64 {
        let config = WorldConfig {
            seed,
            ..WorldConfig::default()
        };
        let noise = SuperSimplex::new(derive_seed(seed, "coast"));
        shape_value(profile, x, y, &config, &noise)
    }

    #[test]
    fn same_seed_same_shape() {
        for profile in [
            ShapeProfile::Radial,
            ShapeProfile::Ellipse,
            ShapeProfile::SquareBump,
            ShapeProfile::Irregular,
            ShapeProfile::Archipelago,
        ] {
            let a = sample(profile, 42, 120.0, 200.0);
            let b = sample(profile, 42, 120.0, 200.0);
            assert!((a - b).abs() < 1e-12, "{profile:?}");
        }
    }

    #[test]
    fn radial_coast_is_not_a_perfect_circle() {
        let r = 180.0;
        let cx = 256.0;
        let cy = 256.0;
        let mut values = Vec::new();
        for k in 0..8 {
            let angle = k as f64 * std::f64::consts::PI / 4.0;
            values.push(sample(
                ShapeProfile::Radial,
                5,
                cx + r * angle.cos(),
                cy + r * angle.sin(),
            ));
        }
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            max - min > 0.05,
            "coastline should vary around the circle; values={values:?}"
        );
    }

    #[test]
    fn archipelago_islands_are_not_collinear() {
        for seed in [9u64, 14, 19] {
            let config = WorldConfig {
                seed,
                ..WorldConfig::default()
            };
            let noise = SuperSimplex::new(derive_seed(seed, "coast"));
            let mut ys = Vec::new();
            for y in (0..512).step_by(8) {
                for x in (0..512).step_by(8) {
                    let s = shape_value(
                        ShapeProfile::Archipelago,
                        x as f64,
                        y as f64,
                        &config,
                        &noise,
                    );
                    if s > 0.35 {
                        ys.push(y as f64);
                    }
                }
            }
            assert!(!ys.is_empty(), "seed {seed} produced no land");
            let mean = ys.iter().sum::<f64>() / ys.len() as f64;
            let std = (ys.iter().map(|y| (y - mean).powi(2)).sum::<f64>() / ys.len() as f64).sqrt();
            assert!(
                std > 60.0,
                "seed {seed}: islands should not sit on one latitude (std={std})"
            );
        }
    }
}
