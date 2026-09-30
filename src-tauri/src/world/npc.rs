use serde::{Deserialize, Serialize};

use crate::world::prng::unit_noise;
use crate::world::settlement::{DistrictKind, Settlement, SettlementKind};

/// Dedicated PRNG channel — do not reuse settlement layout/name channels.
const CHANNEL_NPC: u64 = 49;

/// Sentinel district index for settlement-wide NPCs (no districts).
const SETTLEMENT_WIDE_DI: usize = 0;

const GIVEN_NAMES: &[&str] = &[
    "Antek", "Basia", "Czesław", "Danuta", "Ewa", "Franciszek", "Grażyna", "Henryk",
    "Irena", "Jan", "Kasia", "Lech", "Magda", "Norbert", "Olga", "Piotr", "Ryszard",
    "Stefania", "Tomek", "Urszula", "Witek", "Zofia", "Adam", "Beata", "Darek",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Npc {
    pub name: String,
    pub role: String,
    pub settlement_index: usize,
    pub district_index: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcMap {
    pub npcs: Vec<Npc>,
}

impl NpcMap {
    pub fn in_district(&self, settlement_index: usize, district_index: usize) -> &[Npc] {
        // Contiguous range: generate() appends all NPCs for a district in order.
        let start = self
            .npcs
            .iter()
            .position(|n| {
                n.settlement_index == settlement_index && n.district_index == district_index
            });
        let Some(start) = start else {
            return &[];
        };
        let end = start
            + self.npcs[start..]
                .iter()
                .take_while(|n| {
                    n.settlement_index == settlement_index && n.district_index == district_index
                })
                .count();
        &self.npcs[start..end]
    }

    /// All NPCs for a settlement (contiguous; includes settlement-wide or per-district).
    pub fn in_settlement(&self, settlement_index: usize) -> &[Npc] {
        let start = self
            .npcs
            .iter()
            .position(|n| n.settlement_index == settlement_index);
        let Some(start) = start else {
            return &[];
        };
        let end = start
            + self.npcs[start..]
                .iter()
                .take_while(|n| n.settlement_index == settlement_index)
                .count();
        &self.npcs[start..end]
    }
}

/// Seed-stable NPC roster: per district, or settlement-wide when no districts.
pub fn generate(seed: u64, settlements: &[Settlement]) -> NpcMap {
    let mut npcs = Vec::new();
    for (si, settlement) in settlements.iter().enumerate() {
        if settlement.districts.is_empty() {
            let count = settlement_npc_count(seed, si, settlement.kind);
            let roles = settlement_roles(settlement.kind);
            for i in 0..count {
                let key = npc_key(si, SETTLEMENT_WIDE_DI, i);
                let given = pick(GIVEN_NAMES, seed, CHANNEL_NPC, key);
                let role = pick(roles, seed, CHANNEL_NPC + 1, key);
                npcs.push(Npc {
                    name: (*given).to_string(),
                    role: (*role).to_string(),
                    settlement_index: si,
                    district_index: SETTLEMENT_WIDE_DI,
                });
            }
            continue;
        }
        for (di, district) in settlement.districts.iter().enumerate() {
            let count = district_npc_count(seed, si, di, district.kind);
            for i in 0..count {
                let key = npc_key(si, di, i);
                let given = pick(GIVEN_NAMES, seed, CHANNEL_NPC, key);
                let role = pick(roles_for(district.kind), seed, CHANNEL_NPC + 1, key);
                npcs.push(Npc {
                    name: (*given).to_string(),
                    role: (*role).to_string(),
                    settlement_index: si,
                    district_index: di,
                });
            }
        }
    }
    NpcMap { npcs }
}

fn npc_key(settlement_index: usize, district_index: usize, npc_i: usize) -> u64 {
    (settlement_index as u64)
        .wrapping_mul(10_007)
        .wrapping_add((district_index as u64).wrapping_mul(97))
        .wrapping_add(npc_i as u64)
}

fn roll_count(seed: u64, si: usize, di: usize, lo: usize, hi: usize) -> usize {
    if hi <= lo {
        return lo;
    }
    let t = unit_noise(seed, CHANNEL_NPC + 2, npc_key(si, di, 0));
    lo + ((t * (hi - lo + 1) as f64).floor() as usize).min(hi - lo)
}

fn district_npc_count(seed: u64, si: usize, di: usize, kind: DistrictKind) -> usize {
    let (lo, hi) = count_range(kind);
    roll_count(seed, si, di, lo, hi)
}

fn settlement_npc_count(seed: u64, si: usize, kind: SettlementKind) -> usize {
    let (lo, hi) = settlement_count_range(kind);
    roll_count(seed, si, SETTLEMENT_WIDE_DI, lo, hi)
}

fn count_range(kind: DistrictKind) -> (usize, usize) {
    match kind {
        DistrictKind::Center => (3, 5),
        DistrictKind::Market => (4, 6),
        DistrictKind::Craft => (3, 5),
        DistrictKind::Port => (2, 4),
        DistrictKind::Temple => (2, 4),
        DistrictKind::Noble => (2, 4),
        DistrictKind::Forest => (2, 3),
        DistrictKind::Residential => (3, 5),
        DistrictKind::Outskirts => (2, 4),
        DistrictKind::Military => (2, 4),
    }
}

fn settlement_count_range(kind: SettlementKind) -> (usize, usize) {
    match kind {
        SettlementKind::Hamlet => (2, 4),
        SettlementKind::Village => (3, 5),
        SettlementKind::Town => (3, 5),
        SettlementKind::City => (4, 6),
    }
}

fn roles_for(kind: DistrictKind) -> &'static [&'static str] {
    match kind {
        DistrictKind::Center => &["Burmistrz", "Urzędnik", "Strażnik", "Karczmarz"],
        DistrictKind::Market => &["Kupiec", "Handlarz", "Przekupień", "Wagomistrz"],
        DistrictKind::Craft => &["Rzemieślnik", "Kowal", "Tkacz", "Cieśla"],
        DistrictKind::Port => &["Marynarz", "Przewoźnik", "Celnik", "Rybak"],
        DistrictKind::Temple => &["Kapłan", "Mnich", "Zakrystian"],
        DistrictKind::Noble => &["Szlachcic", "Sługa", "Zarządca"],
        DistrictKind::Forest => &["Leśniczy", "Myśliwy", "Zielarz"],
        DistrictKind::Residential => &["Mieszkaniec", "Rzemieślnik", "Gospodarz"],
        DistrictKind::Outskirts => &["Chłop", "Pastuch", "Wędrowiec"],
        DistrictKind::Military => &["Żołnierz", "Kapitan", "Wartownik"],
    }
}

fn settlement_roles(kind: SettlementKind) -> &'static [&'static str] {
    match kind {
        SettlementKind::Hamlet => &["Chłop", "Pastuch", "Gospodarz"],
        SettlementKind::Village => &["Sołtys", "Chłop", "Rzemieślnik", "Karczmarz"],
        SettlementKind::Town => &["Mieszczanin", "Rzemieślnik", "Kupiec", "Strażnik"],
        SettlementKind::City => &["Mieszczanin", "Urzędnik", "Kupiec", "Strażnik"],
    }
}

fn pick<'a>(items: &'a [&'a str], seed: u64, channel: u64, index: u64) -> &'a str {
    let n = items.len().max(1) as f64;
    let i = (unit_noise(seed, channel, index) * n).floor() as usize;
    items[i.min(items.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::config::WorldConfig;
    use crate::world::settlement::generate as generate_settlements;

    #[test]
    fn same_seed_same_npcs() {
        let config = WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        };
        let map = generate_settlements(config.clone());
        let a = generate(6, &map.settlements);
        let b = generate(6, &map.settlements);
        assert_eq!(a.npcs.len(), b.npcs.len());
        for (x, y) in a.npcs.iter().zip(b.npcs.iter()) {
            assert_eq!(x.name, y.name);
            assert_eq!(x.role, y.role);
            assert_eq!(x.settlement_index, y.settlement_index);
            assert_eq!(x.district_index, y.district_index);
        }
    }

    #[test]
    fn different_seed_changes_roster() {
        let a_map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let b_map = generate_settlements(WorldConfig {
            seed: 7,
            ..WorldConfig::default()
        });
        let a = generate(6, &a_map.settlements);
        let b = generate(7, &b_map.settlements);
        assert!(
            a.npcs.len() != b.npcs.len()
                || a.npcs
                    .iter()
                    .zip(b.npcs.iter())
                    .any(|(x, y)| x.name != y.name || x.role != y.role)
        );
    }

    #[test]
    fn settlements_without_districts_have_npcs() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let npcs = generate(6, &map.settlements);
        let mut found = false;
        for (si, s) in map.settlements.iter().enumerate() {
            if s.districts.is_empty() {
                found = true;
                let roster = npcs.in_settlement(si);
                assert!(
                    !roster.is_empty(),
                    "settlement {si} ({:?}) without districts should have NPCs",
                    s.kind
                );
                assert!(roster.iter().all(|n| n.settlement_index == si));
            }
        }
        assert!(found, "seed 6 should include a settlement without districts");
    }

    #[test]
    fn in_district_returns_contiguous_slice() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let npcs = generate(6, &map.settlements);
        let Some((si, _)) = map
            .settlements
            .iter()
            .enumerate()
            .find(|(_, s)| !s.districts.is_empty())
        else {
            panic!("expected a settlement with districts for seed 6");
        };
        let di = 0usize;
        let slice = npcs.in_district(si, di);
        assert!(!slice.is_empty());
        assert!(slice
            .iter()
            .all(|n| n.settlement_index == si && n.district_index == di));
    }
}
