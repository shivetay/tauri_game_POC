use noise::SuperSimplex;

use crate::world::biome::biome;
use crate::world::config::{LodLevel, WorldConfig};
use crate::world::noise::{fbm, redistribute, clamp01};
use crate::world::prng::hash_seed;
use crate::world::shape::{apply_shape, shape_value};
use crate::world::types::{RegionBounds, TerrainCell, TileType};

pub struct TerrainSampler {
    config: WorldConfig,
    elev_noise: SuperSimplex,
    moist_noise: SuperSimplex,
    detail_noise: SuperSimplex,
    coast_noise: SuperSimplex,
}

impl TerrainSampler {
    pub fn new(config: WorldConfig) -> Self {
        let elev_seed = hash_seed(&config.seed);
        let moist_seed = hash_seed(&format!("{}:moisture", config.seed));
        let detail_seed = hash_seed(&format!("{}:detail", config.seed));
        let coast_seed = hash_seed(&format!("{}:coast", config.seed));

        Self {
            config,
            elev_noise: SuperSimplex::new(elev_seed),
            moist_noise: SuperSimplex::new(moist_seed),
            detail_noise: SuperSimplex::new(detail_seed),
            coast_noise: SuperSimplex::new(coast_seed),
        }
    }

    pub fn elevation_at(&self, world_x: f64, world_y: f64, lod: LodLevel) -> f32 {
        let c = &self.config;
        let wx = (world_x / c.world_width as f64) * c.scale as f64;
        let wy = (world_y / c.world_height as f64) * c.scale as f64;

        let raw = fbm(
            &self.elev_noise,
            wx,
            wy,
            c.octaves,
            c.persistence as f64,
            c.lacunarity as f64,
        );

        let profile = c.resolved_shape_profile();
        let shape = shape_value(profile, world_x, world_y, c, &self.coast_noise);
        let shaped = apply_shape(raw, shape, f64::from(c.island_mix));
        let mut e = redistribute(
            shaped,
            c.redistribution_exponent as f64,
            c.redistribution_fudge as f64,
        );

        if lod == LodLevel::Micro {
            let detail = fbm(
                &self.detail_noise,
                world_x * 0.15,
                world_y * 0.15,
                c.detail_octaves,
                0.5,
                2.0,
            );
            e += (detail - 0.5) * c.detail_strength as f64;
        }

        clamp01(e) as f32
    }

    pub fn moisture_at(&self, world_x: f64, world_y: f64) -> f32 {
        let c = &self.config;
        let wx = (world_x / c.world_width as f64) * c.scale as f64;
        let wy = (world_y / c.world_height as f64) * c.scale as f64;
        fbm(
            &self.moist_noise,
            wx,
            wy,
            c.octaves,
            c.persistence as f64,
            c.lacunarity as f64,
        ) as f32
    }

    pub fn cell_at(&self, world_x: f64, world_y: f64, lod: LodLevel) -> TerrainCell {
        let elevation = self.elevation_at(world_x, world_y, lod);
        let moisture = self.moisture_at(world_x, world_y);
        let biome_type = biome(elevation, moisture);
        TerrainCell {
            elevation,
            moisture,
            biome: biome_type,
        }
    }

    /// Tłumi detail noise na krawędzi regionu (0 = brzeg, 1 = środek)
    pub fn region_edge_blend(&self, world_x: f64, world_y: f64, bounds: RegionBounds) -> f32 {
        let margin = (self.config.region_size as f64 * 0.15).max(4.0);
        let dx = (world_x - f64::from(bounds.world_x0))
            .min(f64::from(bounds.world_x1) - world_x)
            .min(margin);
        let dy = (world_y - f64::from(bounds.world_y0))
            .min(f64::from(bounds.world_y1) - world_y)
            .min(margin);
        let d = dx.min(dy);
        (d / margin).clamp(0.0, 1.0) as f32
    }

    /// Elewacja micro z blendem na brzegu regionu
    pub fn elevation_at_region(
        &self,
        world_x: f64,
        world_y: f64,
        bounds: RegionBounds,
    ) -> f32 {
        let macro_e = self.elevation_at(world_x, world_y, LodLevel::Macro);
        let micro_e = self.elevation_at(world_x, world_y, LodLevel::Micro);
        let blend = self.region_edge_blend(world_x, world_y, bounds);
        macro_e + (micro_e - macro_e) * blend
    }

    pub fn cell_at_region(&self, world_x: f64, world_y: f64, bounds: RegionBounds) -> TerrainCell {
        let elevation = self.elevation_at_region(world_x, world_y, bounds);
        let moisture = self.moisture_at(world_x, world_y);
        TerrainCell {
            elevation,
            moisture,
            biome: biome(elevation, moisture),
        }
    }
}