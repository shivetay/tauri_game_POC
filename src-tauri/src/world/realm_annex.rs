//! Post-peace settlement transfer (E1). Stance / rebellion is E3+.

use crate::world::borders::{Realm, TerritoryMap};
use crate::world::settlement::{Settlement, SettlementKind};

/// Owner realm id at world coordinates, if the cell is owned land.
pub fn owner_at_world(territory: &TerritoryMap, x: f32, y: f32) -> Option<usize> {
    let cols = territory.cols as usize;
    let rows = territory.rows as usize;
    if cols == 0 || rows == 0 || territory.step <= 0.0 || territory.owners.len() != cols * rows {
        return None;
    }
    let gx = (x / territory.step).floor() as i32;
    let gy = (y / territory.step).floor() as i32;
    if gx < 0 || gy < 0 || gx as usize >= cols || gy as usize >= rows {
        return None;
    }
    let o = territory.owners[gy as usize * cols + gx as usize];
    if o == u16::MAX {
        None
    } else {
        Some(o as usize)
    }
}

/// Move settlements whose centroid lies on `winner` land out of `loser` into `winner`.
/// Returns indices of transferred settlements (stable order).
pub fn transfer_settlements_after_annex(
    territory: &TerritoryMap,
    settlements: &[Settlement],
    realms: &mut [Realm],
    winner: usize,
    loser: usize,
) -> Vec<usize> {
    if winner >= realms.len() || loser >= realms.len() || winner == loser {
        return Vec::new();
    }

    let mut moved = Vec::new();
    let loser_idxs = realms[loser].settlement_idxs.clone();
    for si in loser_idxs {
        let Some(s) = settlements.get(si) else {
            continue;
        };
        if owner_at_world(territory, s.x, s.y) != Some(winner) {
            continue;
        }
        realms[loser].settlement_idxs.retain(|&i| i != si);
        if !realms[winner].settlement_idxs.contains(&si) {
            realms[winner].settlement_idxs.push(si);
        }
        moved.push(si);
    }

    realms[winner].settlement_idxs.sort_unstable();
    realms[loser].settlement_idxs.sort_unstable();
    ensure_capital(&mut realms[winner], settlements);
    ensure_capital(&mut realms[loser], settlements);
    moved
}

fn kind_rank(kind: SettlementKind) -> u8 {
    match kind {
        SettlementKind::City => 3,
        SettlementKind::Town => 2,
        SettlementKind::Village => 1,
        SettlementKind::Hamlet => 0,
    }
}

/// If capital is no longer in `settlement_idxs`, pick the largest remaining settlement.
pub fn ensure_capital(realm: &mut Realm, settlements: &[Settlement]) {
    if realm.settlement_idxs.contains(&realm.capital_idx) {
        return;
    }
    let Some(best) = realm
        .settlement_idxs
        .iter()
        .copied()
        .max_by_key(|&i| {
            settlements
                .get(i)
                .map(|s| (s.population, kind_rank(s.kind)))
                .unwrap_or((0, 0))
        })
    else {
        return;
    };
    realm.capital_idx = best;
}

/// Realm that currently lists this settlement, if any.
pub fn realm_id_for_settlement(realms: &[Realm], settlement_idx: usize) -> Option<usize> {
    realms
        .iter()
        .find(|r| r.settlement_idxs.contains(&settlement_idx))
        .map(|r| r.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::borders::{
        CultureKind, GovernmentKind, ReligionKind, RulerAmbition,
    };

    fn stub_settlement(x: f32, y: f32, pop: u32, name: &str) -> Settlement {
        Settlement {
            x,
            y,
            kind: SettlementKind::Town,
            population: pop,
            radius: 1.0,
            name: name.into(),
            core_dx: 0.0,
            core_dy: 0.0,
            districts: Vec::new(),
            road_approaches: Vec::new(),
            streets: Vec::new(),
            walled: false,
        }
    }

    fn stub_realm(id: usize, capital: usize, idxs: Vec<usize>) -> Realm {
        Realm {
            id,
            name: format!("R{id}"),
            capital_idx: capital,
            settlement_idxs: idxs,
            ruler_name: "W".into(),
            government: GovernmentKind::Monarchy,
            culture: CultureKind::Agrarian,
            religion: ReligionKind::AncestorRites,
            ambition: RulerAmbition::ExpandBorders,
            fame: 40,
            manpower: 100,
            military_strength: 50,
            goals: Vec::new(),
            relations: Vec::new(),
            exports: Vec::new(),
            imports: Vec::new(),
            trade_routes: Vec::new(),
            laws: Vec::new(),
        }
    }

    #[test]
    fn transfer_moves_settlement_on_winner_cell() {
        // step=10 → cell 0 is [0,10), cell 1 is [10,20)
        let territory = TerritoryMap {
            cols: 2,
            rows: 1,
            step: 10.0,
            owners: vec![0, 0], // both cells winner after annex
        };
        let settlements = vec![
            stub_settlement(5.0, 5.0, 100, "Keep"),
            stub_settlement(15.0, 5.0, 200, "Taken"),
        ];
        let mut realms = vec![
            stub_realm(0, 0, vec![0]),
            stub_realm(1, 1, vec![1]),
        ];
        let moved = transfer_settlements_after_annex(&territory, &settlements, &mut realms, 0, 1);
        assert_eq!(moved, vec![1]);
        assert_eq!(realms[0].settlement_idxs, vec![0, 1]);
        assert!(realms[1].settlement_idxs.is_empty());
        assert_eq!(realms[0].capital_idx, 0);
        // loser empty — capital left as orphaned index 1
        assert_eq!(realms[1].capital_idx, 1);
    }

    #[test]
    fn ensure_capital_picks_largest_remaining() {
        let settlements = vec![
            stub_settlement(0.0, 0.0, 50, "A"),
            stub_settlement(1.0, 0.0, 500, "B"),
            stub_settlement(2.0, 0.0, 100, "C"),
        ];
        let mut realm = stub_realm(0, 0, vec![1, 2]);
        ensure_capital(&mut realm, &settlements);
        assert_eq!(realm.capital_idx, 1);
    }
}
