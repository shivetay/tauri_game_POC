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

/// Scale of a single landmass: below 1 is an island, above 1 is a continent.
fn single_landmass_scale(seed: u64) -> f64 {
    let u = unit_noise(seed, 10, 0);
    if u < 0.5 {
        0.68 + u * 0.36
    } else {
        1.10 + (u - 0.5) * 0.36
    }
}

fn single_landmass(seed: u64, w: f64, h: f64) -> (f64, f64, f64) {
    let scale = single_landmass_scale(seed);
    let shift = if scale > 1.0 { 0.10 } else { 0.06 };
    let lx = w * 0.5 + (unit_noise(seed, 11, 0) - 0.5) * 2.0 * shift * w;
    let ly = h * 0.5 + (unit_noise(seed, 12, 0) - 0.5) * 2.0 * shift * h;
    (scale, lx, ly)
}

fn blob_shape(
    world_x: f64,
    world_y: f64,
    ix: f64,
    iy: f64,
    radius: f64,
    aspect: f64,
    ox: f64,
    oy: f64,
    warp: f64,
) -> f64 {
    let dx = (world_x - ix) / radius + ox * warp;
    let dy = (world_y - iy) / (radius * aspect) + oy * warp;
    radial_falloff(dx, dy)
}

/// Land in SquareBump's inner sea. 0 = open water.
fn square_bump_inner_land(
    seed: u64,
    world_x: f64,
    world_y: f64,
    w: f64,
    h: f64,
    ox: f64,
    oy: f64,
) -> f64 {
    if unit_noise(seed, 20, 0) < 0.42 {
        return 0.0;
    }

    let span = w.min(h);
    if unit_noise(seed, 25, 0) < 0.62 {
        let n = 1 + (derive_seed(seed, "bump-isles") % 3) as usize;
        let mut max_shape = 0.0_f64;
        for i in 0..n {
            let idx = i as u64;
            let ix = (0.40 + unit_noise(seed, 21, idx) * 0.20) * w;
            let iy = (0.40 + unit_noise(seed, 22, idx) * 0.20) * h;
            let radius = span * (0.05 + unit_noise(seed, 23, idx) * 0.06);
            let aspect = 0.55 + unit_noise(seed, 24, idx) * 0.90;
            max_shape = max_shape.max(blob_shape(
                world_x, world_y, ix, iy, radius, aspect, ox, oy, 0.50,
            ));
        }
        max_shape
    } else {
        let ix = (0.42 + unit_noise(seed, 21, 0) * 0.16) * w;
        let iy = (0.42 + unit_noise(seed, 22, 0) * 0.16) * h;
        let radius = span * (0.12 + unit_noise(seed, 23, 0) * 0.07);
        let aspect = 0.55 + unit_noise(seed, 24, 0) * 0.80;
        blob_shape(world_x, world_y, ix, iy, radius, aspect, ox, oy, 0.50)
    }
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
            let (scale, lx, ly) = single_landmass(config.seed, w, h);
            let dx = (world_x - lx) / (w * 0.5 * scale) + ox * 0.45;
            let dy = (world_y - ly) / (h * 0.5 * scale) + oy * 0.45;
            radial_falloff(dx, dy)
        }
        ShapeProfile::Ellipse => {
            let seed_h = hash_seed(config.seed) as f64;
            let aspect = 0.6 + (seed_h % 100.0) / 100.0 * 0.8; // 0.6–1.4
            let (scale, lx, ly) = single_landmass(config.seed, w, h);
            let dx = (world_x - lx) / (w * 0.5 * scale) + ox * 0.45;
            let dy = (world_y - ly) / (h * 0.5 * aspect * scale) + oy * 0.45;
            radial_falloff(dx, dy)
        }
        ShapeProfile::SquareBump => {
            let nx = (2.0 * world_x) / w - 1.0 + ox * 0.30;
            let ny = (2.0 * world_y) / h - 1.0 + oy * 0.30;
            let rim = (1.0 - (1.0 - nx * nx) * (1.0 - ny * ny)).clamp(0.0, 1.0);
            let inner = square_bump_inner_land(config.seed, world_x, world_y, w, h, ox, oy);
            rim.max(inner)
        }
        ShapeProfile::Irregular => {
            let (scale, lx, ly) = single_landmass(config.seed, w, h);
            let dx = (world_x - lx) / (w * 0.5 * scale) + ox * 0.75;
            let dy = (world_y - ly) / (h * 0.5 * scale) + oy * 0.75;
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
        ShapeProfile::Continents => {
            let n = 2 + (derive_seed(config.seed, "cont") % 2) as usize;
            let mut max_shape = 0.0_f64;
            for i in 0..n {
                let idx = i as u64;
                let angle = std::f64::consts::TAU * (idx as f64 / n as f64)
                    + (unit_noise(config.seed, 1, idx) - 0.5) * 0.9;
                let dist = 0.20 + unit_noise(config.seed, 2, idx) * 0.14;
                let ix = cx + dist * w * angle.cos();
                let iy = cy + dist * h * angle.sin();
                let radius = w * (0.28 + unit_noise(config.seed, 3, idx) * 0.12);
                let aspect = 0.50 + unit_noise(config.seed, 4, idx) * 0.80;
                let dx = (world_x - ix) / radius + ox * 0.50;
                let dy = (world_y - iy) / (radius * aspect) + oy * 0.50;
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

    use super::{shape_value, single_landmass, single_landmass_scale};
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
            ShapeProfile::Continents,
        ] {
            let a = sample(profile, 42, 120.0, 200.0);
            let b = sample(profile, 42, 120.0, 200.0);
            assert!((a - b).abs() < 1e-12, "{profile:?}");
        }
    }

    #[test]
    fn radial_coast_is_not_a_perfect_circle() {
        let (scale, lx, ly) = single_landmass(5, 512.0, 512.0);
        let r = 256.0 * scale * 0.70;
        let mut values = Vec::new();
        for k in 0..8 {
            let angle = k as f64 * std::f64::consts::PI / 4.0;
            values.push(sample(
                ShapeProfile::Radial,
                5,
                lx + r * angle.cos(),
                ly + r * angle.sin(),
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

    #[test]
    fn continents_span_multiple_quadrants() {
        for seed in [5u64, 11, 17] {
            let config = WorldConfig {
                seed,
                ..WorldConfig::default()
            };
            let noise = SuperSimplex::new(derive_seed(seed, "coast"));
            let mut quads = [false; 4];
            for y in (0..512).step_by(8) {
                for x in (0..512).step_by(8) {
                    let s = shape_value(
                        ShapeProfile::Continents,
                        x as f64,
                        y as f64,
                        &config,
                        &noise,
                    );
                    if s > 0.35 {
                        let qi = usize::from(x >= 256) + if y >= 256 { 2 } else { 0 };
                        quads[qi] = true;
                    }
                }
            }
            let occupied = quads.iter().filter(|q| **q).count();
            assert!(
                occupied >= 2,
                "seed {seed}: continents should span multiple quadrants ({quads:?})"
            );
        }
    }

    #[test]
    fn continents_cover_more_land_than_archipelago() {
        let seed = 5u64;
        let config = WorldConfig {
            seed,
            ..WorldConfig::default()
        };
        let noise = SuperSimplex::new(derive_seed(seed, "coast"));
        let mut arch = 0;
        let mut cont = 0;
        let mut total = 0;
        for y in (0..512).step_by(8) {
            for x in (0..512).step_by(8) {
                total += 1;
                let wx = x as f64;
                let wy = y as f64;
                if shape_value(ShapeProfile::Archipelago, wx, wy, &config, &noise) > 0.35 {
                    arch += 1;
                }
                if shape_value(ShapeProfile::Continents, wx, wy, &config, &noise) > 0.35 {
                    cont += 1;
                }
            }
        }
        assert!(
            cont > arch * 2,
            "continents should be much larger than islands; cont={cont} arch={arch}"
        );
        assert!(
            cont < total * 3 / 4,
            "continents should still leave ocean; cont={cont} total={total}"
        );
    }

    fn radial_land_count(seed: u64) -> (usize, usize) {
        let config = WorldConfig {
            seed,
            ..WorldConfig::default()
        };
        let noise = SuperSimplex::new(derive_seed(seed, "coast"));
        let mut land = 0;
        let mut total = 0;
        for y in (0..512).step_by(8) {
            for x in (0..512).step_by(8) {
                total += 1;
                if shape_value(
                    ShapeProfile::Radial,
                    x as f64,
                    y as f64,
                    &config,
                    &noise,
                ) > 0.35
                {
                    land += 1;
                }
            }
        }
        (land, total)
    }

    #[test]
    fn radial_can_be_island_or_continent() {
        let island_seed = (0..40)
            .map(|i| i * 6)
            .find(|s| single_landmass_scale(*s) < 0.9)
            .expect("expected an island-scale Radial seed");
        let continent_seed = (0..40)
            .map(|i| i * 6)
            .find(|s| single_landmass_scale(*s) > 1.05)
            .expect("expected a continent-scale Radial seed");
        let (island, total) = radial_land_count(island_seed);
        let (continent, _) = radial_land_count(continent_seed);
        assert!(
            continent > island * 3 / 2,
            "continent Radial should cover more land; island_seed={island_seed} island={island} continent_seed={continent_seed} continent={continent}"
        );
        assert!(
            continent < total * 9 / 10,
            "single continent should still leave ocean; continent={continent} total={total}"
        );
    }

    fn square_bump_inner_sea_land(seed: u64) -> usize {
        let config = WorldConfig {
            seed,
            ..WorldConfig::default()
        };
        let noise = SuperSimplex::new(derive_seed(seed, "coast"));
        let mut land = 0;
        for y in (192..320).step_by(8) {
            for x in (192..320).step_by(8) {
                if shape_value(
                    ShapeProfile::SquareBump,
                    x as f64,
                    y as f64,
                    &config,
                    &noise,
                ) > 0.35
                {
                    land += 1;
                }
            }
        }
        land
    }

    #[test]
    fn square_bump_keeps_rim_land() {
        for seed in [2u64, 8, 14, 20] {
            let edge = sample(ShapeProfile::SquareBump, seed, 8.0, 256.0);
            let corner = sample(ShapeProfile::SquareBump, seed, 8.0, 8.0);
            assert!(edge > 0.5, "seed {seed}: rim should stay land; edge={edge}");
            assert!(
                corner > 0.5,
                "seed {seed}: corners should stay land; corner={corner}"
            );
        }
    }

    #[test]
    fn square_bump_inner_sea_can_be_open_or_have_land() {
        let seeds: Vec<u64> = (0..40).map(|i| i * 6 + 2).collect();
        let with_land = seeds
            .iter()
            .filter(|s| square_bump_inner_sea_land(**s) > 0)
            .count();
        let open = seeds.len() - with_land;
        assert!(
            open >= 1,
            "expected some SquareBump seeds with open inner sea"
        );
        assert!(
            with_land >= 1,
            "expected some SquareBump seeds with land in the inner sea"
        );
    }
}
