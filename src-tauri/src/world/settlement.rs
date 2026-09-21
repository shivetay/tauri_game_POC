use serde::{Deserialize, Serialize};

use crate::world::config::{LodLevel, WorldConfig};
use crate::world::elevation::{is_water_biome, COAST_M, HIGH_M, UPLAND_M};
use crate::world::prng::unit_noise;
use crate::world::sampler::TerrainSampler;
use crate::world::types::{TerrainCell, TileType};

/// 1 world unit = 1 km. Marker radius is stored in those units.
const STEP: f64 = 8.0;
const CHANNEL_X: u64 = 41;
const CHANNEL_Y: u64 = 42;
const CHANNEL_POP: u64 = 43;
const SCORE_FLOOR: f32 = 0.18;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SettlementKind {
    Hamlet,
    Village,
    Town,
    City,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settlement {
    pub x: f32,
    pub y: f32,
    pub kind: SettlementKind,
    pub population: u32,
    pub radius: f32,
}

struct Tier {
    kind: SettlementKind,
    pop_min: u32,
    pop_max: u32,
    radius_min: f32,
    radius_max: f32,
    spacing: f32,
    per_land: usize,
    count_min: usize,
    count_max: usize,
}

/// Largest first. Population and radius ranges are inclusive.
const TIERS: [Tier; 4] = [
    Tier {
        kind: SettlementKind::City,
        pop_min: 8_000,
        pop_max: 50_000,
        radius_min: 2.5,
        radius_max: 8.0,
        spacing: 56.0,
        per_land: 350,
        count_min: 1,
        count_max: 6,
    },
    Tier {
        kind: SettlementKind::Town,
        pop_min: 1_200,
        pop_max: 8_000,
        radius_min: 0.9,
        radius_max: 2.5,
        spacing: 28.0,
        per_land: 90,
        count_min: 3,
        count_max: 24,
    },
    Tier {
        kind: SettlementKind::Village,
        pop_min: 150,
        pop_max: 1_200,
        radius_min: 0.35,
        radius_max: 0.9,
        spacing: 14.0,
        per_land: 28,
        count_min: 8,
        count_max: 80,
    },
    Tier {
        kind: SettlementKind::Hamlet,
        pop_min: 20,
        pop_max: 150,
        radius_min: 0.15,
        radius_max: 0.35,
        spacing: 9.0,
        per_land: 22,
        count_min: 12,
        count_max: 100,
    },
];

struct Candidate {
    x: f32,
    y: f32,
    score: f32,
    index: u64,
}

pub fn generate(config: WorldConfig) -> Vec<Settlement> {
    let seed = config.seed;
    let world_w = config.world_width as f64;
    let world_h = config.world_height as f64;
    let sampler = TerrainSampler::new(config);
    let cols = (world_w / STEP).round().max(1.0) as u32;
    let rows = (world_h / STEP).round().max(1.0) as u32;

    let mut candidates: Vec<Candidate> = Vec::new();
    for gy in 0..rows {
        for gx in 0..cols {
            let index = u64::from(gy) * u64::from(cols) + u64::from(gx);
            let jx = unit_noise(seed, CHANNEL_X, index);
            let jy = unit_noise(seed, CHANNEL_Y, index);
            let x = (f64::from(gx) + 0.18 + jx * 0.64) * STEP;
            let y = (f64::from(gy) + 0.18 + jy * 0.64) * STEP;
            if x < 4.0 || y < 4.0 || x > world_w - 4.0 || y > world_h - 4.0 {
                continue;
            }
            let cell = sampler.cell_at(x, y, LodLevel::Macro);
            let Some(score) = site_score(&cell) else {
                continue;
            };
            if score < SCORE_FLOOR {
                continue;
            }
            candidates.push(Candidate {
                x: x as f32,
                y: y as f32,
                score,
                index,
            });
        }
    }

    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.index.cmp(&b.index))
    });

    let n_land = candidates.len();
    let mut used = vec![false; n_land];
    let mut placed_xy: Vec<(f32, f32)> = Vec::new();
    let mut out: Vec<Settlement> = Vec::new();

    for tier in TIERS {
        let want = target_count(n_land, &tier);
        let mut n = 0;
        for i in 0..candidates.len() {
            if n >= want {
                break;
            }
            if used[i] {
                continue;
            }
            let c = &candidates[i];
            if !far_enough(c.x, c.y, tier.spacing, &placed_xy) {
                continue;
            }
            used[i] = true;
            placed_xy.push((c.x, c.y));
            let t = unit_noise(seed, CHANNEL_POP, c.index) as f32;
            out.push(Settlement {
                x: c.x,
                y: c.y,
                kind: tier.kind,
                population: lerp_u32(tier.pop_min, tier.pop_max, t),
                radius: lerp_f32(tier.radius_min, tier.radius_max, t),
            });
            n += 1;
        }
    }

    out
}

fn target_count(n_land: usize, tier: &Tier) -> usize {
    if n_land == 0 {
        return 0;
    }
    (n_land / tier.per_land).clamp(tier.count_min, tier.count_max)
}

fn far_enough(x: f32, y: f32, min_d: f32, placed: &[(f32, f32)]) -> bool {
    let min2 = min_d * min_d;
    placed.iter().all(|(px, py)| {
        let dx = x - px;
        let dy = y - py;
        dx * dx + dy * dy >= min2
    })
}

fn site_score(cell: &TerrainCell) -> Option<f32> {
    if is_water_biome(cell.elevation) {
        return None;
    }
    let biome_s = match cell.biome {
        TileType::Grass => 1.0,
        TileType::Savanna => 0.92,
        TileType::Forest => 0.72,
        TileType::Shrubland => 0.58,
        TileType::Sand => 0.70,
        TileType::RockyShore => 0.62,
        TileType::Desert => 0.28,
        TileType::Rainforest => 0.38,
        TileType::Swamp => 0.12,
        TileType::Tundra | TileType::Mountain | TileType::Snow => return None,
        TileType::Water | TileType::DeepWater => return None,
    };
    let elev_s = if cell.elevation < COAST_M {
        0.82
    } else if cell.elevation < UPLAND_M {
        1.0
    } else if cell.elevation < HIGH_M {
        0.42
    } else {
        return None;
    };
    let moist_s = 1.0 - (cell.moisture - 0.4).abs() * 0.35;
    Some(biome_s * elev_s * moist_s)
}

fn lerp_u32(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    a + ((b - a) as f32 * t).round() as u32
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::config::WorldConfig;
    use crate::world::elevation::WATER_M;

    fn seed_config(seed: u64) -> WorldConfig {
        let mut config = WorldConfig::default();
        config.seed = seed;
        config
    }

    #[test]
    fn seed_6_is_deterministic() {
        let a = generate(seed_config(6));
        let b = generate(seed_config(6));
        assert_eq!(a.len(), b.len());
        assert!(!a.is_empty());
        for (left, right) in a.iter().zip(b.iter()) {
            assert_eq!(left.kind, right.kind);
            assert_eq!(left.population, right.population);
            assert!((left.x - right.x).abs() < 1e-5);
            assert!((left.y - right.y).abs() < 1e-5);
        }
    }

    #[test]
    fn seed_6_has_all_kinds_on_land() {
        let config = seed_config(6);
        let sampler = TerrainSampler::new(config.clone());
        let settlements = generate(config);
        let mut saw = [false; 4];
        for s in &settlements {
            let cell = sampler.cell_at(s.x as f64, s.y as f64, LodLevel::Macro);
            assert!(cell.elevation >= WATER_M, "settlement in water at ({}, {})", s.x, s.y);
            match s.kind {
                SettlementKind::City => {
                    saw[0] = true;
                    assert!((8_000..=50_000).contains(&s.population));
                }
                SettlementKind::Town => {
                    saw[1] = true;
                    assert!((1_200..=8_000).contains(&s.population));
                }
                SettlementKind::Village => {
                    saw[2] = true;
                    assert!((150..=1_200).contains(&s.population));
                }
                SettlementKind::Hamlet => {
                    saw[3] = true;
                    assert!((20..=150).contains(&s.population));
                }
            }
        }
        assert!(saw.iter().all(|v| *v), "missing settlement kinds: {saw:?}");
    }
}
