use noise::{NoiseFn, SuperSimplex};

use crate::world::config::{ShapeProfile, WorldConfig};
use crate::world::noise::{fbm, lerp};
use crate::world::prng::{hash_seed, hash_seed_u64};

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

    match profile {
        ShapeProfile::Radial => {
            let dx = (world_x - cx) / (w * 0.5);
            let dy = (world_y - cy) / (h * 0.5);
            let d = (dx * dx + dy * dy).sqrt();
            (1.0 - d).clamp(0.0, 1.0)
        }
        ShapeProfile::Ellipse => {
            let seed_h = hash_seed(&config.seed) as f64;
            let aspect = 0.6 + (seed_h % 100.0) / 100.0 * 0.8; // 0.6–1.4
            let dx = (world_x - cx) / (w * 0.5);
            let dy = (world_y - cy) / (h * 0.5 * aspect);
            let d = (dx * dx + dy * dy).sqrt();
            (1.0 - d).clamp(0.0, 1.0)
        }
        ShapeProfile::SquareBump => {
            let nx = (2.0 * world_x) / w - 1.0;
            let ny = (2.0 * world_y) / h - 1.0;
            1.0 - (1.0 - nx * nx) * (1.0 - ny * ny)
        }
        ShapeProfile::Irregular => {
            let dx = (world_x - cx) / (w * 0.5);
            let dy = (world_y - cy) / (h * 0.5);
            let base = (1.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            let coast = fbm(coast_noise, world_x * 0.02, world_y * 0.02, 3, 0.5, 2.0);
            (base + (coast - 0.5) * 0.35).clamp(0.0, 1.0)
        }
        ShapeProfile::Archipelago => {
            let n = 3 + (hash_seed(&format!("{}:arch", config.seed)) % 4) as usize;
            let mut max_shape = 0.0_f64;
            for i in 0..n {
                let h64 = hash_seed_u64(&format!("{}:island:{i}", config.seed));
                let ix = (h64 % config.world_width as u64) as f64;
                let iy = ((h64 >> 32) % config.world_height as u64) as f64;
                let dx = (world_x - ix) / (config.region_size as f64 * 0.8);
                let dy = (world_y - iy) / (config.region_size as f64 * 0.8);
                let d = (dx * dx + dy * dy).sqrt();
                max_shape = max_shape.max((1.0 - d).clamp(0.0, 1.0));
            }
            max_shape
        }
    }
}

/// Mieszanka surowej elewacji z kształtem wyspy
pub fn apply_shape(raw_elevation: f64, shape: f64, island_mix: f64) -> f64 {
    lerp(raw_elevation, shape, island_mix as f64)
}