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
        &sampler,
        world_w as f32,
        world_h as f32,
        &mut out,
    );
    refine_street_layouts(seed, &sampler, &mut out);
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
        );
        if points.len() < 2 {
            continue;
        }
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

fn is_dry_land(sampler: &TerrainSampler, x: f32, y: f32) -> bool {
    !is_water_biome(sampler.cell_at(x as f64, y as f64, LodLevel::Macro).elevation)
}

/// Whether this cell may host a road: dry land, or a deterministic river bridge.
fn road_cell_pass(
    sampler: &TerrainSampler,
    x: f32,
    y: f32,
    kind: RoadKind,
    seed: u64,
) -> Option<(u32, Option<RoadCrossing>)> {
    let cell = sampler.cell_at(x as f64, y as f64, LodLevel::Macro);
    let river = sampler.river_at(x as f64, y as f64);
    let elev = cell.elevation;

    if !is_water_biome(elev) {
        let cost = if elev >= HIGH_M {
            16
        } else if elev >= UPLAND_M {
            12
        } else {
            10
        };
        return Some((cost, None));
    }

    // Ocean / deep lake — no road.
    if river.channel <= 0.55 {
        return None;
    }

    if !river_bridge_allowed(kind, seed, x, y) {
        return None;
    }
    Some((48, Some(RoadCrossing::Bridge)))
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

/// Sample the step between road cells — thin rivers sit between 6u cell centers.
fn step_river_crossing(
    sampler: &TerrainSampler,
    kind: RoadKind,
    seed: u64,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> Option<(u32, Option<(f32, f32)>)> {
    let mut extra = 0u32;
    let mut bridge_at: Option<(f32, f32)> = None;
    for k in 1..8 {
        let t = k as f32 / 8.0;
        let x = x0 + (x1 - x0) * t;
        let y = y0 + (y1 - y0) * t;
        let cell = sampler.cell_at(x as f64, y as f64, LodLevel::Macro);
        let river = sampler.river_at(x as f64, y as f64);
        if !is_water_biome(cell.elevation) {
            continue;
        }
        if river.channel <= 0.55 {
            return None; // ocean / lake span
        }
        if !river_bridge_allowed(kind, seed, x, y) {
            return None; // no passage at this crossing
        }
        if bridge_at.is_none() {
            // Modest toll — short bridge beats a long dry detour.
            extra = 18;
            bridge_at = Some((x, y));
        }
    }
    Some((extra, bridge_at))
}

fn cell_allows_road(
    sampler: &TerrainSampler,
    x: f32,
    y: f32,
    kind: RoadKind,
    seed: u64,
) -> bool {
    road_cell_pass(sampler, x, y, kind, seed).is_some()
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
    let mut prev_bridge: Vec<Option<(f32, f32)>> = vec![None; n];
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
                if !cell_allows_road(sampler, ax, ay, kind, seed)
                    || !cell_allows_road(sampler, bx, by, kind, seed)
                {
                    continue;
                }
            }
            let wx = (nx as f32 + 0.5) * ROAD_CELL;
            let wy = (ny as f32 + 0.5) * ROAD_CELL;
            let Some((terrain, _)) = road_cell_pass(sampler, wx, wy, kind, seed) else {
                continue;
            };
            let Some((cross_extra, bridge_at)) =
                step_river_crossing(sampler, kind, seed, cx, cy, wx, wy)
            else {
                continue;
            };
            let next = cost
                .saturating_add(step * terrain / 10)
                .saturating_add(cross_extra);
            let ni = idx(nx, ny);
            if next < dist[ni] {
                dist[ni] = next;
                prev[ni] = idx(x, y) as i32;
                prev_bridge[ni] = bridge_at;
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
    let mut bridges = Vec::new();
    let mut cur = idx(goal.0, goal.1) as i32;
    while cur >= 0 {
        let i = cur as usize;
        let cx = (i as i32) % cols;
        let cy = (i as i32) / cols;
        cells.push((cx, cy));
        bridges.push(prev_bridge[i]);
        cur = prev[i];
        if (cx, cy) == start {
            break;
        }
    }
    cells.reverse();
    bridges.reverse();
    if cells.is_empty() {
        return Vec::new();
    }

    let mut points = Vec::with_capacity(cells.len() + bridges.len() + 2);
    points.push(road_pt(x0, y0));
    for (i, (cx, cy)) in cells.iter().enumerate() {
        if let Some((bx, by)) = bridges[i] {
            points.push(RoadPoint {
                x: bx,
                y: by,
                crossing: Some(RoadCrossing::Bridge),
            });
        }
        let wx = (*cx as f32 + 0.5) * ROAD_CELL;
        let wy = (*cy as f32 + 0.5) * ROAD_CELL;
        let crossing = road_cell_pass(sampler, wx, wy, kind, seed).and_then(|(_, c)| c);
        points.push(RoadPoint {
            x: wx,
            y: wy,
            crossing,
        });
    }
    points.push(road_pt(x1, y1));
    simplify_road(points)
}

fn simplify_road(points: Vec<RoadPoint>) -> Vec<RoadPoint> {
    if points.len() <= 3 {
        return points;
    }
    let mut out = vec![points[0].clone()];
    for i in 1..points.len() - 1 {
        let a = out.last().unwrap();
        let b = &points[i];
        let c = &points[i + 1];
        let abx = b.x - a.x;
        let aby = b.y - a.y;
        let bcx = c.x - b.x;
        let bcy = c.y - b.y;
        let cross = (abx * bcy - aby * bcx).abs();
        let keep = cross > 4.0 || b.crossing.is_some();
        if keep {
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
        land_path(sampler, seed, kind, x0, y0, x1, y1, world_w, world_h)
    };
    if points.len() < 2 {
        return points;
    }
    let points = if points
        .iter()
        .skip(1)
        .rev()
        .skip(1)
        .any(|p| !cell_allows_road(sampler, p.x, p.y, kind, seed))
    {
        land_path(sampler, seed, kind, x0, y0, x1, y1, world_w, world_h)
    } else {
        points
    };
    clamp_road_to_land_or_bridge(sampler, kind, seed, points)
}

/// Densify, then keep dry ground; river spans become a single Bridge or the road ends at shore.
fn clamp_road_to_land_or_bridge(
    sampler: &TerrainSampler,
    kind: RoadKind,
    seed: u64,
    points: Vec<RoadPoint>,
) -> Vec<RoadPoint> {
    if points.len() < 2 {
        return points;
    }
    let mut dense: Vec<(f32, f32)> = Vec::new();
    for w in points.windows(2) {
        let (ax, ay) = (w[0].x, w[0].y);
        let (bx, by) = (w[1].x, w[1].y);
        let dist = ((bx - ax) * (bx - ax) + (by - ay) * (by - ay)).sqrt();
        let steps = ((dist / 1.25).ceil() as usize).clamp(1, 64);
        for s in 0..steps {
            let t = s as f32 / steps as f32;
            dense.push((ax + (bx - ax) * t, ay + (by - ay) * t));
        }
    }
    if let Some(last) = points.last() {
        dense.push((last.x, last.y));
    }

    let mut out: Vec<RoadPoint> = Vec::new();
    let mut i = 0usize;
    while i < dense.len() {
        let (x, y) = dense[i];
        let elev = sampler
            .cell_at(x as f64, y as f64, LodLevel::Macro)
            .elevation;
        let river = sampler.river_at(x as f64, y as f64);

        if !is_water_biome(elev) {
            push_road_pt_dedup(&mut out, road_pt(x, y));
            i += 1;
            continue;
        }

        // Water: try to span a river with a bridge to the next dry sample.
        let river_ok = river.channel > 0.55 && river_bridge_allowed(kind, seed, x, y);
        if river_ok {
            let mut j = i;
            while j < dense.len() {
                let (sx, sy) = dense[j];
                let e = sampler
                    .cell_at(sx as f64, sy as f64, LodLevel::Macro)
                    .elevation;
                let r = sampler.river_at(sx as f64, sy as f64);
                if !is_water_biome(e) {
                    break;
                }
                if r.channel <= 0.55 || !river_bridge_allowed(kind, seed, sx, sy) {
                    // Hits ocean / blocked river — stop at last dry.
                    return out;
                }
                j += 1;
            }
            if j < dense.len() {
                let mid = dense[i + (j - i) / 2];
                push_road_pt_dedup(
                    &mut out,
                    RoadPoint {
                        x: mid.0,
                        y: mid.1,
                        crossing: Some(RoadCrossing::Bridge),
                    },
                );
                i = j;
                continue;
            }
        }

        // Dead-end at shore (or no passage).
        break;
    }

    if out.len() < 2 {
        Vec::new()
    } else {
        out
    }
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
                    // Mid-segment may skim a bridge span between marked vertices.
                    assert!(
                        river.channel > 0.55
                            || p.crossing == Some(RoadCrossing::Bridge)
                            || q.crossing == Some(RoadCrossing::Bridge),
                        "road segment in non-bridge water at ({x}, {y})"
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
