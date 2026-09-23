use noise::SuperSimplex;

use crate::world::biome::biome;
use crate::world::config::{LodLevel, WorldConfig};
use crate::world::elevation::{
    is_water_biome, norm_to_meters, preserve_macro_sea_class, SEA_LEVEL_NORM, WATER_M,
};
use crate::world::noise::{clamp01, fbm, lerp, redistribute, smoothstep};
use crate::world::prng::{derive_seed, hash_seed};
use crate::world::ridge::orogeny_delta;
use crate::world::river::{RiverNetwork, RiverSample, RIVER_CHANNEL_ELEV_M, RIVER_MOISTURE_BOOST};
use crate::world::shape::shape_value;
use crate::world::types::{RegionBounds, TerrainCell, TileType};

pub struct TerrainSampler {
    config: WorldConfig,
    elev_noise: SuperSimplex,
    moist_noise: SuperSimplex,
    detail_noise: SuperSimplex,
    coast_noise: SuperSimplex,
    ridge_noise: SuperSimplex,
    ridge_warp_noise: SuperSimplex,
    rivers: RiverNetwork,
}

impl TerrainSampler {
    pub fn new(config: WorldConfig) -> Self {
        let elev_seed = hash_seed(config.seed);
        let moist_seed = derive_seed(config.seed, "moisture");
        let detail_seed = derive_seed(config.seed, "detail");
        let coast_seed = derive_seed(config.seed, "coast");
        let ridge_seed = derive_seed(config.seed, "ridge");
        let ridge_warp_seed = derive_seed(config.seed, "ridge-warp");

        let sampler = Self {
            config,
            elev_noise: SuperSimplex::new(elev_seed),
            moist_noise: SuperSimplex::new(moist_seed),
            detail_noise: SuperSimplex::new(detail_seed),
            coast_noise: SuperSimplex::new(coast_seed),
            ridge_noise: SuperSimplex::new(ridge_seed),
            ridge_warp_noise: SuperSimplex::new(ridge_warp_seed),
            rivers: RiverNetwork::empty(0.0, 0.0),
        };

        let seed = sampler.config.seed;
        let ww = sampler.config.world_width;
        let wh = sampler.config.world_height;
        let rivers = RiverNetwork::build(seed, ww, wh, |x, y| {
            sampler.elevation_at(x, y, LodLevel::Macro)
        });
        Self { rivers, ..sampler }
    }

    pub fn elevation_norm_at(&self, world_x: f64, world_y: f64, lod: LodLevel) -> f32 {
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
        let land_t = f64::from(c.land_size.clamp(0.0, 1.0));
        let land_factor = smoothstep(lerp(0.40, 0.30, land_t), lerp(0.46, 0.42, land_t), shape);

        let rough = f64::from(c.terrain_roughness.clamp(0.0, 1.0));
        let continental = redistribute(raw, lerp(1.0, 1.35, rough), 1.0);
        let land_base = 0.38 + continental * lerp(0.06, 0.18, rough);
        let ocean_base = 0.10 + continental * lerp(0.04, 0.10, rough);
        let mut e = lerp(ocean_base, land_base, land_factor);

        e += orogeny_delta(
            &self.ridge_noise,
            &self.ridge_warp_noise,
            c.seed,
            world_x,
            world_y,
            c.world_width as f64,
            c.world_height as f64,
            land_factor,
            f64::from(c.orogeny_strength),
        );

        if land_factor < 0.25 {
            e = e.min(f64::from(SEA_LEVEL_NORM) - 0.03);
        }

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

    pub fn elevation_at(&self, world_x: f64, world_y: f64, lod: LodLevel) -> f32 {
        norm_to_meters(self.elevation_norm_at(world_x, world_y, lod))
    }

    pub(crate) fn river_network(&self) -> &RiverNetwork {
        &self.rivers
    }

    pub(crate) fn river_at(&self, world_x: f64, world_y: f64) -> RiverSample {
        self.rivers.sample(world_x, world_y)
    }

    /// Apply river channel carve + bank moisture. Only carves when already on land.
    fn apply_river(
        &self,
        world_x: f64,
        world_y: f64,
        mut elevation: f32,
        mut moisture: f32,
    ) -> (f32, f32) {
        if is_water_biome(elevation) {
            return (elevation, moisture);
        }

        let river = self.river_at(world_x, world_y);
        moisture = (moisture + river.proximity * RIVER_MOISTURE_BOOST).clamp(0.0, 1.0);

        if river.channel > 0.55 {
            elevation = RIVER_CHANNEL_ELEV_M;
        }

        (elevation, moisture)
    }

    pub fn cell_at(&self, world_x: f64, world_y: f64, lod: LodLevel) -> TerrainCell {
        let base_elev = self.elevation_at(world_x, world_y, lod);
        let base_moist = self.moisture_at(world_x, world_y);
        let (elevation, moisture) = self.apply_river(world_x, world_y, base_elev, base_moist);
        TerrainCell {
            elevation,
            moisture,
            biome: biome(elevation, moisture),
        }
    }

    pub fn moisture_at(&self, world_x: f64, world_y: f64) -> f32 {
        let c = &self.config;
        let wx = (world_x / c.world_width as f64) * c.scale as f64;
        let wy = (world_y / c.world_height as f64) * c.scale as f64;
        let raw_m = fbm(
            &self.moist_noise,
            wx,
            wy,
            c.octaves,
            c.persistence as f64,
            c.lacunarity as f64,
        );
        let m_strength = f64::from(c.moisture_strength.clamp(0.0, 1.0));
        clamp01(0.5 + (raw_m - 0.5) * m_strength) as f32
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

    pub fn config(&self) -> &WorldConfig {
        &self.config
    }

    /// Region elevation (m) and the Macro norm used for sea-class gating.
    fn elevation_and_macro_at_region(
        &self,
        world_x: f64,
        world_y: f64,
        bounds: RegionBounds,
    ) -> (f32, f32) {
        let macro_n = self.elevation_norm_at(world_x, world_y, LodLevel::Macro);
        let micro_n = self.elevation_norm_at(world_x, world_y, LodLevel::Micro);
        let blend = self.region_edge_blend(world_x, world_y, bounds);
        let norm = preserve_macro_sea_class(macro_n, macro_n + (micro_n - macro_n) * blend);
        (norm_to_meters(norm), macro_n)
    }

    /// Elewacja micro z blendem na brzegu regionu
    pub fn elevation_at_region(&self, world_x: f64, world_y: f64, bounds: RegionBounds) -> f32 {
        self.elevation_and_macro_at_region(world_x, world_y, bounds).0
    }

    pub fn cell_at_region(&self, world_x: f64, world_y: f64, bounds: RegionBounds) -> TerrainCell {
        let (base_elev, macro_n) = self.elevation_and_macro_at_region(world_x, world_y, bounds);
        let base_moist = self.moisture_at(world_x, world_y);
        // Only carve when macro land — avoids flipping ocean cells via river noise.
        let (elevation, moisture) = if macro_n >= SEA_LEVEL_NORM {
            self.apply_river(world_x, world_y, base_elev, base_moist)
        } else {
            (base_elev, base_moist)
        };
        TerrainCell {
            elevation,
            moisture,
            biome: biome(elevation, moisture),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::config::WorldConfig;

    #[test]
    fn river_channel_on_land_is_water() {
        let sampler = TerrainSampler::new(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let mut found = false;
        'search: for yi in 0..256u32 {
            for xi in 0..256u32 {
                let x = f64::from(xi) * 2.0;
                let y = f64::from(yi) * 2.0;
                let base = sampler.elevation_at(x, y, LodLevel::Macro);
                if is_water_biome(base) {
                    continue;
                }
                let river = sampler.river_at(x, y);
                if river.channel <= 0.55 {
                    continue;
                }
                let cell = sampler.cell_at(x, y, LodLevel::Macro);
                assert_eq!(cell.biome, TileType::Water);
                assert!(cell.elevation < WATER_M);
                found = true;
                break 'search;
            }
        }
        assert!(found, "expected a land river channel for seed 6");
    }

    #[test]
    fn river_bank_boosts_arid_moisture_toward_fertile_biome() {
        let sampler = TerrainSampler::new(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        // Directly verify moisture boost turns arid lowland into non-Desert.
        let dry = 0.08f32;
        let prox = 0.85f32;
        let boosted = (dry + prox * RIVER_MOISTURE_BOOST).clamp(0.0, 1.0);
        let elev = crate::world::elevation::norm_to_meters(0.45);
        let b = biome(elev, boosted);
        assert_ne!(b, TileType::Desert);
        assert!(matches!(
            b,
            TileType::Savanna | TileType::Grass | TileType::Swamp
        ));

        // And that real samples exist with meaningful bank proximity on land.
        let mut found_bank = false;
        'search: for yi in 0..256u32 {
            for xi in 0..256u32 {
                let x = f64::from(xi) * 2.0;
                let y = f64::from(yi) * 2.0;
                let base_elev = sampler.elevation_at(x, y, LodLevel::Macro);
                if is_water_biome(base_elev) {
                    continue;
                }
                let river = sampler.river_at(x, y);
                if river.proximity < 0.5 || river.channel > 0.55 {
                    continue;
                }
                let cell = sampler.cell_at(x, y, LodLevel::Macro);
                assert!(cell.moisture >= sampler.moisture_at(x, y) - 1e-4);
                found_bank = true;
                break 'search;
            }
        }
        assert!(found_bank, "expected a land bank sample for seed 6");
    }

    #[test]
    fn cell_at_is_deterministic_with_rivers() {
        let sampler = TerrainSampler::new(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let a = sampler.cell_at(140.0, 200.0, LodLevel::Macro);
        let b = sampler.cell_at(140.0, 200.0, LodLevel::Macro);
        assert_eq!(a.biome, b.biome);
        assert!((a.elevation - b.elevation).abs() < 1e-5);
        assert!((a.moisture - b.moisture).abs() < 1e-5);
    }
}
