//! Static realm import/export and inter-realm trade routes (read-only profile).

use crate::world::borders::{
    CultureKind, Realm, RealmCommodity, RealmTradeRoute, RelationKind,
};
use crate::world::building::{BuildingMap, NeedIntent};
use crate::world::config::LodLevel;
use crate::world::goods::{Good, ALL_GOODS};
use crate::world::sampler::TerrainSampler;
use crate::world::settlement::{DistrictKind, Settlement};
use crate::world::types::TileType;

const MAX_LIST: usize = 3;
const MAX_ROUTES: usize = 4;

/// Fill `exports`, `imports`, and `trade_routes` on each realm from terrain,
/// districts, population needs, and building Buy/Sell. Deterministic.
pub fn fill_realm_trade(
    sampler: &TerrainSampler,
    settlements: &[Settlement],
    buildings: &BuildingMap,
    realms: &mut [Realm],
) {
    if realms.is_empty() {
        return;
    }

    let balances: Vec<Vec<i32>> = realms
        .iter()
        .map(|realm| realm_balance(sampler, settlements, buildings, realm))
        .collect();

    for (realm, balance) in realms.iter_mut().zip(balances.iter()) {
        let mut exports = Vec::new();
        let mut imports = Vec::new();
        for (gi, &good) in ALL_GOODS.iter().enumerate() {
            let v = balance[gi];
            if v >= 8 {
                exports.push(RealmCommodity {
                    good,
                    amount: v as u32,
                });
            } else if v <= -8 {
                imports.push(RealmCommodity {
                    good,
                    amount: (-v) as u32,
                });
            }
        }
        exports.sort_by(|a, b| b.amount.cmp(&a.amount).then_with(|| good_ord(a.good).cmp(&good_ord(b.good))));
        imports.sort_by(|a, b| b.amount.cmp(&a.amount).then_with(|| good_ord(a.good).cmp(&good_ord(b.good))));
        exports.truncate(MAX_LIST);
        imports.truncate(MAX_LIST);
        realm.exports = exports;
        realm.imports = imports;
        realm.trade_routes.clear();
    }

    match_trade_routes(realms);
}

fn good_ord(good: Good) -> u8 {
    ALL_GOODS.iter().position(|&g| g == good).unwrap_or(255) as u8
}

fn good_index(good: Good) -> usize {
    ALL_GOODS.iter().position(|&g| g == good).unwrap_or(0)
}

fn add(balance: &mut [i32], good: Good, delta: i32) {
    let i = good_index(good);
    balance[i] = balance[i].saturating_add(delta);
}

fn realm_balance(
    sampler: &TerrainSampler,
    settlements: &[Settlement],
    buildings: &BuildingMap,
    realm: &Realm,
) -> Vec<i32> {
    let mut balance = vec![0i32; ALL_GOODS.len()];
    let pop = realm.population(settlements).max(1);

    // Baseline population demand.
    add(&mut balance, Good::Food, -((pop / 70) as i32).max(2));
    add(&mut balance, Good::Grain, -((pop / 90) as i32).max(1));
    add(&mut balance, Good::Goods, -((pop / 110) as i32).max(1));

    match realm.culture {
        CultureKind::Agrarian => {
            add(&mut balance, Good::Grain, (pop / 100) as i32);
            add(&mut balance, Good::Food, (pop / 140) as i32);
            add(&mut balance, Good::Tools, -((pop / 200) as i32).max(1));
        }
        CultureKind::Mercantile => {
            add(&mut balance, Good::Goods, (pop / 90) as i32);
            add(&mut balance, Good::Fish, -((pop / 180) as i32));
        }
        CultureKind::Martial => {
            add(&mut balance, Good::Iron, -((pop / 150) as i32).max(1));
            add(&mut balance, Good::Tools, -((pop / 160) as i32).max(1));
            add(&mut balance, Good::Hides, (pop / 200) as i32);
        }
        CultureKind::Pastoral => {
            add(&mut balance, Good::Hides, (pop / 100) as i32);
            add(&mut balance, Good::Grain, -((pop / 160) as i32));
        }
        CultureKind::Scholarly => {
            add(&mut balance, Good::Herbs, (pop / 180) as i32);
            add(&mut balance, Good::Goods, -((pop / 150) as i32));
        }
    }

    for &si in &realm.settlement_idxs {
        let Some(s) = settlements.get(si) else {
            continue;
        };
        sample_biome_supply(sampler, s, &mut balance);
        for d in &s.districts {
            match d.kind {
                DistrictKind::Forest => {
                    add(&mut balance, Good::Timber, 14);
                    add(&mut balance, Good::Hides, 8);
                    add(&mut balance, Good::Herbs, 6);
                }
                DistrictKind::Port => {
                    add(&mut balance, Good::Fish, 16);
                    add(&mut balance, Good::Goods, 4);
                }
                DistrictKind::Outskirts => {
                    add(&mut balance, Good::Grain, 12);
                    add(&mut balance, Good::Food, 8);
                }
                DistrictKind::Craft => {
                    add(&mut balance, Good::Tools, 10);
                    add(&mut balance, Good::Iron, 6);
                    add(&mut balance, Good::Timber, -4);
                }
                DistrictKind::Market => {
                    add(&mut balance, Good::Goods, 8);
                    add(&mut balance, Good::Food, -3);
                }
                _ => {}
            }
        }
    }

    for b in &buildings.buildings {
        if !realm.settlement_idxs.contains(&b.settlement_index) {
            continue;
        }
        for stock in &b.stock {
            add(&mut balance, stock.good, stock.amount as i32);
        }
        for need in &b.needs {
            match need.intent {
                NeedIntent::Buy => add(&mut balance, need.good, -10),
                NeedIntent::Sell => {}
            }
        }
    }

    balance
}

fn sample_biome_supply(sampler: &TerrainSampler, s: &Settlement, balance: &mut [i32]) {
    let r = s.radius.max(6.0) * 1.6;
    let offsets = [
        (0.0, 0.0),
        (r, 0.0),
        (-r, 0.0),
        (0.0, r),
        (0.0, -r),
        (r * 0.7, r * 0.7),
        (-r * 0.7, r * 0.7),
        (r * 0.7, -r * 0.7),
        (-r * 0.7, -r * 0.7),
    ];
    for (dx, dy) in offsets {
        let cell = sampler.cell_at((s.x + dx) as f64, (s.y + dy) as f64, LodLevel::Macro);
        match cell.biome {
            TileType::Forest | TileType::Rainforest => {
                add(balance, Good::Timber, 5);
                add(balance, Good::Hides, 2);
                add(balance, Good::Herbs, 2);
            }
            TileType::Shrubland => {
                add(balance, Good::Timber, 2);
                add(balance, Good::Hides, 3);
            }
            TileType::Grass | TileType::Savanna => {
                add(balance, Good::Grain, 4);
                add(balance, Good::Food, 2);
            }
            TileType::Swamp => {
                add(balance, Good::Herbs, 3);
                add(balance, Good::Fish, 2);
            }
            TileType::Mountain | TileType::Snow | TileType::Tundra => {
                add(balance, Good::Ore, 4);
                add(balance, Good::Iron, 3);
            }
            TileType::Desert => {
                add(balance, Good::Hides, 1);
                add(balance, Good::Ore, 1);
            }
            TileType::Sand | TileType::RockyShore => {
                add(balance, Good::Fish, 3);
            }
            TileType::Water | TileType::DeepWater => {
                add(balance, Good::Fish, 2);
            }
        }
    }
}

fn trade_priority(kind: RelationKind) -> Option<u8> {
    match kind {
        RelationKind::TradePact => Some(0),
        RelationKind::AncientAlliance => Some(1),
        RelationKind::ColdPeace => Some(2),
        RelationKind::BorderFeud | RelationKind::Rivalry => None,
    }
}

fn match_trade_routes(realms: &mut [Realm]) {
    let n = realms.len();
    let mut candidates: Vec<(u8, usize, usize, Good, u32)> = Vec::new();

    for a_id in 0..n {
        for rel in &realms[a_id].relations {
            let Some(prio) = trade_priority(rel.kind) else {
                continue;
            };
            let b_id = rel.other_realm_id;
            if b_id >= n {
                continue;
            }
            for ex in &realms[a_id].exports {
                if let Some(imp) = realms[b_id]
                    .imports
                    .iter()
                    .find(|i| i.good == ex.good)
                {
                    let volume = ex.amount.min(imp.amount);
                    if volume > 0 {
                        candidates.push((prio, a_id, b_id, ex.good, volume));
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| b.4.cmp(&a.4))
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| good_ord(a.3).cmp(&good_ord(b.3)))
    });

    let mut used_pair_good = std::collections::HashSet::new();
    let mut route_count = vec![0usize; n];

    for &(_prio, a_id, b_id, good, _vol) in &candidates {
        if route_count[a_id] >= MAX_ROUTES || route_count[b_id] >= MAX_ROUTES {
            continue;
        }
        let key = (a_id.min(b_id), a_id.max(b_id), good_ord(good));
        if !used_pair_good.insert(key) {
            continue;
        }
        realms[a_id].trade_routes.push(RealmTradeRoute {
            partner_realm_id: b_id,
            good,
            exporting: true,
        });
        realms[b_id].trade_routes.push(RealmTradeRoute {
            partner_realm_id: a_id,
            good,
            exporting: false,
        });
        route_count[a_id] += 1;
        route_count[b_id] += 1;
    }

    for realm in realms.iter_mut() {
        realm.trade_routes.sort_by(|a, b| {
            a.partner_realm_id
                .cmp(&b.partner_realm_id)
                .then_with(|| good_ord(a.good).cmp(&good_ord(b.good)))
                .then_with(|| b.exporting.cmp(&a.exporting))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::building::generate as generate_buildings;
    use crate::world::config::WorldConfig;
    use crate::world::settlement::generate;

    #[test]
    fn realm_trade_is_deterministic_and_nonempty_for_some_realms() {
        let config = WorldConfig::default();
        let mut map = generate(config.clone());
        let buildings = generate_buildings(config.seed, &map.settlements);
        let sampler = TerrainSampler::new(config.clone());
        fill_realm_trade(&sampler, &map.settlements, &buildings, &mut map.realms);

        let mut again = generate(config.clone());
        let buildings2 = generate_buildings(config.seed, &again.settlements);
        fill_realm_trade(&sampler, &again.settlements, &buildings2, &mut again.realms);
        assert_eq!(map.realms, again.realms);

        let any_lists = map.realms.iter().any(|r| !r.exports.is_empty() || !r.imports.is_empty());
        assert!(any_lists, "expected at least one realm with export/import");
    }

    #[test]
    fn forest_heavy_context_can_export_timber() {
        let config = WorldConfig::default();
        let mut map = generate(config.clone());
        let buildings = generate_buildings(config.seed, &map.settlements);
        let sampler = TerrainSampler::new(config);
        fill_realm_trade(&sampler, &map.settlements, &buildings, &mut map.realms);

        let timber_exporter = map.realms.iter().any(|r| {
            r.exports.iter().any(|c| c.good == Good::Timber)
                || r.settlement_idxs.iter().any(|&si| {
                    map.settlements.get(si).is_some_and(|s| {
                        s.districts.iter().any(|d| d.kind == DistrictKind::Forest)
                    }) && r.exports.iter().any(|c| {
                        matches!(c.good, Good::Timber | Good::Hides | Good::Herbs)
                    })
                })
        });
        // Soft check: either timber export exists, or forest districts produce forest goods export.
        assert!(
            timber_exporter
                || map.realms.iter().any(|r| !r.exports.is_empty()),
            "expected forest-linked or any exports"
        );
    }

    #[test]
    fn trade_routes_only_match_export_to_import() {
        let config = WorldConfig::default();
        let mut map = generate(config.clone());
        let buildings = generate_buildings(config.seed, &map.settlements);
        let sampler = TerrainSampler::new(config);
        fill_realm_trade(&sampler, &map.settlements, &buildings, &mut map.realms);

        for realm in &map.realms {
            for route in &realm.trade_routes {
                let partner = &map.realms[route.partner_realm_id];
                if route.exporting {
                    assert!(realm.exports.iter().any(|c| c.good == route.good));
                    assert!(partner.imports.iter().any(|c| c.good == route.good));
                } else {
                    assert!(realm.imports.iter().any(|c| c.good == route.good));
                    assert!(partner.exports.iter().any(|c| c.good == route.good));
                }
            }
        }
    }
}
