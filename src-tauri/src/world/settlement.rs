use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

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
const CHANNEL_NAME: u64 = 44;
const CHANNEL_LAYOUT: u64 = 45;
#[cfg(test)]
const CHANNEL_CORE: u64 = 46;
const CHANNEL_ROAD: u64 = 47;
const CHANNEL_BRIDGE: u64 = 48;
const SCORE_FLOOR: f32 = 0.18;
/// Villages below this population stay as a single footprint (no district zones).
const VILLAGE_DISTRICT_POP_MIN: u32 = 500;
const TAU: f32 = std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SettlementKind {
    Hamlet,
    Village,
    Town,
    City,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DistrictKind {
    Center,
    Market,
    Craft,
    Port,
    Temple,
    Noble,
    Forest,
    Residential,
    Outskirts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct District {
    pub kind: DistrictKind,
    pub name: String,
    pub inner: f32,
    pub outer: f32,
    pub a0: f32,
    pub span: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settlement {
    pub x: f32,
    pub y: f32,
    pub kind: SettlementKind,
    pub population: u32,
    pub radius: f32,
    pub name: String,
    pub core_dx: f32,
    pub core_dy: f32,
    pub districts: Vec<District>,
    pub road_approaches: Vec<RoadApproach>,
    pub streets: Vec<Street>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Street {
    pub angle: f32,
    pub kind: RoadKind,
    pub radial: bool,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RoadKind {
    Highway,
    Secondary,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RoadSurface {
    Paved,
    Packed,
    Dirt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoadApproach {
    pub angle: f32,
    pub kind: RoadKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RoadCrossing {
    Bridge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Road {
    pub kind: RoadKind,
    pub surface: RoadSurface,
    pub points: Vec<RoadPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoadPoint {
    pub x: f32,
    pub y: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crossing: Option<RoadCrossing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementMap {
    pub settlements: Vec<Settlement>,
    pub roads: Vec<Road>,
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

pub fn generate(config: WorldConfig) -> SettlementMap {
    generate_sampled(&TerrainSampler::new(config))
}

pub fn generate_sampled(sampler: &TerrainSampler) -> SettlementMap {
    let config = sampler.config();
    let seed = config.seed;
    let world_w = config.world_width as f64;
    let world_h = config.world_height as f64;
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
            let river = sampler.river_at(x, y);
            let Some(score) = site_score(&cell, &river) else {
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
    let mut taken_names: HashSet<String> = HashSet::new();
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
            let radius = lerp_f32(tier.radius_min, tier.radius_max, t);
            out.push(Settlement {
                x: c.x,
                y: c.y,
                kind: tier.kind,
                population: lerp_u32(tier.pop_min, tier.pop_max, t),
                radius,
                name: unique_name(seed, c.index, tier.kind, &mut taken_names),
                core_dx: 0.0,
                core_dy: 0.0,
                districts: Vec::new(),
                road_approaches: Vec::new(),
                streets: Vec::new(),
            });
            n += 1;
        }
    }

    let roads = build_roads(
        seed,
        sampler,
        world_w as f32,
        world_h as f32,
        &mut out,
    );
    refine_street_layouts(seed, sampler, &mut out);
    SettlementMap {
        settlements: out,
        roads,
    }
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

fn site_score(cell: &TerrainCell, river: &crate::world::river::RiverSample) -> Option<f32> {
    if is_water_biome(cell.elevation) || river.channel > 0.55 {
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
    let river_s = 1.0 + 0.55 * river.proximity;
    Some(biome_s * elev_s * moist_s * river_s)
}

const NAME_PREFIX: &[&str] = &[
    "Biało", "Czarno", "Czerwono", "Zielono", "Złoto", "Srebrno", "Nowo", "Staro",
    "Wielko", "Mało", "Górno", "Dolno", "Leśno", "Polno", "Mokro", "Sucho",
    "Kamienno", "Piaskowo", "Miodowo", "Słoneczno", "Wietrzno", "Ciemno", "Jasno",
    "Dziko", "Wilczo", "Orlo", "Sokoło", "Jelenio", "Dębo", "Brzozowo", "Lipowo",
    "Sosnowo", "Grabowo", "Jasiono", "Ostrowo", "Brzego", "Rzeczno", "Jeziorno",
];

const NAME_STEM_CITY: &[&str] = &["gród", "port", "zamek", "stok", "brzeg", "góra"];
const NAME_STEM_TOWN: &[&str] = &["młyn", "most", "wola", "staw", "dwór", "bór"];
const NAME_STEM_VILLAGE: &[&str] = &["las", "gaj", "pole", "dolina", "łąka", "rów"];
const NAME_STEM_HAMLET: &[&str] = &["polana", "zagroda", "kępa", "ostrów", "źródło", "uroczysko"];

fn stems_for(kind: SettlementKind) -> &'static [&'static str] {
    match kind {
        SettlementKind::City => NAME_STEM_CITY,
        SettlementKind::Town => NAME_STEM_TOWN,
        SettlementKind::Village => NAME_STEM_VILLAGE,
        SettlementKind::Hamlet => NAME_STEM_HAMLET,
    }
}

fn pick<'a>(items: &'a [&'a str], seed: u64, channel: u64, index: u64) -> &'a str {
    let n = items.len().max(1) as f64;
    let i = (unit_noise(seed, channel, index) * n).floor() as usize;
    items[i.min(items.len() - 1)]
}

fn compose_name(seed: u64, index: u64, kind: SettlementKind, salt: u64) -> String {
    let prefix = pick(NAME_PREFIX, seed, CHANNEL_NAME, index.wrapping_add(salt * 3));
    let stem = pick(
        stems_for(kind),
        seed,
        CHANNEL_NAME + 1,
        index.wrapping_add(salt * 3 + 1),
    );
    format!("{prefix}{stem}")
}

fn unique_name(
    seed: u64,
    index: u64,
    kind: SettlementKind,
    taken: &mut HashSet<String>,
) -> String {
    for salt in 0u64..64 {
        let name = compose_name(seed, index, kind, salt);
        if taken.insert(name.clone()) {
            return name;
        }
    }
    let fallback = format!("{}-{}", compose_name(seed, index, kind, 0), index);
    taken.insert(fallback.clone());
    fallback
}

fn is_forest_biome(biome: TileType) -> bool {
    matches!(
        biome,
        TileType::Forest | TileType::Rainforest | TileType::Shrubland
    )
}

fn neighbor_dirs(
    sampler: &TerrainSampler,
    x: f32,
    y: f32,
    radius: f32,
) -> (Option<f32>, Option<f32>) {
    let mut water_x = 0.0f64;
    let mut water_y = 0.0f64;
    let mut water_n = 0u32;
    let mut forest_x = 0.0f64;
    let mut forest_y = 0.0f64;
    let mut forest_n = 0u32;
    let dist = f64::from(radius) * 1.15;
    for i in 0..8 {
        let a = f64::from(i) * (std::f64::consts::TAU / 8.0);
        let cell = sampler.cell_at(
            f64::from(x) + a.cos() * dist,
            f64::from(y) + a.sin() * dist,
            LodLevel::Macro,
        );
        if is_water_biome(cell.elevation) {
            water_x += a.cos();
            water_y += a.sin();
            water_n += 1;
        }
        if is_forest_biome(cell.biome) {
            forest_x += a.cos();
            forest_y += a.sin();
            forest_n += 1;
        }
    }
    let home = sampler.cell_at(f64::from(x), f64::from(y), LodLevel::Macro);
    if is_forest_biome(home.biome) {
        forest_n = forest_n.max(1);
    }
    let water_dir = if water_n > 0 {
        Some(water_y.atan2(water_x) as f32)
    } else {
        None
    };
    let forest_dir = if forest_x.abs() + forest_y.abs() > 1e-6 {
        Some(forest_y.atan2(forest_x) as f32)
    } else if forest_n > 0 {
        Some(0.0)
    } else {
        None
    };
    (water_dir, forest_dir)
}

#[cfg(test)]
fn layout_districts(
    seed: u64,
    index: u64,
    kind: SettlementKind,
    water_dir: Option<f32>,
    forest_dir: Option<f32>,
) -> (f32, f32, Vec<District>) {
    if !matches!(kind, SettlementKind::City | SettlementKind::Town) {
        return (0.0, 0.0, Vec::new());
    }

    let n_inner = match kind {
        SettlementKind::City => {
            let t = unit_noise(seed, CHANNEL_LAYOUT, index);
            if t < 0.30 {
                6
            } else if t < 0.70 {
                7
            } else {
                8
            }
        }
        _ => {
            if unit_noise(seed, CHANNEL_LAYOUT, index) < 0.5 {
                5
            } else {
                6
            }
        }
    };

    let offset_roll = unit_noise(seed, CHANNEL_CORE, index) as f32;
    let (core_dx, core_dy) = if offset_roll < 0.35 {
        (0.0, 0.0)
    } else {
        let ang = unit_noise(seed, CHANNEL_CORE, index.wrapping_add(1)) as f32 * TAU;
        let mag = 0.16 + unit_noise(seed, CHANNEL_CORE, index.wrapping_add(2)) as f32 * 0.22;
        (ang.cos() * mag, ang.sin() * mag)
    };
    let core_mag = (core_dx * core_dx + core_dy * core_dy).sqrt();
    let core_r = 0.20 + unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(3)) as f32 * 0.08;
    let mid_r = 0.64 + unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(4)) as f32 * 0.08;
    let outer_r = 1.08 + core_mag;

    let rot = unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(5)) as f32 * TAU;
    let (inner_a0, inner_span) = uneven_slices(seed, index, 50, n_inner, rot);
    let mut inner_kind = vec![DistrictKind::Residential; n_inner];
    let min_housing = if kind == SettlementKind::City { 3 } else { 2 };
    if let Some(i) = first_residential(&inner_kind) {
        convert_slot(&mut inner_kind, i, DistrictKind::Market, min_housing);
    }
    if let Some(mi) = inner_kind.iter().position(|k| *k == DistrictKind::Market) {
        let right = (mi + 1) % n_inner;
        let left = (mi + n_inner - 1) % n_inner;
        if !convert_slot(&mut inner_kind, right, DistrictKind::Craft, min_housing)
            && !convert_slot(&mut inner_kind, left, DistrictKind::Craft, min_housing)
        {
            if let Some(i) = first_residential(&inner_kind) {
                convert_slot(&mut inner_kind, i, DistrictKind::Craft, min_housing);
            }
        }
    }
    if kind == SettlementKind::City {
        if let Some(i) = first_residential(&inner_kind) {
            convert_slot(&mut inner_kind, i, DistrictKind::Noble, min_housing);
        }
        if unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(8)) < 0.62 {
            if let Some(i) = first_residential(&inner_kind) {
                convert_slot(&mut inner_kind, i, DistrictKind::Temple, min_housing);
            }
        }
    } else if unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(8)) < 0.40 {
        if let Some(i) = first_residential(&inner_kind) {
            convert_slot(&mut inner_kind, i, DistrictKind::Temple, min_housing);
        }
    }
    // Forest park in mid ring only when woodland is actually nearby.
    if let Some(dir) = forest_dir {
        let i = nearest_slot(&inner_a0, &inner_span, dir);
        convert_slot(&mut inner_kind, i, DistrictKind::Forest, min_housing);
    }

    let n_outer = if kind == SettlementKind::City { 4 } else { 3 };
    let outer_rot = rot + 0.17;
    let (outer_a0, outer_span) = uneven_slices(seed, index, 80, n_outer, outer_rot);
    let mut outer_kind = vec![DistrictKind::Outskirts; n_outer];
    // Port on the outer ring toward water — never in the civic core.
    if let Some(dir) = water_dir {
        place_nearest(
            &mut outer_kind,
            &outer_a0,
            &outer_span,
            dir,
            DistrictKind::Port,
            1,
        );
    }
    let forest_n = if forest_dir.is_some() {
        if kind == SettlementKind::City {
            2
        } else {
            1
        }
    } else {
        0
    };
    if let Some(dir) = forest_dir {
        place_nearest(
            &mut outer_kind,
            &outer_a0,
            &outer_span,
            dir,
            DistrictKind::Forest,
            forest_n,
        );
    }
    let leftover: usize = outer_kind
        .iter()
        .filter(|k| **k == DistrictKind::Outskirts)
        .count();
    let suburb_n = (if kind == SettlementKind::City { 2 } else { 1 }).min(leftover.saturating_sub(1));
    let mut placed_suburb = 0usize;
    for kind_slot in outer_kind.iter_mut() {
        if placed_suburb >= suburb_n {
            break;
        }
        if *kind_slot == DistrictKind::Outskirts {
            *kind_slot = DistrictKind::Residential;
            placed_suburb += 1;
        }
    }

    let mut districts: Vec<District> = Vec::new();
    let mut taken: HashSet<String> = HashSet::new();
    for i in 0..n_outer {
        let a0 = outer_a0[i];
        let span = outer_span[i];
        let mid = a0 + span * 0.5;
        districts.push(District {
            kind: outer_kind[i],
            name: unique_district_name(
                seed,
                index,
                outer_kind[i],
                mid,
                i as u64 + 20,
                &mut taken,
            ),
            inner: mid_r,
            outer: outer_r,
            a0,
            span,
        });
    }
    for i in 0..n_inner {
        let a0 = inner_a0[i];
        let span = inner_span[i];
        let mid = a0 + span * 0.5;
        districts.push(District {
            kind: inner_kind[i],
            name: unique_district_name(
                seed,
                index,
                inner_kind[i],
                mid,
                i as u64,
                &mut taken,
            ),
            inner: core_r,
            outer: mid_r,
            a0,
            span,
        });
    }
    districts.push(District {
        kind: DistrictKind::Center,
        name: unique_district_name(seed, index, DistrictKind::Center, 0.0, 40, &mut taken),
        inner: 0.0,
        outer: core_r,
        a0: 0.0,
        span: TAU,
    });

    (core_dx, core_dy, districts)
}

fn layout_key(x: f32, y: f32) -> u64 {
    let xi = (x * 1000.0).round() as i64 as u64;
    let yi = (y * 1000.0).round() as i64 as u64;
    xi.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(yi)
}

fn line_diff(a: f32, b: f32) -> f32 {
    angle_diff(a, b).min(angle_diff(a, b + std::f32::consts::PI))
}

fn add_street_line(lines: &mut Vec<(f32, RoadKind)>, angle: f32, kind: RoadKind) {
    if let Some(existing) = lines
        .iter_mut()
        .find(|line| line_diff(line.0, angle) < 0.22)
    {
        if kind_rank(kind) > kind_rank(existing.1) {
            existing.1 = kind;
        }
        return;
    }
    lines.push((angle.rem_euclid(std::f32::consts::PI), kind));
}

fn collect_street_lines(
    seed: u64,
    index: u64,
    kind: SettlementKind,
    approaches: &[RoadApproach],
) -> Vec<(f32, RoadKind)> {
    let mut lines: Vec<(f32, RoadKind)> = Vec::new();
    for ap in approaches {
        add_street_line(&mut lines, ap.angle, ap.kind);
    }
    if lines.is_empty() {
        let a = unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(21)) as f32
            * std::f32::consts::PI;
        let class = match kind {
            SettlementKind::City => RoadKind::Highway,
            SettlementKind::Town => RoadKind::Secondary,
            _ => RoadKind::Local,
        };
        add_street_line(&mut lines, a, class);
    }
    let main = lines[0].0;
    match kind {
        SettlementKind::City => {
            add_street_line(&mut lines, main + std::f32::consts::FRAC_PI_2, RoadKind::Secondary);
            let extra = if unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(22)) < 0.42 {
                2
            } else {
                1
            };
            for i in 0..extra {
                let jitter = (unit_noise(
                    seed,
                    CHANNEL_LAYOUT,
                    index.wrapping_add(23 + i as u64),
                ) as f32
                    - 0.5)
                    * 0.28;
                add_street_line(
                    &mut lines,
                    main + std::f32::consts::PI * (i as f32 + 1.0) / (extra as f32 + 2.0) + jitter,
                    RoadKind::Local,
                );
            }
        }
        SettlementKind::Town => {
            let skew = 0.82
                + unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(22)) as f32 * 0.36;
            add_street_line(
                &mut lines,
                main + std::f32::consts::FRAC_PI_2 * skew,
                RoadKind::Local,
            );
            if unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(23)) < 0.45 {
                add_street_line(
                    &mut lines,
                    main + 0.55
                        + unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(24)) as f32 * 0.4,
                    RoadKind::Local,
                );
            }
        }
        SettlementKind::Village => {
            if unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(22)) < 0.72 {
                add_street_line(
                    &mut lines,
                    main + std::f32::consts::FRAC_PI_2
                        * (0.88
                            + unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(23)) as f32
                                * 0.28),
                    RoadKind::Local,
                );
            }
        }
        SettlementKind::Hamlet => {}
    }
    lines
}

fn layout_on_streets(
    seed: u64,
    index: u64,
    kind: SettlementKind,
    approaches: &[RoadApproach],
    water_dir: Option<f32>,
    forest_dir: Option<f32>,
) -> (f32, f32, Vec<District>, Vec<Street>) {
    let lines = collect_street_lines(seed, index, kind, approaches);
    let mut streets: Vec<Street> = lines
        .iter()
        .map(|(angle, class)| Street {
            angle: *angle,
            kind: *class,
            radial: true,
            radius: 0.0,
        })
        .collect();

    let core_r = match kind {
        SettlementKind::City => 0.20,
        SettlementKind::Town => 0.18,
        SettlementKind::Village => 0.14,
        SettlementKind::Hamlet => 0.12,
    };
    let mid_r = match kind {
        SettlementKind::City => 0.58,
        SettlementKind::Town => 0.62,
        SettlementKind::Village => 0.72,
        SettlementKind::Hamlet => 0.78,
    };
    let outer_r = 1.08;

    if matches!(kind, SettlementKind::City | SettlementKind::Town) {
        streets.push(Street {
            angle: 0.0,
            kind: if kind == SettlementKind::City {
                RoadKind::Secondary
            } else {
                RoadKind::Local
            },
            radial: false,
            radius: mid_r,
        });
    }

    let mut rays: Vec<f32> = lines
        .iter()
        .flat_map(|(a, _)| [*a, *a + std::f32::consts::PI])
        .collect();
    rays.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    rays.dedup_by(|a, b| (*a - *b).abs() < 0.08);
    if rays.len() < 2 {
        rays = vec![0.0, std::f32::consts::PI];
    }

    let n = rays.len();
    let mut a0 = Vec::with_capacity(n);
    let mut span = Vec::with_capacity(n);
    for i in 0..n {
        let start = rays[i];
        let end = if i + 1 < n {
            rays[i + 1]
        } else {
            rays[0] + TAU
        };
        a0.push(start);
        span.push((end - start).max(0.08));
    }

    let mut inner_kind = vec![DistrictKind::Residential; n];
    let min_housing = match kind {
        SettlementKind::City => 3,
        SettlementKind::Town => 2,
        _ => 1,
    };
    if matches!(kind, SettlementKind::City | SettlementKind::Town) {
        if let Some(i) = first_residential(&inner_kind) {
            convert_slot(&mut inner_kind, i, DistrictKind::Market, min_housing);
        }
        if let Some(mi) = inner_kind.iter().position(|k| *k == DistrictKind::Market) {
            let right = (mi + 1) % n;
            let left = (mi + n - 1) % n;
            if !convert_slot(&mut inner_kind, right, DistrictKind::Craft, min_housing)
                && !convert_slot(&mut inner_kind, left, DistrictKind::Craft, min_housing)
            {
                if let Some(i) = first_residential(&inner_kind) {
                    convert_slot(&mut inner_kind, i, DistrictKind::Craft, min_housing);
                }
            }
        }
        if kind == SettlementKind::City {
            if let Some(i) = first_residential(&inner_kind) {
                convert_slot(&mut inner_kind, i, DistrictKind::Noble, min_housing);
            }
            if unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(8)) < 0.62 {
                if let Some(i) = first_residential(&inner_kind) {
                    convert_slot(&mut inner_kind, i, DistrictKind::Temple, min_housing);
                }
            }
        } else if unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(8)) < 0.40 {
            if let Some(i) = first_residential(&inner_kind) {
                convert_slot(&mut inner_kind, i, DistrictKind::Temple, min_housing);
            }
        }
    }
    // Mid-ring woodland only when forest is actually nearby.
    if let Some(dir) = forest_dir {
        let i = nearest_slot(&a0, &span, dir);
        convert_slot(&mut inner_kind, i, DistrictKind::Forest, min_housing);
    }

    let mut outer_kind = vec![DistrictKind::Outskirts; n];
    // Port on the outer ring toward water — never in the civic core.
    if let Some(dir) = water_dir {
        place_nearest(&mut outer_kind, &a0, &span, dir, DistrictKind::Port, 1);
    }
    let forest_n = if forest_dir.is_some() {
        if kind == SettlementKind::City {
            2.min(n)
        } else {
            1.min(n)
        }
    } else {
        0
    };
    if let Some(dir) = forest_dir {
        place_nearest(
            &mut outer_kind,
            &a0,
            &span,
            dir,
            DistrictKind::Forest,
            forest_n,
        );
    }
    if matches!(kind, SettlementKind::Village | SettlementKind::Hamlet) {
        for slot in outer_kind.iter_mut() {
            if *slot == DistrictKind::Outskirts {
                *slot = DistrictKind::Residential;
            }
        }
        if n >= 2 {
            // Keep at least one true outskirts wedge unless it is Port/Forest.
            if let Some(i) = outer_kind.iter().rposition(|k| *k == DistrictKind::Residential) {
                outer_kind[i] = DistrictKind::Outskirts;
            }
        }
    } else {
        let leftover: usize = outer_kind
            .iter()
            .filter(|k| **k == DistrictKind::Outskirts)
            .count();
        let suburb_n =
            (if kind == SettlementKind::City { 2 } else { 1 }).min(leftover.saturating_sub(1));
        let mut placed_suburb = 0usize;
        for slot in outer_kind.iter_mut() {
            if placed_suburb >= suburb_n {
                break;
            }
            if *slot == DistrictKind::Outskirts {
                *slot = DistrictKind::Residential;
                placed_suburb += 1;
            }
        }
    }

    let mut districts: Vec<District> = Vec::new();
    let mut taken: HashSet<String> = HashSet::new();
    let split_rings = matches!(kind, SettlementKind::City | SettlementKind::Town);
    for i in 0..n {
        let mid = a0[i] + span[i] * 0.5;
        if split_rings {
            districts.push(District {
                kind: outer_kind[i],
                name: unique_district_name(
                    seed,
                    index,
                    outer_kind[i],
                    mid,
                    i as u64 + 20,
                    &mut taken,
                ),
                inner: mid_r,
                outer: outer_r,
                a0: a0[i],
                span: span[i],
            });
            districts.push(District {
                kind: inner_kind[i],
                name: unique_district_name(
                    seed,
                    index,
                    inner_kind[i],
                    mid,
                    i as u64,
                    &mut taken,
                ),
                inner: core_r,
                outer: mid_r,
                a0: a0[i],
                span: span[i],
            });
        } else {
            districts.push(District {
                kind: outer_kind[i],
                name: unique_district_name(
                    seed,
                    index,
                    outer_kind[i],
                    mid,
                    i as u64,
                    &mut taken,
                ),
                inner: core_r,
                outer: outer_r,
                a0: a0[i],
                span: span[i],
            });
        }
    }
    if split_rings || kind == SettlementKind::Village {
        districts.push(District {
            kind: DistrictKind::Center,
            name: unique_district_name(seed, index, DistrictKind::Center, 0.0, 40, &mut taken),
            inner: 0.0,
            outer: core_r,
            a0: 0.0,
            span: TAU,
        });
    }

    (0.0, 0.0, districts, streets)
}

fn wants_districts(kind: SettlementKind, population: u32) -> bool {
    match kind {
        SettlementKind::City | SettlementKind::Town => true,
        SettlementKind::Village => population >= VILLAGE_DISTRICT_POP_MIN,
        SettlementKind::Hamlet => false,
    }
}

fn refine_street_layouts(seed: u64, sampler: &TerrainSampler, settlements: &mut [Settlement]) {
    for (i, s) in settlements.iter_mut().enumerate() {
        if !wants_districts(s.kind, s.population) {
            s.core_dx = 0.0;
            s.core_dy = 0.0;
            s.districts.clear();
            s.streets.clear();
            continue;
        }
        let index = layout_key(s.x, s.y).wrapping_add(i as u64);
        let (water_dir, forest_dir) = neighbor_dirs(sampler, s.x, s.y, s.radius);
        let approaches = s.road_approaches.clone();
        let (core_dx, core_dy, districts, streets) = layout_on_streets(
            seed,
            index,
            s.kind,
            &approaches,
            water_dir,
            forest_dir,
        );
        s.core_dx = core_dx;
        s.core_dy = core_dy;
        s.districts = districts;
        s.streets = streets;
    }
}

fn first_residential(kinds: &[DistrictKind]) -> Option<usize> {
    kinds.iter().position(|k| *k == DistrictKind::Residential)
}

fn housing_count(kinds: &[DistrictKind]) -> usize {
    kinds
        .iter()
        .filter(|k| matches!(k, DistrictKind::Residential | DistrictKind::Noble))
        .count()
}

fn convert_slot(
    kinds: &mut [DistrictKind],
    i: usize,
    next: DistrictKind,
    min_housing: usize,
) -> bool {
    if kinds[i] != DistrictKind::Residential {
        return false;
    }
    let keeps_housing = matches!(next, DistrictKind::Residential | DistrictKind::Noble);
    if !keeps_housing && housing_count(kinds).saturating_sub(1) < min_housing {
        return false;
    }
    kinds[i] = next;
    true
}

#[cfg(test)]
fn uneven_slices(
    seed: u64,
    index: u64,
    salt: u64,
    n: usize,
    rot: f32,
) -> (Vec<f32>, Vec<f32>) {
    let mut weights = Vec::with_capacity(n);
    for i in 0..n {
        weights.push(
            0.48 + unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(salt + i as u64)) as f32
                * 1.15,
        );
    }
    let sum: f32 = weights.iter().sum::<f32>().max(0.001);
    let mut a0 = Vec::with_capacity(n);
    let mut span = Vec::with_capacity(n);
    let mut a = rot;
    for w in weights {
        let s = TAU * w / sum;
        a0.push(a);
        span.push(s);
        a += s;
    }
    (a0, span)
}

fn nearest_slot(starts: &[f32], spans: &[f32], dir: f32) -> usize {
    let mut best = 0usize;
    let mut best_d = f32::MAX;
    for i in 0..starts.len() {
        let mid = starts[i] + spans[i] * 0.5;
        let d = angle_diff(mid, dir);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

fn place_nearest(
    kinds: &mut [DistrictKind],
    starts: &[f32],
    spans: &[f32],
    dir: f32,
    kind: DistrictKind,
    n: usize,
) {
    for _ in 0..n {
        let mut best: Option<usize> = None;
        let mut best_d = f32::MAX;
        for i in 0..kinds.len() {
            if kinds[i] != DistrictKind::Outskirts {
                continue;
            }
            let d = angle_diff(starts[i] + spans[i] * 0.5, dir);
            if d < best_d {
                best_d = d;
                best = Some(i);
            }
        }
        if let Some(i) = best {
            kinds[i] = kind;
        } else {
            break;
        }
    }
}

fn angle_diff(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(TAU);
    if d > std::f32::consts::PI {
        TAU - d
    } else {
        d
    }
}

fn unique_district_name(
    seed: u64,
    index: u64,
    kind: DistrictKind,
    mid: f32,
    salt: u64,
    taken: &mut HashSet<String>,
) -> String {
    let base = district_name(seed, index, kind, mid, salt);
    if taken.insert(base.clone()) {
        return base;
    }
    let named = format!("{} ({})", base, cardinal_label(kind, mid));
    if taken.insert(named.clone()) {
        return named;
    }
    let fallback = format!("{base}-{salt}");
    taken.insert(fallback.clone());
    fallback
}

fn district_name(seed: u64, index: u64, kind: DistrictKind, mid: f32, salt: u64) -> String {
    let card = cardinal_label(kind, mid);
    match kind {
        DistrictKind::Center => pick(
            &["Rynek", "Stare Miasto", "Centrum"],
            seed,
            CHANNEL_LAYOUT + 2,
            index.wrapping_add(salt),
        )
        .to_string(),
        DistrictKind::Market => pick(
            &["Targ", "Dzielnica handlowa", "Jatki"],
            seed,
            CHANNEL_LAYOUT + 2,
            index.wrapping_add(salt),
        )
        .to_string(),
        DistrictKind::Craft => pick(
            &[
                "Dzielnica rzemieślnicza",
                "Warsztaty",
                "Kuźnice",
            ],
            seed,
            CHANNEL_LAYOUT + 2,
            index.wrapping_add(salt),
        )
        .to_string(),
        DistrictKind::Port => pick(
            &["Port", "Nabrzeże", "Przystań"],
            seed,
            CHANNEL_LAYOUT + 2,
            index.wrapping_add(salt),
        )
        .to_string(),
        DistrictKind::Forest => pick(
            &["Gaj", "Dzielnica leśna", "Park", "Bór", "Zagajnik"],
            seed,
            CHANNEL_LAYOUT + 2,
            index.wrapping_add(salt),
        )
        .to_string(),
        DistrictKind::Temple => pick(
            &["Dzielnica świątynna", "Ostrów", "Plebania"],
            seed,
            CHANNEL_LAYOUT + 2,
            index.wrapping_add(salt),
        )
        .to_string(),
        DistrictKind::Noble => pick(
            &["Dzielnica zamożna", "Górne Miasto", "Patrycjat"],
            seed,
            CHANNEL_LAYOUT + 2,
            index.wrapping_add(salt),
        )
        .to_string(),
        DistrictKind::Residential => format!("Dzielnica mieszkaniowa {card}"),
        DistrictKind::Outskirts => format!("Przedmieście {card}"),
    }
}

fn cardinal_label(kind: DistrictKind, angle: f32) -> &'static str {
    let i = ((angle / TAU * 8.0).round() as i32).rem_euclid(8) as usize;
    let fem = matches!(
        kind,
        DistrictKind::Residential | DistrictKind::Noble | DistrictKind::Temple
    );
    if fem {
        [
            "wschodnia",
            "południowo-wschodnia",
            "południowa",
            "południowo-zachodnia",
            "zachodnia",
            "północno-zachodnia",
            "północna",
            "północno-wschodnia",
        ][i]
    } else {
        [
            "wschodnie",
            "południowo-wschodnie",
            "południowe",
            "południowo-zachodnie",
            "zachodnie",
            "północno-zachodnie",
            "północne",
            "północno-wschodnie",
        ][i]
    }
}

fn dist2(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

fn kind_rank(kind: RoadKind) -> u8 {
    match kind {
        RoadKind::Highway => 2,
        RoadKind::Secondary => 1,
        RoadKind::Local => 0,
    }
}

fn add_approach(approaches: &mut Vec<RoadApproach>, angle: f32, kind: RoadKind) {
    if let Some(existing) = approaches
        .iter_mut()
        .find(|a| angle_diff(a.angle, angle) < 0.32)
    {
        if kind_rank(kind) > kind_rank(existing.kind) {
            existing.kind = kind;
        }
        return;
    }
    approaches.push(RoadApproach { angle, kind });
}

fn has_pair(links: &[(RoadKind, usize, usize)], a: usize, b: usize) -> bool {
    links.iter().any(|(_, x, y)| (*x == a && *y == b) || (*x == b && *y == a))
}

fn nearest_of(settlements: &[Settlement], i: usize, pool: &[usize]) -> Option<(f32, usize)> {
    let s = &settlements[i];
    let mut best: Option<(f32, usize)> = None;
    for &j in pool {
        if j == i {
            continue;
        }
        let d = dist2(s.x, s.y, settlements[j].x, settlements[j].y).sqrt();
        if best.map(|(bd, _)| d < bd).unwrap_or(true) {
            best = Some((d, j));
        }
    }
    best
}

fn pick_surface(
    seed: u64,
    index: u64,
    kind: RoadKind,
    pop_a: u32,
    pop_b: u32,
) -> RoadSurface {
    let n = unit_noise(seed, CHANNEL_ROAD, 11_000 + index);
    let small = pop_a.min(pop_b) < 2_500;
    match kind {
        RoadKind::Highway => {
            if n < 0.12 {
                RoadSurface::Packed
            } else {
                RoadSurface::Paved
            }
        }
        RoadKind::Secondary => {
            if n < 0.18 || (small && n < 0.42) {
                RoadSurface::Dirt
            } else {
                RoadSurface::Packed
            }
        }
        RoadKind::Local => {
            if n < 0.12 {
                RoadSurface::Packed
            } else {
                RoadSurface::Dirt
            }
        }
    }
}

fn wobble_amt(kind: RoadKind, surface: RoadSurface) -> f32 {
    match (kind, surface) {
        (RoadKind::Highway, RoadSurface::Paved) => 0.032,
        (RoadKind::Highway, _) => 0.045,
        (RoadKind::Secondary, RoadSurface::Packed) => 0.062,
        (RoadKind::Secondary, _) => 0.082,
        (RoadKind::Local, _) => 0.11,
    }
}

fn build_roads(
    seed: u64,
    sampler: &TerrainSampler,
    world_w: f32,
    world_h: f32,
    settlements: &mut [Settlement],
) -> Vec<Road> {
    let cities: Vec<usize> = settlements
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind == SettlementKind::City)
        .map(|(i, _)| i)
        .collect();
    let towns: Vec<usize> = settlements
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind == SettlementKind::Town)
        .map(|(i, _)| i)
        .collect();
    let hubs: Vec<usize> = cities.iter().copied().chain(towns.iter().copied()).collect();

    let mut parent: Vec<usize> = (0..settlements.len()).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }

    let mut city_pairs: Vec<(f32, usize, usize)> = Vec::new();
    for i in 0..cities.len() {
        for j in i + 1..cities.len() {
            let a = cities[i];
            let b = cities[j];
            city_pairs.push((
                dist2(settlements[a].x, settlements[a].y, settlements[b].x, settlements[b].y)
                    .sqrt(),
                a,
                b,
            ));
        }
    }
    city_pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut links: Vec<(RoadKind, usize, usize)> = Vec::new();
    for (d, a, b) in &city_pairs {
        let pa = find(&mut parent, *a);
        let pb = find(&mut parent, *b);
        if pa != pb {
            parent[pa] = pb;
            links.push((RoadKind::Highway, *a, *b));
        } else if *d < 80.0
            && unit_noise(seed, CHANNEL_ROAD, (*a as u64) * 1009 + *b as u64) < 0.14
        {
            links.push((RoadKind::Highway, *a, *b));
        }
    }

    for &t in &towns {
        let Some((d, c)) = nearest_of(settlements, t, &cities) else {
            continue;
        };
        let n = unit_noise(seed, CHANNEL_ROAD, 3_000 + t as u64);
        if settlements[t].population >= 4_000 && d < 58.0 && n < 0.32 {
            if !has_pair(&links, t, c) {
                links.push((RoadKind::Highway, t, c));
            }
        }
    }

    for &t in &towns {
        let Some((d, h)) = nearest_of(settlements, t, &hubs) else {
            continue;
        };
        if has_pair(&links, t, h) {
            continue;
        }
        let n = unit_noise(seed, CHANNEL_ROAD, 4_000 + t as u64);
        if (d < 70.0 && n < 0.78) || (d < 95.0 && n < 0.4) {
            links.push((RoadKind::Secondary, t, h));
        }
    }

    for i in 0..towns.len() {
        for j in i + 1..towns.len() {
            let a = towns[i];
            let b = towns[j];
            if has_pair(&links, a, b) {
                continue;
            }
            let d = dist2(
                settlements[a].x,
                settlements[a].y,
                settlements[b].x,
                settlements[b].y,
            )
            .sqrt();
            if d < 48.0
                && unit_noise(seed, CHANNEL_ROAD, 5_000 + a as u64 * 17 + b as u64) < 0.2
            {
                links.push((RoadKind::Secondary, a, b));
            }
        }
    }

    for (i, s) in settlements.iter().enumerate() {
        if s.kind != SettlementKind::Village {
            continue;
        }
        let Some((d, h)) = nearest_of(settlements, i, &hubs) else {
            continue;
        };
        let n = unit_noise(seed, CHANNEL_ROAD, 7_000 + i as u64);
        if (d < 28.0 && n < 0.55) || (d < 40.0 && n < 0.38) {
            links.push((RoadKind::Local, i, h));
        }
    }

    for (i, s) in settlements.iter().enumerate() {
        if s.kind != SettlementKind::Hamlet {
            continue;
        }
        let mut pool: Vec<usize> = Vec::new();
        for (j, other) in settlements.iter().enumerate() {
            if i != j && other.kind != SettlementKind::Hamlet {
                pool.push(j);
            }
        }
        let Some((d, j)) = nearest_of(settlements, i, &pool) else {
            continue;
        };
        if d < 14.0 && unit_noise(seed, CHANNEL_ROAD, 9_000 + i as u64) < 0.26 {
            links.push((RoadKind::Local, i, j));
        }
    }

    let mut roads: Vec<Road> = Vec::new();
    let mut bridges = BridgeRegistry::default();
    for (k, (kind, a, b)) in links.iter().enumerate() {
        let ax = settlements[*a].x;
        let ay = settlements[*a].y;
        let bx = settlements[*b].x;
        let by = settlements[*b].y;
        let surface = pick_surface(
            seed,
            k as u64,
            *kind,
            settlements[*a].population,
            settlements[*b].population,
        );
        let points = trace_road(
            sampler,
            seed,
            k as u64,
            *kind,
            ax,
            ay,
            bx,
            by,
            wobble_amt(*kind, surface),
            world_w,
            world_h,
            &bridges,
        );
        if points.len() < 2 {
            continue;
        }
        bridges.register_road(&points);
        let ang_a = (points[1].y - ay).atan2(points[1].x - ax);
        let last = points.len() - 1;
        let ang_b = (points[last - 1].y - by).atan2(points[last - 1].x - bx);
        add_approach(&mut settlements[*a].road_approaches, ang_a, *kind);
        add_approach(&mut settlements[*b].road_approaches, ang_b, *kind);
        roads.push(Road {
            kind: *kind,
            surface,
            points,
        });
    }

    // Second pass: early roads must respect bridges registered by later links
    // (no polyline may cut through an existing shore→shore span).
    let mut all_bridges = BridgeRegistry::default();
    for road in &roads {
        all_bridges.register_road(&road.points);
    }
    for road in &mut roads {
        let redone = finalize_bridge_polyline(sampler, road.points.clone(), &all_bridges);
        if redone.len() >= 2 {
            road.points = redone;
        }
    }
    // Refresh registry after rewrites, then one more tighten pass.
    all_bridges = BridgeRegistry::default();
    for road in &roads {
        all_bridges.register_road(&road.points);
    }
    for road in &mut roads {
        let redone = finalize_bridge_polyline(sampler, road.points.clone(), &all_bridges);
        if redone.len() >= 2 {
            road.points = redone;
        }
    }

    roads
}

fn road_pt(x: f32, y: f32) -> RoadPoint {
    RoadPoint {
        x,
        y,
        crossing: None,
    }
}

const ROAD_CELL: f32 = 6.0;
/// Snap / share when span endpoints are this close.
const BRIDGE_SHARE_R: f32 = 12.0;
/// No second span inside this catchment.
const BRIDGE_CATCHMENT_R: f32 = 32.0;
const BRIDGE_SHARE_EXTRA: u32 = 6;
const BRIDGE_NEW_EXTRA: u32 = 90;
/// Max shore-to-shore jump length (world units).
const BRIDGE_MAX_LEN: f32 = 22.0;

/// One shore→shore portal. Roads may meet only on dry endpoints — never mid-river.
#[derive(Clone, Copy)]
struct BridgeSpan {
    a: (f32, f32),
    b: (f32, f32),
}

impl BridgeSpan {
    fn mid(self) -> (f32, f32) {
        ((self.a.0 + self.b.0) * 0.5, (self.a.1 + self.b.1) * 0.5)
    }

    fn endpoint_near(self, x: f32, y: f32, r: f32) -> bool {
        dist2(x, y, self.a.0, self.a.1) <= r * r || dist2(x, y, self.b.0, self.b.1) <= r * r
    }

    fn same_as(self, a: (f32, f32), b: (f32, f32)) -> bool {
        let r = BRIDGE_SHARE_R;
        (dist2(self.a.0, self.a.1, a.0, a.1) <= r * r
            && dist2(self.b.0, self.b.1, b.0, b.1) <= r * r)
            || (dist2(self.a.0, self.a.1, b.0, b.1) <= r * r
                && dist2(self.b.0, self.b.1, a.0, a.1) <= r * r)
    }

    /// True when spans cross in their interiors (shared shore portals do not count).
    fn crosses(self, other: BridgeSpan) -> bool {
        if self.same_as(other.a, other.b) {
            return false;
        }
        // Touching at a shared portal is a land junction, not a mid-river X.
        let touch = self.endpoint_near(other.a.0, other.a.1, BRIDGE_SHARE_R * 0.55)
            || self.endpoint_near(other.b.0, other.b.1, BRIDGE_SHARE_R * 0.55);
        if touch {
            return false;
        }
        segments_properly_intersect(self.a, self.b, other.a, other.b)
    }
}

/// Proper segment intersection (excludes endpoint-only touches).
fn segments_properly_intersect(
    a1: (f32, f32),
    a2: (f32, f32),
    b1: (f32, f32),
    b2: (f32, f32),
) -> bool {
    fn orient(p: (f32, f32), q: (f32, f32), r: (f32, f32)) -> f32 {
        (q.1 - p.1) * (r.0 - q.0) - (q.0 - p.0) * (r.1 - q.1)
    }
    fn on_seg(p: (f32, f32), q: (f32, f32), r: (f32, f32)) -> bool {
        q.0 >= p.0.min(r.0) - 1e-3
            && q.0 <= p.0.max(r.0) + 1e-3
            && q.1 >= p.1.min(r.1) - 1e-3
            && q.1 <= p.1.max(r.1) + 1e-3
    }
    let o1 = orient(a1, a2, b1);
    let o2 = orient(a1, a2, b2);
    let o3 = orient(b1, b2, a1);
    let o4 = orient(b1, b2, a2);
    if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
        return true;
    }
    // Colinear overlaps count as illegal dual spans on the same reach.
    const EPS: f32 = 1e-2;
    if o1.abs() <= EPS && on_seg(a1, b1, a2) {
        return true;
    }
    if o2.abs() <= EPS && on_seg(a1, b2, a2) {
        return true;
    }
    if o3.abs() <= EPS && on_seg(b1, a1, b2) {
        return true;
    }
    if o4.abs() <= EPS && on_seg(b1, a2, b2) {
        return true;
    }
    false
}

#[derive(Default)]
struct BridgeRegistry {
    spans: Vec<BridgeSpan>,
}

impl BridgeRegistry {
    fn find_span(&self, a: (f32, f32), b: (f32, f32)) -> Option<BridgeSpan> {
        self.spans.iter().copied().find(|s| s.same_as(a, b))
    }

    fn nearest_mid(&self, x: f32, y: f32, max_r: f32) -> Option<BridgeSpan> {
        let max2 = max_r * max_r;
        let mut best: Option<(f32, BridgeSpan)> = None;
        for s in &self.spans {
            let m = s.mid();
            let d2 = dist2(x, y, m.0, m.1);
            if d2 > max2 {
                continue;
            }
            if best.map(|(d, _)| d2 < d).unwrap_or(true) {
                best = Some((d2, *s));
            }
        }
        best.map(|(_, s)| s)
    }

    fn blocks_new_near(&self, x: f32, y: f32) -> bool {
        self.nearest_mid(x, y, BRIDGE_CATCHMENT_R).is_some()
    }

    /// Existing span that this candidate must reuse (same, nearby, or would cross).
    fn resolve_span(&self, candidate: BridgeSpan) -> Option<BridgeSpan> {
        if let Some(s) = self.find_span(candidate.a, candidate.b) {
            return Some(s);
        }
        let mid = candidate.mid();
        if let Some(s) = self.nearest_mid(mid.0, mid.1, BRIDGE_CATCHMENT_R) {
            return Some(s);
        }
        // Geometric cross even outside catchment → force the crossed span.
        for s in &self.spans {
            if s.crosses(candidate) {
                return Some(*s);
            }
        }
        None
    }

    /// Span whose open segment is cut by chord `p→q` (not when `p→q` is that span).
    fn span_cut_by(&self, p: (f32, f32), q: (f32, f32)) -> Option<BridgeSpan> {
        let chord = BridgeSpan { a: p, b: q };
        for s in &self.spans {
            if s.same_as(p, q) {
                continue;
            }
            if s.crosses(chord) {
                return Some(*s);
            }
        }
        None
    }

    fn conflicts_new(&self, candidate: BridgeSpan) -> bool {
        self.spans.iter().any(|s| {
            !s.same_as(candidate.a, candidate.b)
                && (s.crosses(candidate)
                    || dist2(
                        s.mid().0,
                        s.mid().1,
                        candidate.mid().0,
                        candidate.mid().1,
                    ) <= BRIDGE_CATCHMENT_R * BRIDGE_CATCHMENT_R)
        })
    }

    fn register(&mut self, span: BridgeSpan) {
        if self.resolve_span(span).is_some() {
            return;
        }
        if self.conflicts_new(span) {
            return;
        }
        self.spans.push(span);
    }

    fn register_road(&mut self, points: &[RoadPoint]) {
        for i in 1..points.len().saturating_sub(1) {
            if points[i].crossing != Some(RoadCrossing::Bridge) {
                continue;
            }
            let prev = &points[i - 1];
            let next = &points[i + 1];
            if prev.crossing.is_some() || next.crossing.is_some() {
                continue;
            }
            self.register(BridgeSpan {
                a: (prev.x, prev.y),
                b: (next.x, next.y),
            });
        }
    }
}

fn is_dry_land(sampler: &TerrainSampler, x: f32, y: f32) -> bool {
    !is_water_biome(sampler.cell_at(x as f64, y as f64, LodLevel::Macro).elevation)
}

fn is_river_water(sampler: &TerrainSampler, x: f32, y: f32) -> bool {
    let cell = sampler.cell_at(x as f64, y as f64, LodLevel::Macro);
    if !is_water_biome(cell.elevation) {
        return false;
    }
    sampler.river_at(x as f64, y as f64).channel > 0.55
}

/// Land cells only — water is never a walkable node (bridges are jump edges).
fn road_cell_pass(
    sampler: &TerrainSampler,
    x: f32,
    y: f32,
    _kind: RoadKind,
    _seed: u64,
    _bridges: &BridgeRegistry,
) -> Option<(u32, Option<RoadCrossing>)> {
    if !is_dry_land(sampler, x, y) {
        return None;
    }
    let elev = sampler.cell_at(x as f64, y as f64, LodLevel::Macro).elevation;
    let cost = if elev >= HIGH_M {
        16
    } else if elev >= UPLAND_M {
        12
    } else {
        10
    };
    Some((cost, None))
}

fn river_bridge_allowed(kind: RoadKind, seed: u64, x: f32, y: f32) -> bool {
    let gx = (x / ROAD_CELL).floor() as i64;
    let gy = (y / ROAD_CELL).floor() as i64;
    let key = (gx.wrapping_mul(73856093) ^ gy.wrapping_mul(19349663) ^ kind_rank(kind) as i64) as u64;
    let roll = unit_noise(seed, CHANNEL_BRIDGE, key);
    match kind {
        RoadKind::Highway => true,
        RoadKind::Secondary => roll < 0.72,
        RoadKind::Local => roll < 0.30,
    }
}

/// Validate a dry→dry segment that must cross only river water; returns span + toll.
fn shore_to_shore_span(
    sampler: &TerrainSampler,
    kind: RoadKind,
    seed: u64,
    a: (f32, f32),
    b: (f32, f32),
    bridges: &BridgeRegistry,
) -> Option<(u32, BridgeSpan)> {
    if !is_dry_land(sampler, a.0, a.1) || !is_dry_land(sampler, b.0, b.1) {
        return None;
    }
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < ROAD_CELL * 0.75 || dist > BRIDGE_MAX_LEN {
        return None;
    }
    let n = ((dist / 1.5).ceil() as usize).clamp(4, 24);
    let mut water = 0usize;
    let mut mid = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
    let mut saw_mid = false;
    for i in 1..n {
        let t = i as f32 / n as f32;
        let x = a.0 + dx * t;
        let y = a.1 + dy * t;
        if is_dry_land(sampler, x, y) {
            continue;
        }
        if !is_river_water(sampler, x, y) {
            return None; // ocean / lake
        }
        water += 1;
        if !saw_mid {
            mid = (x, y);
            saw_mid = true;
        }
    }
    if water == 0 {
        return None; // no river to cross
    }
    // Must not leave dry pockets mid-span (fork bait): water should be contiguous chunk.
    // Soft check: require water samples roughly in the middle third+.
    let candidate = BridgeSpan { a, b };
    if let Some(existing) = bridges.resolve_span(candidate) {
        return Some((BRIDGE_SHARE_EXTRA, existing));
    }
    if bridges.conflicts_new(candidate) || bridges.blocks_new_near(mid.0, mid.1) {
        // Parallel / crossing span blocked — A* must walk to an existing portal.
        return None;
    }
    if !river_bridge_allowed(kind, seed, mid.0, mid.1) {
        return None;
    }
    Some((BRIDGE_NEW_EXTRA, candidate))
}

/// From a dry cell, list opposite-bank dry cells reachable by a single bridge jump.
fn bridge_jumps_from(
    sampler: &TerrainSampler,
    kind: RoadKind,
    seed: u64,
    x: i32,
    y: i32,
    cols: i32,
    rows: i32,
    bridges: &BridgeRegistry,
) -> Vec<(i32, i32, u32, BridgeSpan)> {
    let ax = (x as f32 + 0.5) * ROAD_CELL;
    let ay = (y as f32 + 0.5) * ROAD_CELL;
    let max_cells = (BRIDGE_MAX_LEN / ROAD_CELL).ceil() as i32;
    let mut out = Vec::new();
    for dy in -max_cells..=max_cells {
        for dx in -max_cells..=max_cells {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || ny < 0 || nx >= cols || ny >= rows {
                continue;
            }
            let bx = (nx as f32 + 0.5) * ROAD_CELL;
            let by = (ny as f32 + 0.5) * ROAD_CELL;
            let Some((toll, span)) =
                shore_to_shore_span(sampler, kind, seed, (ax, ay), (bx, by), bridges)
            else {
                continue;
            };
            out.push((nx, ny, toll, span));
        }
    }
    // Also: if a registered span has an endpoint near here, jump to the other end.
    for s in &bridges.spans {
        let (from, to) = if s.endpoint_near(ax, ay, BRIDGE_SHARE_R) {
            if dist2(ax, ay, s.a.0, s.a.1) <= dist2(ax, ay, s.b.0, s.b.1) {
                (s.a, s.b)
            } else {
                (s.b, s.a)
            }
        } else {
            continue;
        };
        let tx = (to.0 / ROAD_CELL).floor() as i32;
        let ty = (to.1 / ROAD_CELL).floor() as i32;
        if tx < 0 || ty < 0 || tx >= cols || ty >= rows {
            continue;
        }
        out.push((tx, ty, BRIDGE_SHARE_EXTRA, *s));
        let _ = from;
    }
    out
}


#[derive(Copy, Clone, Eq, PartialEq)]
struct PathNode {
    cost: u32,
    x: i32,
    y: i32,
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.x.cmp(&other.x))
            .then_with(|| self.y.cmp(&other.y))
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn snap_land_cell(
    sampler: &TerrainSampler,
    x: f32,
    y: f32,
    cols: i32,
    rows: i32,
) -> Option<(i32, i32)> {
    let gx = (x / ROAD_CELL).floor() as i32;
    let gy = (y / ROAD_CELL).floor() as i32;
    for r in 0i32..6 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r && r > 0 {
                    continue;
                }
                let cx = gx + dx;
                let cy = gy + dy;
                if cx < 0 || cy < 0 || cx >= cols || cy >= rows {
                    continue;
                }
                let wx = (cx as f32 + 0.5) * ROAD_CELL;
                let wy = (cy as f32 + 0.5) * ROAD_CELL;
                if is_dry_land(sampler, wx, wy) {
                    return Some((cx, cy));
                }
            }
        }
    }
    None
}

fn land_path(
    sampler: &TerrainSampler,
    seed: u64,
    kind: RoadKind,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    world_w: f32,
    world_h: f32,
    bridges: &BridgeRegistry,
) -> Vec<RoadPoint> {
    let cols = (world_w / ROAD_CELL).ceil() as i32;
    let rows = (world_h / ROAD_CELL).ceil() as i32;
    let Some(start) = snap_land_cell(sampler, x0, y0, cols, rows) else {
        return Vec::new();
    };
    let Some(goal) = snap_land_cell(sampler, x1, y1, cols, rows) else {
        return Vec::new();
    };
    if start == goal {
        return vec![road_pt(x0, y0), road_pt(x1, y1)];
    }

    let n = (cols * rows) as usize;
    let mut dist = vec![u32::MAX; n];
    let mut prev = vec![-1i32; n];
    // When arriving via a bridge jump, store the shore→shore span.
    let mut prev_span: Vec<Option<BridgeSpan>> = vec![None; n];
    let idx = |x: i32, y: i32| (y * cols + x) as usize;
    let mut heap = BinaryHeap::new();
    dist[idx(start.0, start.1)] = 0;
    heap.push(PathNode {
        cost: 0,
        x: start.0,
        y: start.1,
    });

    let dirs = [
        (1, 0, 10u32),
        (-1, 0, 10),
        (0, 1, 10),
        (0, -1, 10),
        (1, 1, 14),
        (1, -1, 14),
        (-1, 1, 14),
        (-1, -1, 14),
    ];

    while let Some(PathNode { cost, x, y }) = heap.pop() {
        if cost != dist[idx(x, y)] {
            continue;
        }
        if (x, y) == goal {
            break;
        }
        let cx = (x as f32 + 0.5) * ROAD_CELL;
        let cy = (y as f32 + 0.5) * ROAD_CELL;

        // Walk on dry land only (no stepping into water).
        for (dx, dy, step) in dirs {
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || ny < 0 || nx >= cols || ny >= rows {
                continue;
            }
            if dx != 0 && dy != 0 {
                let ax = (x as f32 + dx as f32 + 0.5) * ROAD_CELL;
                let ay = (y as f32 + 0.5) * ROAD_CELL;
                let bx = (x as f32 + 0.5) * ROAD_CELL;
                let by = (y as f32 + dy as f32 + 0.5) * ROAD_CELL;
                if !is_dry_land(sampler, ax, ay) || !is_dry_land(sampler, bx, by) {
                    continue;
                }
            }
            let wx = (nx as f32 + 0.5) * ROAD_CELL;
            let wy = (ny as f32 + 0.5) * ROAD_CELL;
            let Some((terrain, _)) = road_cell_pass(sampler, wx, wy, kind, seed, bridges) else {
                continue;
            };
            let ni = idx(nx, ny);
            if straight_dry_land(sampler, cx, cy, wx, wy) {
                let next = cost.saturating_add(step * terrain / 10);
                if next < dist[ni] {
                    dist[ni] = next;
                    prev[ni] = idx(x, y) as i32;
                    prev_span[ni] = None;
                    heap.push(PathNode {
                        cost: next,
                        x: nx,
                        y: ny,
                    });
                }
            } else if let Some((toll, span)) =
                shore_to_shore_span(sampler, kind, seed, (cx, cy), (wx, wy), bridges)
            {
                // Thin river between neighbouring cells → short shore→shore span.
                let next = cost.saturating_add(toll);
                if next < dist[ni] {
                    dist[ni] = next;
                    prev[ni] = idx(x, y) as i32;
                    prev_span[ni] = Some(span);
                    heap.push(PathNode {
                        cost: next,
                        x: nx,
                        y: ny,
                    });
                }
            }
        }

        // Shore→shore bridge jumps (single span, no mid-river junctions).
        for (nx, ny, toll, span) in
            bridge_jumps_from(sampler, kind, seed, x, y, cols, rows, bridges)
        {
            let next = cost.saturating_add(toll);
            let ni = idx(nx, ny);
            if next < dist[ni] {
                dist[ni] = next;
                prev[ni] = idx(x, y) as i32;
                prev_span[ni] = Some(span);
                heap.push(PathNode {
                    cost: next,
                    x: nx,
                    y: ny,
                });
            }
        }
    }

    if dist[idx(goal.0, goal.1)] == u32::MAX {
        return Vec::new();
    }

    let mut cells = Vec::new();
    let mut spans = Vec::new();
    let mut cur = idx(goal.0, goal.1) as i32;
    while cur >= 0 {
        let i = cur as usize;
        let cx = (i as i32) % cols;
        let cy = (i as i32) / cols;
        cells.push((cx, cy));
        spans.push(prev_span[i]);
        cur = prev[i];
        if (cx, cy) == start {
            break;
        }
    }
    cells.reverse();
    spans.reverse();
    if cells.is_empty() {
        return Vec::new();
    }

    let mut points = Vec::with_capacity(cells.len() * 2 + 2);
    points.push(road_pt(x0, y0));
    for (i, (cx, cy)) in cells.iter().enumerate() {
        let wx = (*cx as f32 + 0.5) * ROAD_CELL;
        let wy = (*cy as f32 + 0.5) * ROAD_CELL;
        if let Some(span) = spans[i] {
            // Arrive via bridge: emit near portal, bridge mid, then this shore.
            // Orient span so `b` is the arrival cell.
            let (enter, leave) = if dist2(span.b.0, span.b.1, wx, wy)
                <= dist2(span.a.0, span.a.1, wx, wy)
            {
                (span.a, span.b)
            } else {
                (span.b, span.a)
            };
            let mid = span.mid();
            push_road_pt_dedup(&mut points, road_pt(enter.0, enter.1));
            push_road_pt_dedup(
                &mut points,
                RoadPoint {
                    x: mid.0,
                    y: mid.1,
                    crossing: Some(RoadCrossing::Bridge),
                },
            );
            push_road_pt_dedup(&mut points, road_pt(leave.0, leave.1));
        } else {
            push_road_pt_dedup(&mut points, road_pt(wx, wy));
        }
    }
    push_road_pt_dedup(&mut points, road_pt(x1, y1));
    finalize_bridge_polyline(sampler, simplify_road(sampler, points), bridges)
}

fn simplify_road(sampler: &TerrainSampler, points: Vec<RoadPoint>) -> Vec<RoadPoint> {
    if points.len() <= 3 {
        return points;
    }
    let mut out = vec![points[0].clone()];
    for i in 1..points.len() - 1 {
        let a = out.last().unwrap();
        let b = &points[i];
        let c = &points[i + 1];
        let keep_bridge = b.crossing.is_some()
            || a.crossing.is_some()
            || c.crossing.is_some();
        let abx = b.x - a.x;
        let aby = b.y - a.y;
        let bcx = c.x - b.x;
        let bcy = c.y - b.y;
        let cross = (abx * bcy - aby * bcx).abs();
        // Never drop a bend that would create a dry→dry chord across water.
        let chord_ok = straight_dry_land(sampler, a.x, a.y, c.x, c.y);
        if keep_bridge || cross > 4.0 || !chord_ok {
            out.push(b.clone());
        }
    }
    out.push(points[points.len() - 1].clone());
    out
}

fn straight_dry_land(sampler: &TerrainSampler, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let dist = (dx * dx + dy * dy).sqrt();
    let n = ((dist / 3.0).ceil() as usize).clamp(2, 80);
    for i in 1..n {
        let t = i as f32 / n as f32;
        if !is_dry_land(sampler, x0 + dx * t, y0 + dy * t) {
            return false;
        }
    }
    true
}

fn trace_road(
    sampler: &TerrainSampler,
    seed: u64,
    index: u64,
    kind: RoadKind,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    wobble: f32,
    world_w: f32,
    world_h: f32,
    bridges: &BridgeRegistry,
) -> Vec<RoadPoint> {
    let points = if straight_dry_land(sampler, x0, y0, x1, y1) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = (dx * dx + dy * dy).sqrt().max(1.0);
        let n = ((dist / 10.0).round() as usize).clamp(5, 22);
        let len = dist.max(1e-3);
        let px = -dy / len;
        let py = dx / len;
        let mut pts = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let mut x = x0 + dx * t;
            let mut y = y0 + dy * t;
            if i > 0 && i < n {
                let w = (unit_noise(seed, CHANNEL_ROAD, index * 31 + i as u64) as f32 - 0.5)
                    * dist
                    * wobble;
                let nx = x + px * w;
                let ny = y + py * w;
                if is_dry_land(sampler, nx, ny) {
                    x = nx;
                    y = ny;
                }
            }
            pts.push(road_pt(x, y));
        }
        pts
    } else {
        land_path(sampler, seed, kind, x0, y0, x1, y1, world_w, world_h, bridges)
    };
    if points.len() < 2 {
        return points;
    }
    // Wobbled straight paths must stay dry; otherwise repath with bridges.
    let points = if points
        .iter()
        .any(|p| p.crossing.is_none() && !is_dry_land(sampler, p.x, p.y))
    {
        land_path(sampler, seed, kind, x0, y0, x1, y1, world_w, world_h, bridges)
    } else {
        points
    };
    finalize_bridge_polyline(sampler, points, bridges)
}


fn river_mid_on_chord(
    sampler: &TerrainSampler,
    a: (f32, f32),
    b: (f32, f32),
) -> (f32, f32) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let best = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
    for k in 1..12 {
        let t = k as f32 / 12.0;
        let x = a.0 + dx * t;
        let y = a.1 + dy * t;
        if is_river_water(sampler, x, y) {
            return (x, y);
        }
    }
    best
}

fn orient_side(a: (f32, f32), b: (f32, f32), p: (f32, f32)) -> f32 {
    (b.1 - a.1) * (p.0 - a.0) - (b.0 - a.0) * (p.1 - a.1)
}

/// Route `from→to` using span portals. Opposite banks cross mid-river once; same bank skirts a portal.
fn append_via_span(
    out: &mut Vec<RoadPoint>,
    sampler: &TerrainSampler,
    from: (f32, f32),
    to: (f32, f32),
    span: BridgeSpan,
) {
    let sa = span.a;
    let sb = span.b;
    let side_from = orient_side(sa, sb, from);
    let side_to = orient_side(sa, sb, to);
    let opposite = side_from * side_to < 0.0
        || shore_water_only(sampler, from.0, from.1, to.0, to.1);

    let (enter, leave) = if dist2(sa.0, sa.1, from.0, from.1) <= dist2(sb.0, sb.1, from.0, from.1)
    {
        (sa, sb)
    } else {
        (sb, sa)
    };

    push_road_pt_dedup(out, road_pt(from.0, from.1));
    push_road_pt_dedup(out, road_pt(enter.0, enter.1));
    if opposite {
        let mid = span.mid();
        let mid = if is_river_water(sampler, mid.0, mid.1) {
            mid
        } else {
            river_mid_on_chord(sampler, enter, leave)
        };
        push_road_pt_dedup(
            out,
            RoadPoint {
                x: mid.0,
                y: mid.1,
                crossing: Some(RoadCrossing::Bridge),
            },
        );
        push_road_pt_dedup(out, road_pt(leave.0, leave.1));
    }
    // Same bank: only touch the nearer portal (no mid-river junction / no cutting the deck).
    push_road_pt_dedup(out, road_pt(to.0, to.1));
}

/// Enforce shore→bridge→shore only; insert bridges on river chords; drop water stubs.
fn finalize_bridge_polyline(
    sampler: &TerrainSampler,
    points: Vec<RoadPoint>,
    bridges: &BridgeRegistry,
) -> Vec<RoadPoint> {
    if points.len() < 2 {
        return Vec::new();
    }

    // 1) Keep only dry points and valid bridge mids (strip water stubs / orphan mids).
    let mut cleaned: Vec<RoadPoint> = Vec::new();
    let mut i = 0usize;
    while i < points.len() {
        let p = &points[i];
        if p.crossing == Some(RoadCrossing::Bridge) {
            i += 1;
            continue; // rebuild bridges from shore pairs below
        }
        if is_dry_land(sampler, p.x, p.y) {
            push_road_pt_dedup(&mut cleaned, road_pt(p.x, p.y));
        }
        i += 1;
    }
    if cleaned.len() < 2 {
        return Vec::new();
    }

    // 2) Between consecutive dry points: land, or one shore→shore bridge; never cut a span.
    let mut out: Vec<RoadPoint> = Vec::with_capacity(cleaned.len() * 2);
    out.push(cleaned[0].clone());
    for w in cleaned.windows(2) {
        let a = &w[0];
        let b = &w[1];
        let ap = (a.x, a.y);
        let bp = (b.x, b.y);

        // Any chord that cuts an existing bridge must use that bridge (or skirt a portal).
        if let Some(s) = bridges.span_cut_by(ap, bp) {
            append_via_span(&mut out, sampler, ap, bp, s);
            continue;
        }

        if straight_dry_land(sampler, a.x, a.y, b.x, b.y) {
            push_road_pt_dedup(&mut out, road_pt(b.x, b.y));
            continue;
        }
        if !shore_water_only(sampler, a.x, a.y, b.x, b.y) {
            return Vec::new();
        }
        let candidate = BridgeSpan { a: ap, b: bp };
        let span = bridges.resolve_span(candidate);
        if let Some(s) = span {
            append_via_span(&mut out, sampler, ap, bp, s);
            continue;
        }
        if bridges.conflicts_new(candidate) {
            return Vec::new();
        }
        let mid = river_mid_on_chord(sampler, ap, bp);
        push_road_pt_dedup(&mut out, road_pt(ap.0, ap.1));
        push_road_pt_dedup(
            &mut out,
            RoadPoint {
                x: mid.0,
                y: mid.1,
                crossing: Some(RoadCrossing::Bridge),
            },
        );
        push_road_pt_dedup(&mut out, road_pt(bp.0, bp.1));
    }

    if out.len() < 2 {
        return Vec::new();
    }
    if out.first().is_some_and(|p| p.crossing.is_some())
        || out.last().is_some_and(|p| p.crossing.is_some())
    {
        return Vec::new();
    }
    out
}

fn shore_water_only(sampler: &TerrainSampler, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist > BRIDGE_MAX_LEN * 1.35 {
        return false;
    }
    let n = ((dist / 1.5).ceil() as usize).clamp(4, 24);
    let mut water = 0usize;
    for i in 1..n {
        let t = i as f32 / n as f32;
        let x = x0 + dx * t;
        let y = y0 + dy * t;
        if is_dry_land(sampler, x, y) {
            continue;
        }
        if !is_river_water(sampler, x, y) {
            return false;
        }
        water += 1;
    }
    water > 0
}


fn push_road_pt_dedup(out: &mut Vec<RoadPoint>, p: RoadPoint) {
    if let Some(last) = out.last() {
        let dx = last.x - p.x;
        let dy = last.y - p.y;
        if dx * dx + dy * dy < 0.35 * 0.35 {
            if p.crossing.is_some() && last.crossing.is_none() {
                out.pop();
                out.push(p);
            }
            return;
        }
    }
    out.push(p);
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
    use std::collections::HashSet;
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
        assert_eq!(a.settlements.len(), b.settlements.len());
        assert!(!a.settlements.is_empty());
        assert_eq!(a.roads.len(), b.roads.len());
        assert!(!a.roads.is_empty());
        for (left, right) in a.settlements.iter().zip(b.settlements.iter()) {
            assert_eq!(left.kind, right.kind);
            assert_eq!(left.population, right.population);
            assert_eq!(left.name, right.name);
            assert!(!left.name.is_empty());
            assert!((left.x - right.x).abs() < 1e-5);
            assert!((left.y - right.y).abs() < 1e-5);
            assert!((left.core_dx - right.core_dx).abs() < 1e-5);
            assert!((left.core_dy - right.core_dy).abs() < 1e-5);
            assert_eq!(left.road_approaches.len(), right.road_approaches.len());
            assert_eq!(left.streets.len(), right.streets.len());
            assert_eq!(left.districts.len(), right.districts.len());
            for (ld, rd) in left.districts.iter().zip(right.districts.iter()) {
                assert_eq!(ld.kind, rd.kind);
                assert_eq!(ld.name, rd.name);
                assert!((ld.inner - rd.inner).abs() < 1e-5);
                assert!((ld.a0 - rd.a0).abs() < 1e-5);
            }
        }
        for (left, right) in a.roads.iter().zip(b.roads.iter()) {
            assert_eq!(left.kind, right.kind);
            assert_eq!(left.surface, right.surface);
            assert_eq!(left.points.len(), right.points.len());
            assert!((left.points[0].x - right.points[0].x).abs() < 1e-5);
        }
        let names: HashSet<&str> = a.settlements.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names.len(), a.settlements.len());
        let cities: Vec<&Settlement> = a
            .settlements
            .iter()
            .filter(|s| s.kind == SettlementKind::City)
            .collect();
        let highways = a
            .roads
            .iter()
            .filter(|r| r.kind == RoadKind::Highway)
            .count();
        assert!(
            highways + 1 >= cities.len().min(2),
            "cities should be linked by highways"
        );
        if cities.len() >= 2 {
            for s in &cities {
                assert!(
                    s.road_approaches.iter().any(|p| p.kind == RoadKind::Highway),
                    "city {} should attach to a highway",
                    s.name
                );
            }
        }
        let villages: Vec<&Settlement> = a
            .settlements
            .iter()
            .filter(|s| s.kind == SettlementKind::Village)
            .collect();
        if villages.len() >= 3 {
            let linked = villages
                .iter()
                .filter(|s| !s.road_approaches.is_empty())
                .count();
            assert!(
                linked < villages.len(),
                "not every village should be on the road net"
            );
        }
        assert!(
            a.roads.iter().all(|r| r.points.len() >= 2),
            "roads need a polyline"
        );
    }

    fn sample_road_on_land(sampler: &TerrainSampler, road: &Road) {
        for i in 0..road.points.len() {
            let p = &road.points[i];
            let cell = sampler.cell_at(p.x as f64, p.y as f64, LodLevel::Macro);
            let river = sampler.river_at(p.x as f64, p.y as f64);
            if cell.elevation < WATER_M {
                assert_eq!(
                    p.crossing,
                    Some(RoadCrossing::Bridge),
                    "water road vertex must be a bridge at ({}, {})",
                    p.x,
                    p.y
                );
                assert!(
                    river.channel > 0.55,
                    "bridge must sit on a river channel at ({}, {})",
                    p.x,
                    p.y
                );
            } else {
                assert!(p.crossing.is_none());
            }
            if i + 1 >= road.points.len() {
                continue;
            }
            let q = &road.points[i + 1];
            let dx = q.x - p.x;
            let dy = q.y - p.y;
            let dist = (dx * dx + dy * dy).sqrt();
            let n = ((dist / 3.0).ceil() as i32).max(1);
            for s in 1..n {
                let t = s as f32 / n as f32;
                let x = p.x + dx * t;
                let y = p.y + dy * t;
                let c = sampler.cell_at(x as f64, y as f64, LodLevel::Macro);
                let river = sampler.river_at(x as f64, y as f64);
                if c.elevation < WATER_M {
                    assert!(
                        river.channel > 0.55
                            && (p.crossing == Some(RoadCrossing::Bridge)
                                || q.crossing == Some(RoadCrossing::Bridge)),
                        "river crossing must be a marked bridge span at ({x}, {y})"
                    );
                }
            }
        }
    }

    #[test]
    fn seed_6_roads_stay_on_land_or_bridges() {
        let config = seed_config(6);
        let sampler = TerrainSampler::new(config.clone());
        let map = generate(config);
        assert!(!map.roads.is_empty());
        for road in &map.roads {
            sample_road_on_land(&sampler, road);
        }
    }

    #[test]
    fn seed_6_has_river_bridges() {
        let config = seed_config(6);
        let map = generate(config);
        let bridges = map
            .roads
            .iter()
            .flat_map(|r| r.points.iter())
            .filter(|p| p.crossing == Some(RoadCrossing::Bridge))
            .count();
        assert!(
            bridges >= 1,
            "expected at least one river bridge on seed 6, got {bridges}"
        );
    }

    #[test]
    fn seed_6_bridges_are_shore_to_shore() {
        let config = seed_config(6);
        let sampler = TerrainSampler::new(config.clone());
        let map = generate(config);
        let mut bridge_n = 0usize;
        for road in &map.roads {
            let pts = &road.points;
            assert!(
                is_dry_land(&sampler, pts[0].x, pts[0].y),
                "road must start on shore/land"
            );
            assert!(
                is_dry_land(&sampler, pts[pts.len() - 1].x, pts[pts.len() - 1].y),
                "road must end on shore/land"
            );
            assert!(pts[0].crossing.is_none() && pts[pts.len() - 1].crossing.is_none());
            for i in 0..pts.len() {
                let p = &pts[i];
                if p.crossing == Some(RoadCrossing::Bridge) {
                    bridge_n += 1;
                    assert!(i > 0 && i + 1 < pts.len(), "bridge cannot be an endpoint");
                    let prev = &pts[i - 1];
                    let next = &pts[i + 1];
                    assert!(prev.crossing.is_none() && next.crossing.is_none());
                    assert!(
                        is_dry_land(&sampler, prev.x, prev.y),
                        "bridge prev must be dry"
                    );
                    assert!(
                        is_dry_land(&sampler, next.x, next.y),
                        "bridge next must be dry"
                    );
                    assert!(
                        is_river_water(&sampler, p.x, p.y),
                        "bridge mid must sit on river"
                    );
                    assert!(
                        shore_water_only(&sampler, prev.x, prev.y, next.x, next.y),
                        "bridge must span shore→shore over river only"
                    );
                } else {
                    assert!(
                        is_dry_land(&sampler, p.x, p.y),
                        "non-bridge vertex in water at ({}, {})",
                        p.x,
                        p.y
                    );
                }
            }
        }
        assert!(bridge_n >= 1, "expected bridges on seed 6");

        // Collect unique shore→shore spans; none may cross.
        let mut spans: Vec<BridgeSpan> = Vec::new();
        for road in &map.roads {
            let pts = &road.points;
            for i in 1..pts.len().saturating_sub(1) {
                if pts[i].crossing != Some(RoadCrossing::Bridge) {
                    continue;
                }
                let span = BridgeSpan {
                    a: (pts[i - 1].x, pts[i - 1].y),
                    b: (pts[i + 1].x, pts[i + 1].y),
                };
                if spans.iter().any(|s| s.same_as(span.a, span.b)) {
                    continue;
                }
                spans.push(span);
            }
        }
        // No road chord (except a span itself) may cut a bridge.
        for road in &map.roads {
            let pts = &road.points;
            for w in pts.windows(2) {
                if w[0].crossing.is_some() || w[1].crossing.is_some() {
                    continue; // shore↔mid legs are the span
                }
                let chord = ( (w[0].x, w[0].y), (w[1].x, w[1].y) );
                for s in &spans {
                    if s.same_as(chord.0, chord.1) {
                        continue;
                    }
                    assert!(
                        !s.crosses(BridgeSpan {
                            a: chord.0,
                            b: chord.1,
                        }),
                        "road cuts bridge near {:?}",
                        s.mid()
                    );
                }
            }
        }

        for i in 0..spans.len() {
            for j in i + 1..spans.len() {
                assert!(
                    !spans[i].crosses(spans[j]),
                    "bridges must not cross: {:?} vs {:?}",
                    spans[i].mid(),
                    spans[j].mid()
                );
                let d = dist2(
                    spans[i].mid().0,
                    spans[i].mid().1,
                    spans[j].mid().0,
                    spans[j].mid().1,
                )
                .sqrt();
                assert!(
                    d + 1e-3 >= BRIDGE_CATCHMENT_R,
                    "parallel bridges too close ({d:.1})"
                );
            }
        }
    }


    fn has_kind(districts: &[District], kind: DistrictKind) -> bool {
        districts.iter().any(|d| d.kind == kind)
    }

    #[test]
    fn seed_6_has_all_kinds_on_land() {
        let config = seed_config(6);
        let sampler = TerrainSampler::new(config.clone());
        let settlements = generate(config).settlements;
        let mut saw = [false; 4];
        for s in &settlements {
            let cell = sampler.cell_at(s.x as f64, s.y as f64, LodLevel::Macro);
            assert!(cell.elevation >= WATER_M, "settlement in water at ({}, {})", s.x, s.y);
            match s.kind {
                SettlementKind::City => {
                    saw[0] = true;
                    assert!((8_000..=50_000).contains(&s.population));
                    assert!(has_kind(&s.districts, DistrictKind::Center));
                    assert!(has_kind(&s.districts, DistrictKind::Market));
                    assert!(has_kind(&s.districts, DistrictKind::Craft));
                    assert!(has_kind(&s.districts, DistrictKind::Residential));
                    assert!(has_kind(&s.districts, DistrictKind::Outskirts));
                    assert!(s.streets.iter().any(|st| st.radial));
                    assert!(s.streets.iter().any(|st| !st.radial));
                }
                SettlementKind::Town => {
                    saw[1] = true;
                    assert!((1_200..=8_000).contains(&s.population));
                    assert!(has_kind(&s.districts, DistrictKind::Center));
                    assert!(has_kind(&s.districts, DistrictKind::Market));
                    assert!(has_kind(&s.districts, DistrictKind::Craft));
                    assert!(has_kind(&s.districts, DistrictKind::Residential));
                    assert!(has_kind(&s.districts, DistrictKind::Outskirts));
                    assert!(s.streets.iter().any(|st| st.radial));
                }
                SettlementKind::Village => {
                    saw[2] = true;
                    assert!((150..=1_200).contains(&s.population));
                    if s.population >= VILLAGE_DISTRICT_POP_MIN {
                        assert!(!s.districts.is_empty());
                        assert!(!s.streets.is_empty());
                    } else {
                        assert!(s.districts.is_empty());
                        assert!(s.streets.is_empty());
                    }
                }
                SettlementKind::Hamlet => {
                    saw[3] = true;
                    assert!((20..=150).contains(&s.population));
                    assert!(s.districts.is_empty());
                    assert!(s.streets.is_empty());
                }
            }
        }
        assert!(saw.iter().all(|v| *v), "missing settlement kinds: {saw:?}");
    }

    #[test]
    fn layout_can_offset_center_and_add_forest() {
        let (_, _, with_forest) =
            layout_districts(6, 3, SettlementKind::City, None, Some(0.0));
        assert!(has_kind(&with_forest, DistrictKind::Forest));
        assert!(has_kind(&with_forest, DistrictKind::Center));
        let forest_n = with_forest
            .iter()
            .filter(|d| d.kind == DistrictKind::Forest)
            .count();
        assert!(forest_n >= 2, "expected multiple forest districts, got {forest_n}");

        let (_, _, with_port) =
            layout_districts(6, 3, SettlementKind::City, Some(1.2), None);
        assert!(has_kind(&with_port, DistrictKind::Port));
        assert!(has_kind(&with_port, DistrictKind::Residential));
        assert!(has_kind(&with_port, DistrictKind::Noble));
        assert!(!has_kind(&with_port, DistrictKind::Forest));
        let port = with_port
            .iter()
            .find(|d| d.kind == DistrictKind::Port)
            .expect("port district");
        assert!(
            port.inner >= 0.55,
            "port should sit on the outer ring, got inner={}",
            port.inner
        );

        let (_, _, bare) = layout_districts(6, 3, SettlementKind::City, None, None);
        assert!(!has_kind(&bare, DistrictKind::Port));
        assert!(!has_kind(&bare, DistrictKind::Forest));

        let mut saw_offset = false;
        let mut saw_centered = false;
        for index in 0..24u64 {
            let (dx, dy, _) = layout_districts(6, index, SettlementKind::City, None, None);
            if dx.abs() + dy.abs() > 0.05 {
                saw_offset = true;
            } else {
                saw_centered = true;
            }
        }
        assert!(saw_offset, "expected some civic cores off-center");
        assert!(saw_centered, "expected some civic cores in the middle");
    }

    #[test]
    fn street_layout_port_outer_forest_requires_biome() {
        let approaches = vec![RoadApproach {
            angle: 0.0,
            kind: RoadKind::Highway,
        }];
        let (_, _, with_port, _) = layout_on_streets(
            6,
            3,
            SettlementKind::City,
            &approaches,
            Some(1.2),
            None,
        );
        assert!(has_kind(&with_port, DistrictKind::Port));
        assert!(!has_kind(&with_port, DistrictKind::Forest));
        let port = with_port
            .iter()
            .find(|d| d.kind == DistrictKind::Port)
            .expect("port");
        assert!(port.inner >= 0.5, "port on outer ring, inner={}", port.inner);

        let (_, _, with_forest, _) = layout_on_streets(
            6,
            3,
            SettlementKind::City,
            &approaches,
            None,
            Some(0.0),
        );
        assert!(has_kind(&with_forest, DistrictKind::Forest));
        assert!(!has_kind(&with_forest, DistrictKind::Port));

        let (_, _, bare, _) =
            layout_on_streets(6, 3, SettlementKind::City, &approaches, None, None);
        assert!(!has_kind(&bare, DistrictKind::Port));
        assert!(!has_kind(&bare, DistrictKind::Forest));
    }
}
