use rayon::prelude::*;

use crate::world::config::{LodLevel, WorldConfig};
use crate::world::sampler::TerrainSampler;
use crate::world::types::{RegionBounds, RegionId, TerrainCell, TerrainGrid};

pub fn generate_global_grid(config: WorldConfig) -> TerrainGrid {
    let sampler = TerrainSampler::new(config.clone());
    let width = config.macro_resolution;
    let height = config.macro_resolution;
    let world_w = config.world_width as f64;
    let world_h = config.world_height as f64;

    let rows: Vec<Vec<TerrainCell>> = (0..height)
        .into_par_iter()
        .map(|y| {
            (0..width)
                .map(|x| {
                    let world_x = (x as f64 + 0.5) / width as f64 * world_w;
                    let world_y = (y as f64 + 0.5) / height as f64 * world_h;
                    sampler.cell_at(world_x, world_y, LodLevel::Macro)
                })
                .collect()
        })
        .collect();

    let cells: Vec<TerrainCell> = rows.into_iter().flatten().collect();

    TerrainGrid {
        width,
        height,
        cells,
    }
}

pub fn generate_region_grid(config: WorldConfig, region: RegionId) -> TerrainGrid {
    let sampler = TerrainSampler::new(config.clone());
    let bounds = RegionBounds::from_region(region, config.region_size);
    let width = config.micro_resolution;
    let height = config.micro_resolution;
    let span_x = bounds.world_x1 - bounds.world_x0;
    let span_y = bounds.world_y1 - bounds.world_y0;

    let rows: Vec<Vec<TerrainCell>> = (0..height)
        .into_par_iter()
        .map(|y| {
            (0..width)
                .map(|x| {
                    let world_x = f64::from(bounds.world_x0)
                        + (x as f64 + 0.5) / width as f64 * f64::from(span_x);
                    let world_y = f64::from(bounds.world_y0)
                        + (y as f64 + 0.5) / height as f64 * f64::from(span_y);
                    sampler.cell_at_region(world_x, world_y, bounds)
                })
                .collect()
        })
        .collect();

    let cells: Vec<TerrainCell> = rows.into_iter().flatten().collect();

    TerrainGrid {
        width,
        height,
        cells,
    }
}
