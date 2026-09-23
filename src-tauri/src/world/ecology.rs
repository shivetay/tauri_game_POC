use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::world::config::{LodLevel, WorldConfig};
use crate::world::elevation::is_water_biome;
use crate::world::prng::unit_noise;
use crate::world::sampler::TerrainSampler;
use crate::world::types::{ChunkId, RegionBounds, RegionId, TileType};

const SPECIES_PER_BIOME: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FaunaGroup {
    LargeMammal,
    Bird,
    SmallFauna,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LifeKind {
    Flora,
    Fauna,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HabitatCell {
    /// Overall life potential for the biome (region wash — layer A).
    pub potential: f32,
    /// Large mammal habitat strength (layer B).
    pub large_mammals: f32,
    /// Bird habitat strength (layer B).
    pub birds: f32,
    /// Small fauna habitat strength (layer B).
    pub small_fauna: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionTileExpect {
    /// General flora one can expect on this region tile (chunk cell).
    pub flora: Vec<String>,
    /// General fauna one can expect on this region tile.
    pub fauna: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionEcologyMap {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<HabitatCell>,
    /// Chunk tiles across the region (usually region_size / chunk_size).
    pub tiles_across: u32,
    pub tiles: Vec<RegionTileExpect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifeInstance {
    pub x: f32,
    pub y: f32,
    pub kind: LifeKind,
    pub group: Option<FaunaGroup>,
    pub species: String,
    pub subspecies: String,
    pub biome: TileType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkEcologyMap {
    /// Vegetation / habitat wash — same color language as region LOD.
    pub width: u32,
    pub height: u32,
    pub cells: Vec<HabitatCell>,
    /// Species present in the chunk (for area hover detail).
    pub flora: Vec<LifeInstance>,
    pub fauna: Vec<LifeInstance>,
}

struct FloraDef {
    name: &'static str,
    subspecies: [&'static str; 2],
}

struct FaunaDef {
    name: &'static str,
    subspecies: [&'static str; 2],
    group: FaunaGroup,
}

fn flora_for(biome: TileType) -> &'static [FloraDef; SPECIES_PER_BIOME] {
    use TileType::*;
    match biome {
        DeepWater => &DEEP_WATER_FLORA,
        Water => &WATER_FLORA,
        Sand => &SAND_FLORA,
        RockyShore => &ROCKY_SHORE_FLORA,
        Desert => &DESERT_FLORA,
        Savanna => &SAVANNA_FLORA,
        Grass => &GRASS_FLORA,
        Swamp => &SWAMP_FLORA,
        Shrubland => &SHRUBLAND_FLORA,
        Forest => &FOREST_FLORA,
        Rainforest => &RAINFOREST_FLORA,
        Mountain => &MOUNTAIN_FLORA,
        Tundra => &TUNDRA_FLORA,
        Snow => &SNOW_FLORA,
    }
}

fn fauna_for(biome: TileType) -> &'static [FaunaDef; SPECIES_PER_BIOME] {
    use TileType::*;
    match biome {
        DeepWater => &DEEP_WATER_FAUNA,
        Water => &WATER_FAUNA,
        Sand => &SAND_FAUNA,
        RockyShore => &ROCKY_SHORE_FAUNA,
        Desert => &DESERT_FAUNA,
        Savanna => &SAVANNA_FAUNA,
        Grass => &GRASS_FAUNA,
        Swamp => &SWAMP_FAUNA,
        Shrubland => &SHRUBLAND_FAUNA,
        Forest => &FOREST_FAUNA,
        Rainforest => &RAINFOREST_FAUNA,
        Mountain => &MOUNTAIN_FAUNA,
        Tundra => &TUNDRA_FAUNA,
        Snow => &SNOW_FAUNA,
    }
}

fn biome_life_base(biome: TileType) -> f32 {
    match biome {
        TileType::DeepWater => 0.35,
        TileType::Water => 0.55,
        TileType::Sand => 0.40,
        TileType::RockyShore => 0.45,
        TileType::Desert => 0.30,
        TileType::Savanna => 0.75,
        TileType::Grass => 0.80,
        TileType::Swamp => 0.70,
        TileType::Shrubland => 0.60,
        TileType::Forest => 0.85,
        TileType::Rainforest => 0.95,
        TileType::Mountain => 0.40,
        TileType::Tundra => 0.35,
        TileType::Snow => 0.20,
    }
}

fn group_bias(biome: TileType, group: FaunaGroup) -> f32 {
    use FaunaGroup::*;
    use TileType::*;
    match (biome, group) {
        (DeepWater | Water, Bird) => 0.55,
        (DeepWater | Water, SmallFauna) => 0.90,
        (DeepWater | Water, LargeMammal) => 0.45,
        (Desert | Sand | RockyShore, SmallFauna) => 0.85,
        (Desert | Sand | RockyShore, Bird) => 0.60,
        (Desert | Sand | RockyShore, LargeMammal) => 0.40,
        (Savanna | Grass, LargeMammal) => 0.90,
        (Savanna | Grass, Bird) => 0.70,
        (Savanna | Grass, SmallFauna) => 0.65,
        (Forest | Rainforest | Shrubland, LargeMammal) => 0.80,
        (Forest | Rainforest | Shrubland, Bird) => 0.85,
        (Forest | Rainforest | Shrubland, SmallFauna) => 0.75,
        (Swamp, Bird) => 0.85,
        (Swamp, SmallFauna) => 0.80,
        (Swamp, LargeMammal) => 0.45,
        (Mountain | Tundra | Snow, Bird) => 0.70,
        (Mountain | Tundra | Snow, LargeMammal) => 0.55,
        (Mountain | Tundra | Snow, SmallFauna) => 0.50,
    }
}

fn habitat_at(seed: u64, world_x: f64, world_y: f64, biome: TileType) -> HabitatCell {
    let base = biome_life_base(biome);
    let idx = ((world_x as i64) << 20) ^ (world_y as i64);
    let idx = idx as u64;

    let pot_n = unit_noise(seed, 401, idx) as f32;
    let potential = (base * (0.55 + 0.45 * pot_n)).clamp(0.0, 1.0);

    let large_n = unit_noise(seed, 402, idx) as f32;
    let bird_n = unit_noise(seed, 403, idx) as f32;
    let small_n = unit_noise(seed, 404, idx) as f32;

    // Soft spatial separation so groups do not fully overlap (B),
    // while still sitting under the biome wash (A).
    let large_mammals =
        (potential * group_bias(biome, FaunaGroup::LargeMammal) * (0.35 + 0.65 * large_n))
            .clamp(0.0, 1.0);
    let birds = (potential * group_bias(biome, FaunaGroup::Bird) * (0.35 + 0.65 * bird_n))
        .clamp(0.0, 1.0);
    let small_fauna =
        (potential * group_bias(biome, FaunaGroup::SmallFauna) * (0.35 + 0.65 * small_n))
            .clamp(0.0, 1.0);

    HabitatCell {
        potential,
        large_mammals,
        birds,
        small_fauna,
    }
}

pub fn generate_region_ecology(config: WorldConfig, region: RegionId) -> RegionEcologyMap {
    let sampler = TerrainSampler::new(config.clone());
    let bounds = RegionBounds::from_region(region, config.region_size);
    // One cell per world unit — readable wash, light payload.
    let width = config.region_size;
    let height = config.region_size;
    let seed = config.seed;
    let chunk_size = config.chunk_size;
    let tiles_across = (config.region_size / chunk_size).max(1);

    let rows: Vec<Vec<HabitatCell>> = (0..height)
        .into_par_iter()
        .map(|y| {
            (0..width)
                .map(|x| {
                    let world_x = f64::from(bounds.world_x0) + x as f64 + 0.5;
                    let world_y = f64::from(bounds.world_y0) + y as f64 + 0.5;
                    let cell = sampler.cell_at(world_x, world_y, LodLevel::Macro);
                    habitat_at(seed, world_x, world_y, cell.biome)
                })
                .collect()
        })
        .collect();

    let cells: Vec<HabitatCell> = rows.into_iter().flatten().collect();

    let mut tiles = Vec::with_capacity((tiles_across * tiles_across) as usize);
    for ty in 0..tiles_across {
        for tx in 0..tiles_across {
            tiles.push(tile_expectation(
                seed,
                &sampler,
                &bounds,
                tx,
                ty,
                chunk_size,
                tiles_across,
            ));
        }
    }

    RegionEcologyMap {
        width,
        height,
        cells,
        tiles_across,
        tiles,
    }
}

fn tile_expectation(
    seed: u64,
    sampler: &TerrainSampler,
    bounds: &RegionBounds,
    tile_cx: u32,
    tile_cy: u32,
    chunk_size: u32,
    tiles_across: u32,
) -> RegionTileExpect {
    let x0 = f64::from(bounds.world_x0) + f64::from(tile_cx * chunk_size);
    let y0 = f64::from(bounds.world_y0) + f64::from(tile_cy * chunk_size);
    let span = f64::from(chunk_size);

    let mut biome_votes: [u32; 14] = [0; 14];
    let mut pot = 0.0f32;
    let mut large = 0.0f32;
    let mut birds = 0.0f32;
    let mut small = 0.0f32;
    let mut samples = 0u32;

    for sy in 0..4u32 {
        for sx in 0..4u32 {
            let world_x = x0 + (sx as f64 + 0.5) / 4.0 * span;
            let world_y = y0 + (sy as f64 + 0.5) / 4.0 * span;
            let cell = sampler.cell_at(world_x, world_y, LodLevel::Macro);
            let hab = habitat_at(seed, world_x, world_y, cell.biome);
            biome_votes[cell.biome as usize] += 1;
            pot += hab.potential;
            large += hab.large_mammals;
            birds += hab.birds;
            small += hab.small_fauna;
            samples += 1;
        }
    }

    let inv = 1.0 / samples.max(1) as f32;
    pot *= inv;
    large *= inv;
    birds *= inv;
    small *= inv;

    let mut best_i = 0usize;
    let mut best_v = 0u32;
    for (i, &v) in biome_votes.iter().enumerate() {
        if v > best_v {
            best_v = v;
            best_i = i;
        }
    }
    let biome = tile_type_from_index(best_i);

    let tile_idx =
        (region_tile_index(tile_cx, tile_cy, tiles_across) as u64).wrapping_add(seed);
    let flora = expect_flora(seed, tile_idx, biome, pot);
    let fauna = expect_fauna(seed, tile_idx, biome, large, birds, small);

    RegionTileExpect { flora, fauna }
}

fn region_tile_index(tx: u32, ty: u32, across: u32) -> u32 {
    ty * across + tx
}

fn tile_type_from_index(i: usize) -> TileType {
    match i {
        0 => TileType::DeepWater,
        1 => TileType::Water,
        2 => TileType::Sand,
        3 => TileType::Grass,
        4 => TileType::Forest,
        5 => TileType::Mountain,
        6 => TileType::Snow,
        7 => TileType::RockyShore,
        8 => TileType::Desert,
        9 => TileType::Savanna,
        10 => TileType::Swamp,
        11 => TileType::Shrubland,
        12 => TileType::Rainforest,
        _ => TileType::Tundra,
    }
}

fn expect_flora(seed: u64, tile_idx: u64, biome: TileType, potential: f32) -> Vec<String> {
    if potential < 0.22 {
        return Vec::new();
    }
    let catalog = flora_for(biome);
    let count = if potential > 0.7 { 4 } else if potential > 0.4 { 3 } else { 2 };
    let mut out = Vec::with_capacity(count);
    let mut used = [false; SPECIES_PER_BIOME];
    for n in 0..count {
        let mut si = (unit_noise(seed, 601 + n as u64, tile_idx) * SPECIES_PER_BIOME as f64)
            as usize
            % SPECIES_PER_BIOME;
        for _ in 0..SPECIES_PER_BIOME {
            if !used[si] {
                break;
            }
            si = (si + 1) % SPECIES_PER_BIOME;
        }
        used[si] = true;
        out.push(catalog[si].name.to_string());
    }
    out
}

fn expect_fauna(
    seed: u64,
    tile_idx: u64,
    biome: TileType,
    large: f32,
    birds: f32,
    small: f32,
) -> Vec<String> {
    let strength = large.max(birds).max(small);
    if strength < 0.22 {
        return Vec::new();
    }
    let catalog = fauna_for(biome);
    let mut groups: Vec<(FaunaGroup, f32)> = vec![
        (FaunaGroup::LargeMammal, large),
        (FaunaGroup::Bird, birds),
        (FaunaGroup::SmallFauna, small),
    ];
    groups.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut out = Vec::new();
    let mut used = [false; SPECIES_PER_BIOME];
    for (g, v) in groups {
        if v < 0.28 || out.len() >= 4 {
            continue;
        }
        let pool: Vec<usize> = catalog
            .iter()
            .enumerate()
            .filter_map(|(i, d)| if d.group == g { Some(i) } else { None })
            .collect();
        if pool.is_empty() {
            continue;
        }
        let take = if v > 0.55 { 2 } else { 1 };
        for n in 0..take {
            if out.len() >= 4 {
                break;
            }
            let mut pi = (unit_noise(
                seed,
                611 + n as u64 + fauna_group_salt(g),
                tile_idx,
            ) * pool.len() as f64) as usize
                % pool.len();
            for _ in 0..pool.len() {
                if !used[pool[pi]] {
                    break;
                }
                pi = (pi + 1) % pool.len();
            }
            let si = pool[pi];
            if used[si] {
                continue;
            }
            used[si] = true;
            out.push(catalog[si].name.to_string());
        }
    }
    out
}

fn fauna_group_salt(g: FaunaGroup) -> u64 {
    match g {
        FaunaGroup::LargeMammal => 0,
        FaunaGroup::Bird => 3,
        FaunaGroup::SmallFauna => 6,
    }
}

fn pick_subspecies(seed: u64, channel: u64, index: u64, options: &[&'static str; 2]) -> &'static str {
    if unit_noise(seed, channel, index) < 0.5 {
        options[0]
    } else {
        options[1]
    }
}

pub fn generate_chunk_ecology(
    config: WorldConfig,
    region: RegionId,
    chunk: ChunkId,
) -> ChunkEcologyMap {
    let sampler = TerrainSampler::new(config.clone());
    let bounds = RegionBounds::from_chunk(region, chunk, config.region_size, config.chunk_size);
    let seed = config.seed;
    let x0 = bounds.world_x0 as f64;
    let y0 = bounds.world_y0 as f64;
    let span = f64::from(config.chunk_size);

    // Finer wash than region (1 cell / world unit) so zoom keeps the same colors.
    let width = config.chunk_size * 4;
    let height = config.chunk_size * 4;
    let mut cells = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let world_x = x0 + (x as f64 + 0.5) / width as f64 * span;
            let world_y = y0 + (y as f64 + 0.5) / height as f64 * span;
            let cell = sampler.cell_at(world_x, world_y, LodLevel::Micro);
            cells.push(habitat_at(seed, world_x, world_y, cell.biome));
        }
    }

    let mut flora = Vec::new();
    let mut fauna = Vec::new();

    // Species samples for hover detail (not drawn as flora dots).
    let steps = 24u32;
    for gy in 0..steps {
        for gx in 0..steps {
            let world_x = x0 + (gx as f64 + 0.5) / steps as f64 * span;
            let world_y = y0 + (gy as f64 + 0.5) / steps as f64 * span;
            let cell = sampler.cell_at(world_x, world_y, LodLevel::Micro);
            let hab = habitat_at(seed, world_x, world_y, cell.biome);
            let idx = ((world_x as i64) << 20) ^ (world_y as i64);
            let idx = idx as u64;

            let flora_chance = hab.potential
                * if is_water_biome(cell.elevation) {
                    0.55
                } else {
                    0.75
                };
            if unit_noise(seed, 501, idx) < f64::from(flora_chance) * 0.65 {
                let catalog = flora_for(cell.biome);
                let si = (unit_noise(seed, 502, idx) * SPECIES_PER_BIOME as f64) as usize
                    % SPECIES_PER_BIOME;
                let def = &catalog[si];
                flora.push(LifeInstance {
                    x: world_x as f32,
                    y: world_y as f32,
                    kind: LifeKind::Flora,
                    group: None,
                    species: def.name.to_string(),
                    subspecies: pick_subspecies(seed, 503, idx, &def.subspecies).to_string(),
                    biome: cell.biome,
                });
            }

            let fauna_strength = hab
                .large_mammals
                .max(hab.birds)
                .max(hab.small_fauna);
            if unit_noise(seed, 511, idx) < f64::from(fauna_strength) * 0.18 {
                let catalog = fauna_for(cell.biome);
                let group_roll = unit_noise(seed, 513, idx) as f32;
                let preferred = if hab.large_mammals >= hab.birds
                    && hab.large_mammals >= hab.small_fauna
                    && group_roll < 0.55
                {
                    FaunaGroup::LargeMammal
                } else if hab.birds >= hab.small_fauna && group_roll < 0.70 {
                    FaunaGroup::Bird
                } else {
                    FaunaGroup::SmallFauna
                };
                let group_species: Vec<&FaunaDef> =
                    catalog.iter().filter(|d| d.group == preferred).collect();
                let pool = if group_species.is_empty() {
                    catalog.iter().collect::<Vec<_>>()
                } else {
                    group_species
                };
                let si =
                    (unit_noise(seed, 512, idx) * pool.len() as f64) as usize % pool.len();
                let def = pool[si];
                fauna.push(LifeInstance {
                    x: world_x as f32,
                    y: world_y as f32,
                    kind: LifeKind::Fauna,
                    group: Some(def.group),
                    species: def.name.to_string(),
                    subspecies: pick_subspecies(seed, 514, idx, &def.subspecies).to_string(),
                    biome: cell.biome,
                });
            }
        }
    }

    ChunkEcologyMap {
        width,
        height,
        cells,
        flora,
        fauna,
    }
}

// --- Catalogs (Polish labels, mainly earthly) ---

const DEEP_WATER_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Glon abisalny", subspecies: ["ciemny", "świecący"] },
    FloraDef { name: "Maty bakteryjne", subspecies: ["siarkowe", "żelaziste"] },
    FloraDef { name: "Gąbka głębinowa", subspecies: ["szklana", "koralowa"] },
    FloraDef { name: "Mech hydrotermalny", subspecies: ["czerwony", "biały"] },
    FloraDef { name: "Krzewik kominowy", subspecies: ["niski", "wysoki"] },
    FloraDef { name: "Alga nitkowata", subspecies: ["rzadka", "gęsta"] },
    FloraDef { name: "Plankton roślinny", subspecies: ["zielony", "czerwony"] },
    FloraDef { name: "Porost skalny głębin", subspecies: ["szary", "fioletowy"] },
    FloraDef { name: "Wodorosty dryfujące", subspecies: ["wąskie", "szerokie"] },
    FloraDef { name: "Kolonia bakterii kominowych", subspecies: ["gęsta", "luźna"] },
];

const WATER_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Trawa morska", subspecies: ["zielona", "siwa"] },
    FloraDef { name: "Listownica", subspecies: ["olbrzymia", "karłowata"] },
    FloraDef { name: "Glon płytki", subspecies: ["brunatny", "czerwony"] },
    FloraDef { name: "Lilia wodna", subspecies: ["biała", "różowa"] },
    FloraDef { name: "Trzcina brzegowa", subspecies: ["wysoka", "niska"] },
    FloraDef { name: "Rdestnica", subspecies: ["wąskolistna", "szerokolistna"] },
    FloraDef { name: "Moczarka", subspecies: ["zwarta", "luźna"] },
    FloraDef { name: "Grążel", subspecies: ["żółty", "kremowy"] },
    FloraDef { name: "Wodorost zatokowy", subspecies: ["pierzasty", "taśmowy"] },
    FloraDef { name: "Mech wodny", subspecies: ["ciemny", "jasny"] },
];

const SAND_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Trawa wydmowa", subspecies: ["piaskowa", "siwa"] },
    FloraDef { name: "Rukiew nadmorska", subspecies: ["niska", "płożąca"] },
    FloraDef { name: "Sosna nadmorska", subspecies: ["krzywa", "wyprostowana"] },
    FloraDef { name: "Rokietta", subspecies: ["kolczasta", "miękka"] },
    FloraDef { name: "Mikołajek", subspecies: ["srebrny", "zielony"] },
    FloraDef { name: "Wierzba piaskowa", subspecies: ["krzew", "drzewko"] },
    FloraDef { name: "Szczotlicha", subspecies: ["gęsta", "rzadka"] },
    FloraDef { name: "Łubin piaskowy", subspecies: ["niebieski", "żółty"] },
    FloraDef { name: "Jałowiec wydmowy", subspecies: ["płożący", "stożkowy"] },
    FloraDef { name: "Kocanka", subspecies: ["żółta", "biała"] },
];

const ROCKY_SHORE_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Porost skalny", subspecies: ["pomarańczowy", "szary"] },
    FloraDef { name: "Glon przyboju", subspecies: ["zielony", "brunatny"] },
    FloraDef { name: "Armeria", subspecies: ["różowa", "biała"] },
    FloraDef { name: "Rozchodnik nadmorski", subspecies: ["żółty", "czerwony"] },
    FloraDef { name: "Traganek skalny", subspecies: ["niski", "płożący"] },
    FloraDef { name: "Wrzosiec nadmorski", subspecies: ["różowy", "fioletowy"] },
    FloraDef { name: "Paproć skalna", subspecies: ["delikatna", "sztywna"] },
    FloraDef { name: "Mech basenów pływowych", subspecies: ["ciemny", "jasny"] },
    FloraDef { name: "Kapusta morska", subspecies: ["liściasta", "kędzierzawa"] },
    FloraDef { name: "Soliród", subspecies: ["zielony", "czerwony"] },
];

const DESERT_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Kaktus kolumnowy", subspecies: ["wysoki", "rozgałęziony"] },
    FloraDef { name: "Agawa", subspecies: ["niebieskawa", "zielona"] },
    FloraDef { name: "Akacja pustynna", subspecies: ["cierniowa", "bezcierniowa"] },
    FloraDef { name: "Trawa stepowa", subspecies: ["srebrna", "złota"] },
    FloraDef { name: "Palma oazowa", subspecies: ["daktylowa", "wachlarzowa"] },
    FloraDef { name: "Aloes", subspecies: ["kolczasty", "gładki"] },
    FloraDef { name: "Bylina solniskowa", subspecies: ["czerwona", "szara"] },
    FloraDef { name: "Saksauł", subspecies: ["niski", "wysoki"] },
    FloraDef { name: "Kwiat pustynny", subspecies: ["biały", "fioletowy"] },
    FloraDef { name: "Mech kamienny", subspecies: ["czarny", "zielony"] },
];

const SAVANNA_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Baobab", subspecies: ["krótki", "olbrzymi"] },
    FloraDef { name: "Akacja parasolowa", subspecies: ["płaska", "stożkowa"] },
    FloraDef { name: "Trawa wysoka", subspecies: ["złota", "zielona"] },
    FloraDef { name: "Krzew cierniowy", subspecies: ["gęsty", "rzadki"] },
    FloraDef { name: "Aloes sawannowy", subspecies: ["kolumnowy", "rozetowy"] },
    FloraDef { name: "Palma dzika", subspecies: ["niska", "wysoka"] },
    FloraDef { name: "Bylina sezonowa", subspecies: ["czerwona", "żółta"] },
    FloraDef { name: "Trawa słoniowa", subspecies: ["gęsta", "luźna"] },
    FloraDef { name: "Krzew jagodowy", subspecies: ["czerwony", "czarny"] },
    FloraDef { name: "Kwiat sawannowy", subspecies: ["pomarańczowy", "biały"] },
];

const GRASS_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Trawa łąkowa", subspecies: ["miękka", "sztywna"] },
    FloraDef { name: "Macierzanka", subspecies: ["różowa", "fioletowa"] },
    FloraDef { name: "Rumianek", subspecies: ["biały", "żółty"] },
    FloraDef { name: "Koniczyna", subspecies: ["biała", "czerwona"] },
    FloraDef { name: "Jaskier", subspecies: ["żółty", "złoty"] },
    FloraDef { name: "Dziurawiec", subspecies: ["niski", "wysoki"] },
    FloraDef { name: "Chaber", subspecies: ["niebieski", "fioletowy"] },
    FloraDef { name: "Mak polny", subspecies: ["czerwony", "różowy"] },
    FloraDef { name: "Wierzba łąkowa", subspecies: ["krzew", "drzewko"] },
    FloraDef { name: "Paproć łąkowa", subspecies: ["delikatna", "sztywna"] },
];

const SWAMP_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Trzcina", subspecies: ["wysoka", "niska"] },
    FloraDef { name: "Pałka wodna", subspecies: ["szeroka", "wąska"] },
    FloraDef { name: "Torfowiec", subspecies: ["czerwony", "zielony"] },
    FloraDef { name: "Olcha bagienna", subspecies: ["niska", "wysoka"] },
    FloraDef { name: "Skrzyp", subspecies: ["polny", "bagienny"] },
    FloraDef { name: "Kosaciec", subspecies: ["żółty", "niebieski"] },
    FloraDef { name: "Bluszcz bagienny", subspecies: ["płożący", "pnący"] },
    FloraDef { name: "Grzyb bagienny", subspecies: ["biały", "czarny"] },
    FloraDef { name: "Rosiczka", subspecies: ["okrągłolistna", "długolistna"] },
    FloraDef { name: "Wierzba bagienna", subspecies: ["srebrna", "zielona"] },
];

const SHRUBLAND_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Jałowiec", subspecies: ["płożący", "drzewiasty"] },
    FloraDef { name: "Wrzos", subspecies: ["różowy", "biały"] },
    FloraDef { name: "Dzika róża", subspecies: ["czerwona", "biała"] },
    FloraDef { name: "Głóg", subspecies: ["kolczasty", "miękki"] },
    FloraDef { name: "Lawenda", subspecies: ["wąskolistna", "szerokolistna"] },
    FloraDef { name: "Rozmaryn", subspecies: ["wyprostowany", "płożący"] },
    FloraDef { name: "Tymianek", subspecies: ["cytrynowy", "klasyczny"] },
    FloraDef { name: "Pistacja dzika", subspecies: ["niska", "wysoka"] },
    FloraDef { name: "Ostrokrzew", subspecies: ["ciernisty", "gładki"] },
    FloraDef { name: "Szałwia", subspecies: ["fioletowa", "srebrna"] },
];

const FOREST_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Dąb", subspecies: ["szypułkowy", "bezszypułkowy"] },
    FloraDef { name: "Buk", subspecies: ["zwyczajny", "czerwony"] },
    FloraDef { name: "Sosna", subspecies: ["zwyczajna", "limba"] },
    FloraDef { name: "Świerk", subspecies: ["pospolity", "srebrny"] },
    FloraDef { name: "Brzoza", subspecies: ["brodawkowata", "omszona"] },
    FloraDef { name: "Paproć orlica", subspecies: ["wysoka", "niska"] },
    FloraDef { name: "Borówka", subspecies: ["czarna", "brusznica"] },
    FloraDef { name: "Grzyb kapeluszowy", subspecies: ["jadalny", "trujący"] },
    FloraDef { name: "Bluszcz", subspecies: ["pospolity", "kolchidzki"] },
    FloraDef { name: "Leszczyna", subspecies: ["krzew", "drzewko"] },
];

const RAINFOREST_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Drzewo koron", subspecies: ["szerokie", "smukłe"] },
    FloraDef { name: "Liana", subspecies: ["gruba", "cienka"] },
    FloraDef { name: "Storczyk", subspecies: ["fioletowy", "biały"] },
    FloraDef { name: "Paproć drzewiasta", subspecies: ["wysoka", "niska"] },
    FloraDef { name: "Bromelia", subspecies: ["czerwona", "zielona"] },
    FloraDef { name: "Figowiec banyan", subspecies: ["pojedynczy", "wielopienny"] },
    FloraDef { name: "Palma deszczowa", subspecies: ["wachlarzowa", "pierzasta"] },
    FloraDef { name: "Grzyb świecący", subspecies: ["zielony", "niebieski"] },
    FloraDef { name: "Epifit", subspecies: ["młody", "dojrzały"] },
    FloraDef { name: "Rafflezja", subspecies: ["czerwona", "plamista"] },
];

const MOUNTAIN_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Kosodrzewina", subspecies: ["gęsta", "rzadka"] },
    FloraDef { name: "Goryczka", subspecies: ["niebieska", "żółta"] },
    FloraDef { name: "Szarotka", subspecies: ["klasyczna", "szerokolistna"] },
    FloraDef { name: "Różeniec", subspecies: ["różowy", "żółty"] },
    FloraDef { name: "Mech skalny", subspecies: ["szary", "zielony"] },
    FloraDef { name: "Porost mapowy", subspecies: ["żółty", "pomarańczowy"] },
    FloraDef { name: "Wierzba karłowata", subspecies: ["płożąca", "krzew"] },
    FloraDef { name: "Dzwonek alpejski", subspecies: ["fioletowy", "biały"] },
    FloraDef { name: "Sasanka", subspecies: ["biała", "fioletowa"] },
    FloraDef { name: "Jałowiec górski", subspecies: ["płożący", "stożkowy"] },
];

const TUNDRA_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Mech reniferowy", subspecies: ["szary", "zielony"] },
    FloraDef { name: "Porost karłowaty", subspecies: ["żółty", "czarny"] },
    FloraDef { name: "Wierzba polarna", subspecies: ["płożąca", "krzewinka"] },
    FloraDef { name: "Wełnianka", subspecies: ["biała", "kremowa"] },
    FloraDef { name: "Skalnica", subspecies: ["różowa", "biała"] },
    FloraDef { name: "Borówka polarna", subspecies: ["czerwona", "niebieska"] },
    FloraDef { name: "Trawa tundry", subspecies: ["krótka", "kępkowa"] },
    FloraDef { name: "Driada", subspecies: ["ośmiopłatkowa", "karłowata"] },
    FloraDef { name: "Jaskier polarny", subspecies: ["żółty", "biały"] },
    FloraDef { name: "Turzyca polarna", subspecies: ["wąska", "szeroka"] },
];

const SNOW_FLORA: [FloraDef; 10] = [
    FloraDef { name: "Porost nunataku", subspecies: ["czarny", "pomarańczowy"] },
    FloraDef { name: "Mech lodowcowy", subspecies: ["zielony", "czerwony"] },
    FloraDef { name: "Alga śnieżna", subspecies: ["czerwona", "zielona"] },
    FloraDef { name: "Wierzba nunatakowa", subspecies: ["karłowata", "płożąca"] },
    FloraDef { name: "Skalnica lodowa", subspecies: ["biała", "różowa"] },
    FloraDef { name: "Turzyca śnieżna", subspecies: ["kępkowa", "luźna"] },
    FloraDef { name: "Porost mapowy lodowy", subspecies: ["żółty", "szary"] },
    FloraDef { name: "Bylina efemeryczna", subspecies: ["biała", "fioletowa"] },
    FloraDef { name: "Mech szczelinowy", subspecies: ["ciemny", "jasny"] },
    FloraDef { name: "Glon firnowy", subspecies: ["różowy", "pomarańczowy"] },
];

const DEEP_WATER_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Ryba abisalna", subspecies: ["zębata", "ślepa"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Kałamarnica olbrzymia", subspecies: ["północna", "równikowa"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Krab kominowy", subspecies: ["czerwony", "czarny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Meduza głębinowa", subspecies: ["świecąca", "ciemna"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Węgorz głębinowy", subspecies: ["smukły", "gruby"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Krewetka świecąca", subspecies: ["różowa", "niebieska"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Rekin głębinowy", subspecies: ["mały", "duży"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Homar abisalny", subspecies: ["kolczasty", "gładki"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Ośmiornica głębinowa", subspecies: ["mimikra", "jadowita"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Wieloryb nurkujący", subspecies: ["krótki nurk", "długi nurk"], group: FaunaGroup::LargeMammal },
];

const WATER_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Ryba ławicowa", subspecies: ["srebrna", "niebieska"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Delfin", subspecies: ["przybrzeżny", "oceaniczny"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Foka", subspecies: ["szara", "cętkowana"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Żółw morski", subspecies: ["zielony", "skórzasty"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Mewa", subspecies: ["biała", "ciemnoskrzydła"], group: FaunaGroup::Bird },
    FaunaDef { name: "Krab brzegowy", subspecies: ["zielony", "czerwony"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Ostryga", subspecies: ["płaska", "kubkowata"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Sum jeziorny", subspecies: ["pstry", "czarny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Wydra", subspecies: ["rzeczna", "morska"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Kormoran", subspecies: ["czarny", "dwubarwny"], group: FaunaGroup::Bird },
];

const SAND_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Krab piaskowy", subspecies: ["szybki", "norujący"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Mewa srebrzysta", subspecies: ["morska", "śródlądowa"], group: FaunaGroup::Bird },
    FaunaDef { name: "Jaszczurka piaskowa", subspecies: ["pstra", "jednolita"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Żuk gnojowy", subspecies: ["mały", "duży"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Zając piaskowy", subspecies: ["jasny", "cętkowany"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Sieweczka", subspecies: ["krótkodzioba", "długodzioba"], group: FaunaGroup::Bird },
    FaunaDef { name: "Wąż piaskowy", subspecies: ["piaskowy", "szary"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Lis pustynny", subspecies: ["jasny", "rudawy"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Skorpion piaskowy", subspecies: ["żółty", "czarny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Żółw lądowy", subspecies: ["płaski", "wysoki"], group: FaunaGroup::LargeMammal },
];

const ROCKY_SHORE_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Mewa klifowa", subspecies: ["głośna", "cicha"], group: FaunaGroup::Bird },
    FaunaDef { name: "Kormoran klifowy", subspecies: ["czarny", "dwubarwny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Foka klifowa", subspecies: ["szara", "plamista"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Krab skalny", subspecies: ["czerwony", "zielony"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Ślimak przyboju", subspecies: ["stożkowy", "spiralny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Ucho morskie", subspecies: ["zielone", "różowe"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Jaszczurka klifowa", subspecies: ["szara", "zielona"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Bielik", subspecies: ["jasny", "ciemny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Wydra skalna", subspecies: ["krótka", "długa"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Maskonur", subspecies: ["kolorowy", "blady"], group: FaunaGroup::Bird },
];

const DESERT_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Wielbłąd", subspecies: ["jednogarbny", "dwugarbny"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Fenek", subspecies: ["uszy duże", "uszy średnie"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Skorpion", subspecies: ["żółty", "czarny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Wąż piaskowy", subspecies: ["rogaty", "gładki"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Jaszczurka pustynna", subspecies: ["kołnierzasta", "gładka"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Orzeł pustynny", subspecies: ["jasny", "ciemny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Gazela", subspecies: ["dorcas", "dama"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Żuk pustynny", subspecies: ["czarny", "metaliczny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Sowa pustynna", subspecies: ["mała", "średnia"], group: FaunaGroup::Bird },
    FaunaDef { name: "Szarańcza", subspecies: ["zielona", "piaskowa"], group: FaunaGroup::SmallFauna },
];

const SAVANNA_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Lew", subspecies: ["grzywa ciemna", "grzywa jasna"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Zebra", subspecies: ["wąskopaska", "szerokopaska"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Żyrafa", subspecies: ["siatkowana", "plamista"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Słoń", subspecies: ["sawannowy", "karłowaty"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Antylopa", subspecies: ["impala", "gnu"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Hiena", subspecies: ["cętkowana", "pręgowana"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Struś", subspecies: ["czarny", "brązowy"], group: FaunaGroup::Bird },
    FaunaDef { name: "Gepard", subspecies: ["klasyczny", "królewski"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Sekretarz", subspecies: ["jasny", "ciemny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Waran", subspecies: ["szary", "żółty"], group: FaunaGroup::SmallFauna },
];

const GRASS_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Sarna", subspecies: ["jasna", "ciemna"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Zając", subspecies: ["szary", "rudawy"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Lis", subspecies: ["rudy", "krzyżak"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Skowronek", subspecies: ["polny", "leśny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Motyl łąkowy", subspecies: ["żółty", "niebieski"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Pszczoła", subspecies: ["miodna", "dzika"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Jeż", subspecies: ["europejski", "uszaty"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Kuna", subspecies: ["leśna", "domowa"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Myszołów", subspecies: ["jasny", "ciemny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Żmija", subspecies: ["zygzak", "czarna"], group: FaunaGroup::SmallFauna },
];

const SWAMP_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Żaba", subspecies: ["zielona", "brunatna"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Bocian", subspecies: ["biały", "czarny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Pijawka", subspecies: ["medyczna", "końska"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Wąż błotny", subspecies: ["wodny", "lądowy"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Wydra bagienna", subspecies: ["ciemna", "jasna"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Czapla", subspecies: ["siwa", "biała"], group: FaunaGroup::Bird },
    FaunaDef { name: "Żółw błotny", subspecies: ["europejski", "cętkowany"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Bóbr", subspecies: ["rzeczny", "jeziorny"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Rak", subspecies: ["sygnałowy", "szlachetny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Błotniak", subspecies: ["stawowy", "zbożowy"], group: FaunaGroup::Bird },
];

const SHRUBLAND_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Koza dzika", subspecies: ["górska", "stepowa"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Zając szarak", subspecies: ["jasny", "ciemny"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Orzeł stepowy", subspecies: ["jasny", "ciemny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Jaszczurka murówka", subspecies: ["zielona", "brązowa"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Wąż Eskulapa", subspecies: ["żółty", "czarny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Lis stepowy", subspecies: ["rudy", "szary"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Przepiórka", subspecies: ["polna", "górska"], group: FaunaGroup::Bird },
    FaunaDef { name: "Żuk krzewiasty", subspecies: ["zielony", "brązowy"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Sowa uszata", subspecies: ["jasna", "ciemna"], group: FaunaGroup::Bird },
    FaunaDef { name: "Żbik", subspecies: ["plamisty", "pręgowany"], group: FaunaGroup::LargeMammal },
];

const FOREST_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Jeleń", subspecies: ["szlachetny", "sika"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Dzik", subspecies: ["europejski", "karłowaty"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Wilk", subspecies: ["szary", "czarny"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Puszczyk", subspecies: ["szary", "rdzawy"], group: FaunaGroup::Bird },
    FaunaDef { name: "Wiewiórka", subspecies: ["ruda", "czarna"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Dzięcioł", subspecies: ["duży", "zielony"], group: FaunaGroup::Bird },
    FaunaDef { name: "Borsuk", subspecies: ["europejski", "azjatycki"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Ryś", subspecies: ["eurazjatycki", "kanadyjski"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Kuna", subspecies: ["leśna", "kamionka"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Jelonek rogacz", subspecies: ["duży", "mały"], group: FaunaGroup::SmallFauna },
];

const RAINFOREST_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Jaguar", subspecies: ["klasyczny", "czarny"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Małpa nadrzewna", subspecies: ["wyjec", "kapucynka"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Tukan", subspecies: ["duży", "mały"], group: FaunaGroup::Bird },
    FaunaDef { name: "Papuga", subspecies: ["ara", "amazonka"], group: FaunaGroup::Bird },
    FaunaDef { name: "Anakonda", subspecies: ["zielona", "żółta"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Żaba drzewna", subspecies: ["czerwona", "niebieska"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Motyl morpho", subspecies: ["niebieski", "metaliczny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Mrówka legionowa", subspecies: ["czerwona", "czarna"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Leniwiec", subspecies: ["trójpalczasty", "dwupalczasty"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Kajman", subspecies: ["czarny", "okularowy"], group: FaunaGroup::LargeMammal },
];

const MOUNTAIN_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Kozica", subspecies: ["alpejska", "pirenejska"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Orzeł przedni", subspecies: ["jasny", "ciemny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Świstak", subspecies: ["alpejski", "stepowy"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Niedźwiedź brunatny", subspecies: ["górski", "nizinny"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Zając bielak", subspecies: ["biały", "szary"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Orzełek", subspecies: ["mały", "średni"], group: FaunaGroup::Bird },
    FaunaDef { name: "Muflon", subspecies: ["europejski", "azjatycki"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Płochacz halny", subspecies: ["jasny", "ciemny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Ryś górski", subspecies: ["jasny", "ciemny"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Salamandra", subspecies: ["plamista", "czarna"], group: FaunaGroup::SmallFauna },
];

const TUNDRA_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Renifer", subspecies: ["tundryjski", "leśny"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Lis polarny", subspecies: ["biały", "niebieski"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Leming", subspecies: ["norweski", "obrożny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Sowa śnieżna", subspecies: ["biała", "cętkowana"], group: FaunaGroup::Bird },
    FaunaDef { name: "Wilk polarny", subspecies: ["biały", "kremowy"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Gęś polarna", subspecies: ["biała", "szara"], group: FaunaGroup::Bird },
    FaunaDef { name: "Zając polarny", subspecies: ["zimowy", "letni"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Piżmowół", subspecies: ["klasyczny", "karłowaty"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Petrel", subspecies: ["śnieżny", "antarktyczny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Niedźwiedź polarny", subspecies: ["dorosły", "młodociany"], group: FaunaGroup::LargeMammal },
];

const SNOW_FAUNA: [FaunaDef; 10] = [
    FaunaDef { name: "Niedźwiedź polarny", subspecies: ["jasny", "kremowy"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Foka lodowa", subspecies: ["obrączkowana", "wąsata"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Petrel śnieżny", subspecies: ["biały", "szary"], group: FaunaGroup::Bird },
    FaunaDef { name: "Ryba podlodowa", subspecies: ["srebrna", "ciemna"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Mewa polarna", subspecies: ["biała", "szara"], group: FaunaGroup::Bird },
    FaunaDef { name: "Wilk lodowy", subspecies: ["biały", "srebrny"], group: FaunaGroup::LargeMammal },
    FaunaDef { name: "Leming śnieżny", subspecies: ["jasny", "ciemny"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Orzeł lodowy", subspecies: ["jasny", "ciemny"], group: FaunaGroup::Bird },
    FaunaDef { name: "Krab podlodowy", subspecies: ["biały", "niebieski"], group: FaunaGroup::SmallFauna },
    FaunaDef { name: "Bieługa", subspecies: ["biała", "szara"], group: FaunaGroup::LargeMammal },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::config::WorldConfig;

    #[test]
    fn region_ecology_is_deterministic() {
        let config = WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        };
        let a = generate_region_ecology(config.clone(), RegionId { rx: 1, ry: 1 });
        let b = generate_region_ecology(config, RegionId { rx: 1, ry: 1 });
        assert_eq!(a.width, b.width);
        assert_eq!(a.cells.len(), b.cells.len());
        for (ca, cb) in a.cells.iter().zip(b.cells.iter()) {
            assert!((ca.potential - cb.potential).abs() < 1e-6);
            assert!((ca.large_mammals - cb.large_mammals).abs() < 1e-6);
        }
        assert_eq!(a.tiles_across, b.tiles_across);
        assert_eq!(a.tiles.len(), b.tiles.len());
        assert_eq!(a.tiles.len(), (a.tiles_across * a.tiles_across) as usize);
        for (ta, tb) in a.tiles.iter().zip(b.tiles.iter()) {
            assert_eq!(ta.flora, tb.flora);
            assert_eq!(ta.fauna, tb.fauna);
        }
    }

    #[test]
    fn chunk_ecology_is_deterministic() {
        let config = WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        };
        let region = RegionId { rx: 2, ry: 2 };
        let chunk = ChunkId { cx: 1, cy: 1 };
        let a = generate_chunk_ecology(config.clone(), region, chunk);
        let b = generate_chunk_ecology(config, region, chunk);
        assert_eq!(a.flora.len(), b.flora.len());
        assert_eq!(a.fauna.len(), b.fauna.len());
        for (ia, ib) in a.flora.iter().zip(b.flora.iter()) {
            assert_eq!(ia.species, ib.species);
            assert!((ia.x - ib.x).abs() < 1e-6);
        }
    }

    #[test]
    fn each_biome_has_ten_species() {
        for biome in [
            TileType::DeepWater,
            TileType::Water,
            TileType::Sand,
            TileType::RockyShore,
            TileType::Desert,
            TileType::Savanna,
            TileType::Grass,
            TileType::Swamp,
            TileType::Shrubland,
            TileType::Forest,
            TileType::Rainforest,
            TileType::Mountain,
            TileType::Tundra,
            TileType::Snow,
        ] {
            assert_eq!(flora_for(biome).len(), 10);
            assert_eq!(fauna_for(biome).len(), 10);
        }
    }
}
