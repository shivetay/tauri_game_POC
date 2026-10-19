use serde::{Deserialize, Serialize};

use crate::world::prng::unit_noise;
use crate::world::settlement::{DistrictKind, Settlement, SettlementKind};

pub use crate::world::goods::Good;

/// Dedicated PRNG channel — do not reuse settlement / NPC channels.
const CHANNEL_BUILDING: u64 = 71;

/// Sentinel district index for settlement-wide buildings (no districts).
const SETTLEMENT_WIDE_DI: usize = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NeedIntent {
    Buy,
    Sell,
}

impl NeedIntent {
    pub fn label(self) -> &'static str {
        match self {
            NeedIntent::Buy => "kupuje",
            NeedIntent::Sell => "sprzedaje",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingNeed {
    pub good: Good,
    pub intent: NeedIntent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingStock {
    pub good: Good,
    pub amount: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildingKind {
    Hall,
    Inn,
    Stall,
    MarketHall,
    Smithy,
    Carpenter,
    Dock,
    FishMarket,
    Temple,
    Barracks,
    House,
    Farmstead,
    Mill,
    ForestLodge,
    Warehouse,
}

impl BuildingKind {
    pub fn label(self) -> &'static str {
        match self {
            BuildingKind::Hall => "Izba / ratusz",
            BuildingKind::Inn => "Gospoda",
            BuildingKind::Stall => "Kram",
            BuildingKind::MarketHall => "Hala targowa",
            BuildingKind::Smithy => "Kuźnia",
            BuildingKind::Carpenter => "Warsztat ciesielski",
            BuildingKind::Dock => "Przystań",
            BuildingKind::FishMarket => "Targ rybny",
            BuildingKind::Temple => "Kaplica",
            BuildingKind::Barracks => "Koszary",
            BuildingKind::House => "Dom",
            BuildingKind::Farmstead => "Zagroda",
            BuildingKind::Mill => "Młyn",
            BuildingKind::ForestLodge => "Leśniczówka",
            BuildingKind::Warehouse => "Skład",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Building {
    pub kind: BuildingKind,
    pub name: String,
    pub settlement_index: usize,
    pub district_index: usize,
    /// World position (settlement + district centroid, seed jitter).
    pub x: f32,
    pub y: f32,
    pub needs: Vec<BuildingNeed>,
    /// Static sell stock from seed (not depleted over time).
    pub stock: Vec<BuildingStock>,
}

impl Building {
    pub fn sells(&self, good: Good) -> u8 {
        self.stock
            .iter()
            .find(|s| s.good == good)
            .map(|s| s.amount)
            .unwrap_or(0)
    }

    /// Stock available for a buy need (`Food` may be filled by `Grain`).
    pub fn stock_for_need(&self, want: Good) -> u8 {
        match want {
            Good::Food => self.sells(Good::Food).max(self.sells(Good::Grain)),
            other => self.sells(other),
        }
    }

    /// Which sell good to take for `want` (prefer exact match).
    pub fn good_for_need(&self, want: Good) -> Option<Good> {
        match want {
            Good::Food => {
                if self.sells(Good::Food) > 0 {
                    Some(Good::Food)
                } else if self.sells(Good::Grain) > 0 {
                    Some(Good::Grain)
                } else {
                    None
                }
            }
            other if self.sells(other) > 0 => Some(other),
            _ => None,
        }
    }

    pub fn adjust_stock(&mut self, good: Good, delta: i16) {
        if let Some(slot) = self.stock.iter_mut().find(|s| s.good == good) {
            let next = (slot.amount as i16 + delta).clamp(0, 100);
            slot.amount = next as u8;
        } else if delta > 0 {
            self.stock.push(BuildingStock {
                good,
                amount: (delta as u8).min(100),
            });
        }
    }

    pub fn title_line(&self) -> String {
        format!("{} — {}", self.name, self.kind.label())
    }

    /// Compact stock text shown next to the building name (e.g. "skóry 12, drewno 18").
    pub fn stock_label(&self) -> Option<String> {
        if self.stock.is_empty() {
            return None;
        }
        Some(
            self.stock
                .iter()
                .map(|s| format!("{} {}", s.good.label(), s.amount))
                .collect::<Vec<_>>()
                .join(", "),
        )
    }

    pub fn needs_label(&self) -> Option<String> {
        if self.needs.is_empty() {
            return None;
        }
        Some(
            self.needs
                .iter()
                .map(|n| format!("{}: {}", n.intent.label(), n.good.label()))
                .collect::<Vec<_>>()
                .join(" · "),
        )
    }

    pub fn preview_line(&self) -> String {
        let mut line = self.title_line();
        if let Some(stock) = self.stock_label() {
            line.push_str(" · [");
            line.push_str(&stock);
            line.push(']');
        }
        if let Some(needs) = self.needs_label() {
            line.push_str(" · ");
            line.push_str(&needs);
        }
        line
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingMap {
    pub buildings: Vec<Building>,
}

impl BuildingMap {
    pub fn in_district(&self, settlement_index: usize, district_index: usize) -> &[Building] {
        let start = self.buildings.iter().position(|b| {
            b.settlement_index == settlement_index && b.district_index == district_index
        });
        let Some(start) = start else {
            return &[];
        };
        let end = start
            + self.buildings[start..]
                .iter()
                .take_while(|b| {
                    b.settlement_index == settlement_index && b.district_index == district_index
                })
                .count();
        &self.buildings[start..end]
    }

    pub fn in_settlement(&self, settlement_index: usize) -> &[Building] {
        let start = self
            .buildings
            .iter()
            .position(|b| b.settlement_index == settlement_index);
        let Some(start) = start else {
            return &[];
        };
        let end = start
            + self.buildings[start..]
                .iter()
                .take_while(|b| b.settlement_index == settlement_index)
                .count();
        &self.buildings[start..end]
    }
}

/// Seed-stable buildings: per district, or settlement-wide when no districts.
pub fn generate(seed: u64, settlements: &[Settlement]) -> BuildingMap {
    let mut buildings = Vec::new();
    for (si, settlement) in settlements.iter().enumerate() {
        if settlement.districts.is_empty() {
            let pool = settlement_pool(settlement.kind);
            let count = roll_count(seed, si, SETTLEMENT_WIDE_DI, 3, 5.min(pool.len().max(3)));
            for i in 0..count {
                let key = building_key(si, SETTLEMENT_WIDE_DI, i);
                let kind = pick_kind(pool, seed, CHANNEL_BUILDING, key);
                buildings.push(make_building(
                    seed,
                    key,
                    kind,
                    si,
                    SETTLEMENT_WIDE_DI,
                    settlement,
                ));
            }
            continue;
        }
        for (di, district) in settlement.districts.iter().enumerate() {
            let pool = district_pool(district.kind);
            let (lo, hi) = (2usize, 4.min(pool.len().max(2)));
            let count = roll_count(seed, si, di, lo, hi);
            for i in 0..count {
                let key = building_key(si, di, i);
                let kind = pick_kind(pool, seed, CHANNEL_BUILDING, key);
                buildings.push(make_building(seed, key, kind, si, di, settlement));
            }
        }
    }
    BuildingMap { buildings }
}

/// Dedicated channel for guaranteed admin halls — must not touch `CHANNEL_BUILDING` rolls.
const CHANNEL_ADMIN: u64 = 72;

/// Ensure each capital settlement has a `Hall`. Inserts into the settlement's contiguous block.
/// Call only when (re)generating buildings, not when reusing a cached `BuildingMap` with NPCs.
pub fn ensure_capital_halls(
    seed: u64,
    settlements: &[Settlement],
    capital_idxs: &[usize],
    map: &mut BuildingMap,
) {
    for &si in capital_idxs {
        let Some(settlement) = settlements.get(si) else {
            continue;
        };
        if map
            .in_settlement(si)
            .iter()
            .any(|b| b.kind == BuildingKind::Hall)
        {
            continue;
        }
        let di = settlement
            .districts
            .iter()
            .position(|d| d.kind == DistrictKind::Center)
            .or_else(|| {
                settlement
                    .districts
                    .iter()
                    .position(|d| d.kind == DistrictKind::Noble)
            })
            .unwrap_or(SETTLEMENT_WIDE_DI);
        // High building_i avoids colliding with normal 0..4 keys for CHANNEL_BUILDING stock/pos.
        let key = building_key(si, di, 10_000);
        let mut hall = make_building(seed, key, BuildingKind::Hall, si, di, settlement);
        // Relabel with admin channel so the guaranteed hall is recognizable and stable.
        let suffix =
            ((unit_noise(seed, CHANNEL_ADMIN, key) * 9.0).floor() as u8) + 1;
        hall.name = format!("{} {}", BuildingKind::Hall.label(), suffix);

        let insert_at = match map.buildings.iter().rposition(|b| b.settlement_index == si) {
            Some(i) => i + 1,
            None => map
                .buildings
                .iter()
                .position(|b| b.settlement_index > si)
                .unwrap_or(map.buildings.len()),
        };
        map.buildings.insert(insert_at, hall);
    }
}

fn make_building(
    seed: u64,
    key: u64,
    kind: BuildingKind,
    settlement_index: usize,
    district_index: usize,
    settlement: &Settlement,
) -> Building {
    let suffix = ((unit_noise(seed, CHANNEL_BUILDING + 3, key) * 9.0).floor() as u8) + 1;
    let district_kind = settlement.districts.get(district_index).map(|d| d.kind);
    let needs = needs_for(kind).to_vec();
    let stock = roll_stock(seed, key, kind, &needs, settlement.kind, district_kind);
    let (x, y) = building_world_pos(seed, key, settlement, district_index);
    Building {
        kind,
        name: format!("{} {}", kind.label(), suffix),
        settlement_index,
        district_index,
        x,
        y,
        needs,
        stock,
    }
}

fn building_world_pos(
    seed: u64,
    key: u64,
    settlement: &Settlement,
    district_index: usize,
) -> (f32, f32) {
    let j = |ch: u64| (unit_noise(seed, ch, key) as f32 - 0.5) * settlement.radius.max(0.5) * 0.2;
    let Some(district) = settlement.districts.get(district_index) else {
        return (settlement.x + j(CHANNEL_BUILDING + 5), settlement.y + j(CHANNEL_BUILDING + 6));
    };
    let mid_a = district.a0 + district.span * 0.5 + j(CHANNEL_BUILDING + 5) * 0.15;
    let mid_r = (district.inner + district.outer) * 0.5;
    let r_scale = 0.85 + unit_noise(seed, CHANNEL_BUILDING + 6, key) as f32 * 0.25;
    let r = mid_r * r_scale;
    let lx = mid_a.cos() * r + settlement.core_dx;
    let ly = mid_a.sin() * r + settlement.core_dy;
    (settlement.x + lx, settlement.y + ly)
}

fn roll_stock(
    seed: u64,
    key: u64,
    kind: BuildingKind,
    needs: &[BuildingNeed],
    settlement_kind: SettlementKind,
    district_kind: Option<DistrictKind>,
) -> Vec<BuildingStock> {
    let settle_mult = match settlement_kind {
        SettlementKind::Hamlet => 0.6,
        SettlementKind::Village => 0.8,
        SettlementKind::Town => 1.0,
        SettlementKind::City => 1.25,
    };
    let mut stock = Vec::new();
    for (i, need) in needs.iter().enumerate() {
        if need.intent != NeedIntent::Sell {
            continue;
        }
        let base = sell_base(kind, need.good) as f64;
        let bonus = district_stock_bonus(district_kind, need.good);
        let jitter = 0.85 + unit_noise(seed, CHANNEL_BUILDING + 4 + i as u64, key) * 0.3;
        let amount = (base * settle_mult * bonus * jitter).round() as i32;
        stock.push(BuildingStock {
            good: need.good,
            amount: amount.clamp(1, 100) as u8,
        });
    }
    stock
}

fn sell_base(kind: BuildingKind, good: Good) -> u8 {
    match (kind, good) {
        (BuildingKind::Inn, Good::Food) => 18,
        (BuildingKind::Stall | BuildingKind::MarketHall, Good::Goods) => 22,
        (BuildingKind::Smithy, Good::Tools) => 14,
        (BuildingKind::Carpenter, Good::Goods) => 12,
        (BuildingKind::Dock | BuildingKind::FishMarket, Good::Fish) => 20,
        (BuildingKind::Farmstead, Good::Grain) => 24,
        (BuildingKind::Mill, Good::Food) => 20,
        (BuildingKind::ForestLodge, Good::Hides) => 16,
        (BuildingKind::ForestLodge, Good::Timber) => 20,
        (BuildingKind::Warehouse, Good::Goods) => 28,
        _ => 10,
    }
}

fn district_stock_bonus(district: Option<DistrictKind>, good: Good) -> f64 {
    match (district, good) {
        (Some(DistrictKind::Forest), Good::Hides | Good::Timber | Good::Herbs) => 1.35,
        (Some(DistrictKind::Outskirts), Good::Grain | Good::Food) => 1.25,
        (Some(DistrictKind::Port), Good::Fish) => 1.35,
        (Some(DistrictKind::Craft), Good::Tools | Good::Iron) => 1.2,
        (Some(DistrictKind::Market), Good::Goods | Good::Hides | Good::Grain) => 1.15,
        (None, _) => 1.0,
        _ => 1.0,
    }
}

fn needs_for(kind: BuildingKind) -> &'static [BuildingNeed] {
    match kind {
        BuildingKind::Hall => &[BuildingNeed {
            good: Good::Goods,
            intent: NeedIntent::Buy,
        }],
        BuildingKind::Inn => &[
            BuildingNeed {
                good: Good::Food,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Food,
                intent: NeedIntent::Sell,
            },
        ],
        BuildingKind::Stall => &[
            BuildingNeed {
                good: Good::Hides,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Grain,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Goods,
                intent: NeedIntent::Sell,
            },
        ],
        BuildingKind::MarketHall => &[
            BuildingNeed {
                good: Good::Hides,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Grain,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Goods,
                intent: NeedIntent::Sell,
            },
        ],
        BuildingKind::Smithy => &[
            BuildingNeed {
                good: Good::Iron,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Tools,
                intent: NeedIntent::Sell,
            },
        ],
        BuildingKind::Carpenter => &[
            BuildingNeed {
                good: Good::Timber,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Goods,
                intent: NeedIntent::Sell,
            },
        ],
        BuildingKind::Dock => &[
            BuildingNeed {
                good: Good::Goods,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Fish,
                intent: NeedIntent::Sell,
            },
        ],
        BuildingKind::FishMarket => &[
            BuildingNeed {
                good: Good::Fish,
                intent: NeedIntent::Sell,
            },
            BuildingNeed {
                good: Good::Goods,
                intent: NeedIntent::Buy,
            },
        ],
        BuildingKind::Temple => &[BuildingNeed {
            good: Good::Herbs,
            intent: NeedIntent::Buy,
        }],
        BuildingKind::Barracks => &[
            BuildingNeed {
                good: Good::Food,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Tools,
                intent: NeedIntent::Buy,
            },
        ],
        BuildingKind::House => &[BuildingNeed {
            good: Good::Food,
            intent: NeedIntent::Buy,
        }],
        BuildingKind::Farmstead => &[
            BuildingNeed {
                good: Good::Grain,
                intent: NeedIntent::Sell,
            },
            BuildingNeed {
                good: Good::Tools,
                intent: NeedIntent::Buy,
            },
        ],
        BuildingKind::Mill => &[
            BuildingNeed {
                good: Good::Grain,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Food,
                intent: NeedIntent::Sell,
            },
        ],
        BuildingKind::ForestLodge => &[
            BuildingNeed {
                good: Good::Hides,
                intent: NeedIntent::Sell,
            },
            BuildingNeed {
                good: Good::Timber,
                intent: NeedIntent::Sell,
            },
            BuildingNeed {
                good: Good::Goods,
                intent: NeedIntent::Buy,
            },
        ],
        BuildingKind::Warehouse => &[
            BuildingNeed {
                good: Good::Goods,
                intent: NeedIntent::Buy,
            },
            BuildingNeed {
                good: Good::Goods,
                intent: NeedIntent::Sell,
            },
        ],
    }
}

fn district_pool(kind: DistrictKind) -> &'static [BuildingKind] {
    match kind {
        DistrictKind::Center => &[
            BuildingKind::Hall,
            BuildingKind::Inn,
            BuildingKind::Warehouse,
        ],
        DistrictKind::Market => &[
            BuildingKind::Stall,
            BuildingKind::MarketHall,
            BuildingKind::Warehouse,
        ],
        DistrictKind::Craft => &[
            BuildingKind::Smithy,
            BuildingKind::Carpenter,
            BuildingKind::Warehouse,
        ],
        DistrictKind::Port => &[
            BuildingKind::Dock,
            BuildingKind::FishMarket,
            BuildingKind::Warehouse,
        ],
        DistrictKind::Temple => &[BuildingKind::Temple, BuildingKind::House],
        DistrictKind::Noble => &[BuildingKind::Hall, BuildingKind::House, BuildingKind::Inn],
        DistrictKind::Forest => &[BuildingKind::ForestLodge, BuildingKind::House],
        DistrictKind::Residential => &[BuildingKind::House, BuildingKind::Inn],
        DistrictKind::Outskirts => &[
            BuildingKind::Farmstead,
            BuildingKind::Mill,
            BuildingKind::Warehouse,
        ],
        DistrictKind::Military => &[BuildingKind::Barracks, BuildingKind::Warehouse],
    }
}

fn settlement_pool(kind: SettlementKind) -> &'static [BuildingKind] {
    match kind {
        SettlementKind::Hamlet => &[
            BuildingKind::Farmstead,
            BuildingKind::House,
            BuildingKind::Hall,
            BuildingKind::Inn,
            BuildingKind::Warehouse,
        ],
        SettlementKind::Village => &[
            BuildingKind::Hall,
            BuildingKind::Inn,
            BuildingKind::Farmstead,
            BuildingKind::Stall,
            BuildingKind::Warehouse,
        ],
        SettlementKind::Town | SettlementKind::City => &[
            BuildingKind::Hall,
            BuildingKind::Inn,
            BuildingKind::Stall,
            BuildingKind::Smithy,
            BuildingKind::Warehouse,
            BuildingKind::House,
        ],
    }
}

fn building_key(settlement_index: usize, district_index: usize, building_i: usize) -> u64 {
    (settlement_index as u64)
        .wrapping_mul(10_007)
        .wrapping_add((district_index as u64).wrapping_mul(97))
        .wrapping_add(building_i as u64)
}

fn roll_count(seed: u64, si: usize, di: usize, lo: usize, hi: usize) -> usize {
    if hi <= lo {
        return lo;
    }
    let t = unit_noise(seed, CHANNEL_BUILDING + 1, building_key(si, di, 0));
    lo + ((t * (hi - lo + 1) as f64).floor() as usize).min(hi - lo)
}

fn pick_kind(pool: &[BuildingKind], seed: u64, channel: u64, index: u64) -> BuildingKind {
    let n = pool.len().max(1) as f64;
    let i = (unit_noise(seed, channel, index) * n).floor() as usize;
    pool[i.min(pool.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::config::WorldConfig;
    use crate::world::settlement::generate as generate_settlements;

    #[test]
    fn same_seed_same_buildings() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let a = generate(6, &map.settlements);
        let b = generate(6, &map.settlements);
        assert_eq!(a.buildings.len(), b.buildings.len());
        for (x, y) in a.buildings.iter().zip(b.buildings.iter()) {
            assert_eq!(x.kind, y.kind);
            assert_eq!(x.name, y.name);
            assert_eq!(x.settlement_index, y.settlement_index);
            assert_eq!(x.district_index, y.district_index);
            assert_eq!(x.needs, y.needs);
            assert_eq!(x.stock, y.stock);
            assert!((x.x - y.x).abs() < 1e-5);
            assert!((x.y - y.y).abs() < 1e-5);
        }
    }

    #[test]
    fn sell_needs_have_positive_stock() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let buildings = generate(6, &map.settlements);
        for b in &buildings.buildings {
            for need in &b.needs {
                if need.intent != NeedIntent::Sell {
                    continue;
                }
                assert!(
                    b.sells(need.good) > 0,
                    "{:?} should stock {:?}",
                    b.kind,
                    need.good
                );
            }
        }
    }

    #[test]
    fn every_settlement_has_buildings() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let buildings = generate(6, &map.settlements);
        for (si, _) in map.settlements.iter().enumerate() {
            assert!(
                !buildings.in_settlement(si).is_empty(),
                "settlement {si} should have buildings"
            );
        }
    }

    #[test]
    fn buildings_have_static_needs() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let buildings = generate(6, &map.settlements);
        assert!(!buildings.buildings.is_empty());
        for b in &buildings.buildings {
            assert_eq!(b.needs.as_slice(), needs_for(b.kind));
            assert!(!b.needs.is_empty());
        }
    }

    #[test]
    fn stall_buys_hides() {
        let needs = needs_for(BuildingKind::Stall);
        assert!(needs.iter().any(|n| {
            n.good == Good::Hides && n.intent == NeedIntent::Buy
        }));
    }
}
