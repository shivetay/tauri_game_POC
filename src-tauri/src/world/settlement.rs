use std::collections::HashSet;

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
const CHANNEL_CORE: u64 = 46;
const SCORE_FLOOR: f32 = 0.18;
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
            let (water_dir, forest_dir) = neighbor_dirs(&sampler, c.x, c.y, radius);
            let (core_dx, core_dy, districts) =
                layout_districts(seed, c.index, tier.kind, water_dir, forest_dir);
            out.push(Settlement {
                x: c.x,
                y: c.y,
                kind: tier.kind,
                population: lerp_u32(tier.pop_min, tier.pop_max, t),
                radius,
                name: unique_name(seed, c.index, tier.kind, &mut taken_names),
                core_dx,
                core_dy,
                districts,
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
    if let Some(dir) = water_dir {
        let i = nearest_slot(&inner_a0, &inner_span, dir);
        convert_slot(&mut inner_kind, i, DistrictKind::Port, min_housing);
    }
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
    let want_inner_forest = forest_dir.is_some()
        || unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(9))
            < if kind == SettlementKind::City {
                0.72
            } else {
                0.42
            };
    if want_inner_forest {
        if let Some(i) = first_residential(&inner_kind) {
            convert_slot(&mut inner_kind, i, DistrictKind::Forest, min_housing);
        }
    }

    let n_outer = if kind == SettlementKind::City { 4 } else { 3 };
    let outer_rot = rot + 0.17;
    let (outer_a0, outer_span) = uneven_slices(seed, index, 80, n_outer, outer_rot);
    let mut outer_kind = vec![DistrictKind::Outskirts; n_outer];
    let forest_n = if forest_dir.is_some() {
        if kind == SettlementKind::City {
            2
        } else {
            1
        }
    } else if unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(6))
        < if kind == SettlementKind::City {
            0.78
        } else {
            0.55
        }
    {
        1
    } else {
        0
    };
    let forest_dir_or = forest_dir.unwrap_or_else(|| {
        unit_noise(seed, CHANNEL_LAYOUT, index.wrapping_add(7)) as f32 * TAU
    });
    place_nearest(
        &mut outer_kind,
        &outer_a0,
        &outer_span,
        forest_dir_or,
        DistrictKind::Forest,
        forest_n,
    );
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
        assert_eq!(a.len(), b.len());
        assert!(!a.is_empty());
        for (left, right) in a.iter().zip(b.iter()) {
            assert_eq!(left.kind, right.kind);
            assert_eq!(left.population, right.population);
            assert_eq!(left.name, right.name);
            assert!(!left.name.is_empty());
            assert!((left.x - right.x).abs() < 1e-5);
            assert!((left.y - right.y).abs() < 1e-5);
            assert!((left.core_dx - right.core_dx).abs() < 1e-5);
            assert!((left.core_dy - right.core_dy).abs() < 1e-5);
            assert_eq!(left.districts.len(), right.districts.len());
            for (ld, rd) in left.districts.iter().zip(right.districts.iter()) {
                assert_eq!(ld.kind, rd.kind);
                assert_eq!(ld.name, rd.name);
                assert!((ld.inner - rd.inner).abs() < 1e-5);
                assert!((ld.a0 - rd.a0).abs() < 1e-5);
            }
        }
        let names: HashSet<&str> = a.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names.len(), a.len());
    }

    fn has_kind(districts: &[District], kind: DistrictKind) -> bool {
        districts.iter().any(|d| d.kind == kind)
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
                    assert!(has_kind(&s.districts, DistrictKind::Center));
                    assert!(has_kind(&s.districts, DistrictKind::Market));
                    assert!(has_kind(&s.districts, DistrictKind::Craft));
                    assert!(has_kind(&s.districts, DistrictKind::Residential));
                    assert!(has_kind(&s.districts, DistrictKind::Outskirts));
                }
                SettlementKind::Town => {
                    saw[1] = true;
                    assert!((1_200..=8_000).contains(&s.population));
                    assert!(has_kind(&s.districts, DistrictKind::Center));
                    assert!(has_kind(&s.districts, DistrictKind::Market));
                    assert!(has_kind(&s.districts, DistrictKind::Craft));
                    assert!(has_kind(&s.districts, DistrictKind::Residential));
                    assert!(has_kind(&s.districts, DistrictKind::Outskirts));
                }
                SettlementKind::Village => {
                    saw[2] = true;
                    assert!((150..=1_200).contains(&s.population));
                    assert!(s.districts.is_empty());
                }
                SettlementKind::Hamlet => {
                    saw[3] = true;
                    assert!((20..=150).contains(&s.population));
                    assert!(s.districts.is_empty());
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
}
