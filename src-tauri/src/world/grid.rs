use rayon::prelude::*;

use crate::world::config::{LodLevel, WorldConfig};
use crate::world::sampler::TerrainSampler;
use crate::world::types::{ChunkId, RegionBounds, RegionId, TerrainCell, TerrainGrid};

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

pub fn generate_view_grid(
    config: WorldConfig,
    region: RegionId,
    world_x0: f32,
    world_y0: f32,
    span: f32,
) -> TerrainGrid {
    let sampler = TerrainSampler::new(config.clone());
    let region_bounds = RegionBounds::from_region(region, config.region_size);
    let width = config.chunk_resolution;
    let height = config.chunk_resolution;
    let span = span.max(1e-3);

    let rows: Vec<Vec<TerrainCell>> = (0..height)
        .into_par_iter()
        .map(|y| {
            (0..width)
                .map(|x| {
                    let world_x = f64::from(world_x0)
                        + (x as f64 + 0.5) / width as f64 * f64::from(span);
                    let world_y = f64::from(world_y0)
                        + (y as f64 + 0.5) / height as f64 * f64::from(span);
                    sampler.cell_at_region(world_x, world_y, region_bounds)
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

pub fn generate_chunk_grid(
    config: WorldConfig,
    region: RegionId,
    chunk: ChunkId,
) -> TerrainGrid {
    let chunk_bounds =
        RegionBounds::from_chunk(region, chunk, config.region_size, config.chunk_size);
    generate_view_grid(
        config,
        region,
        chunk_bounds.world_x0,
        chunk_bounds.world_y0,
        chunk_bounds.world_x1 - chunk_bounds.world_x0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::elevation::{is_water_biome, WATER_M};
    use crate::world::types::{ChunkId, RegionBounds};

    fn is_water_cell(cell: &TerrainCell) -> bool {
        is_water_biome(cell.elevation) || cell.elevation < WATER_M
    }

    #[test]
    fn region_zoom_preserves_macro_land_and_water() {
        let config = WorldConfig::default();
        let sampler = TerrainSampler::new(config);

        for ry in 0..8 {
            for rx in 0..8 {
                let bounds = RegionBounds::from_region(RegionId { rx, ry }, 64);
                for y in (0..64).step_by(8) {
                    for x in (0..64).step_by(8) {
                        let wx = f64::from(bounds.world_x0) + x as f64 + 0.5;
                        let wy = f64::from(bounds.world_y0) + y as f64 + 0.5;
                        let macro_cell = sampler.cell_at(wx, wy, LodLevel::Macro);
                        let region_cell = sampler.cell_at_region(wx, wy, bounds);
                        assert_eq!(
                            is_water_cell(&macro_cell),
                            is_water_cell(&region_cell),
                            "sea class mismatch at ({wx},{wy}) region ({rx},{ry})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn chunk_zoom_matches_region_at_same_world_point() {
        let config = WorldConfig::default();

        for ry in 0..8 {
            for rx in 0..8 {
                let region = RegionId { rx, ry };
                let region_grid = generate_region_grid(config.clone(), region);
                let region_bounds = RegionBounds::from_region(region, config.region_size);

                for cy in 0..8 {
                    for cx in 0..8 {
                        let chunk = ChunkId { cx, cy };
                        let chunk_grid =
                            generate_chunk_grid(config.clone(), region, chunk);

                        let chunk_bounds = RegionBounds::from_chunk(
                            region,
                            chunk,
                            config.region_size,
                            config.chunk_size,
                        );
                        let wx = f64::from(chunk_bounds.world_x0) + 4.0;
                        let wy = f64::from(chunk_bounds.world_y0) + 4.0;

                        let region_px = ((wx - f64::from(region_bounds.world_x0))
                            / f64::from(region_bounds.world_x1 - region_bounds.world_x0)
                            * config.micro_resolution as f64) as u32;
                        let region_py = ((wy - f64::from(region_bounds.world_y0))
                            / f64::from(region_bounds.world_y1 - region_bounds.world_y0)
                            * config.micro_resolution as f64) as u32;
                        let region_idx = (region_py * config.micro_resolution + region_px) as usize;

                        let chunk_px = ((wx - f64::from(chunk_bounds.world_x0))
                            / f64::from(chunk_bounds.world_x1 - chunk_bounds.world_x0)
                            * config.chunk_resolution as f64) as u32;
                        let chunk_py = ((wy - f64::from(chunk_bounds.world_y0))
                            / f64::from(chunk_bounds.world_y1 - chunk_bounds.world_y0)
                            * config.chunk_resolution as f64) as u32;
                        let chunk_idx = (chunk_py * config.chunk_resolution + chunk_px) as usize;

                        let region_cell = &region_grid.cells[region_idx];
                        let chunk_cell = &chunk_grid.cells[chunk_idx];

                        assert_eq!(
                            region_cell.biome, chunk_cell.biome,
                            "biome mismatch at ({wx},{wy}) region ({rx},{ry}) chunk ({cx},{cy})"
                        );
                        assert_eq!(
                            is_water_cell(region_cell),
                            is_water_cell(chunk_cell),
                            "sea class mismatch at ({wx},{wy}) region ({rx},{ry}) chunk ({cx},{cy})"
                        );
                    }
                }
            }
        }
    }
}
