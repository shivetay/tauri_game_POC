//! Session-layer diplomacy: tension, abstract war, occupation claims, embargoes.

use crate::world::borders::{Realm, RelationKind, RulerAmbition, TerritoryMap};

/// Higher = more hostile. Range intended −100..=100.
pub type Tension = i16;

/// Tension at or above this starts an abstract war (any casus — not only conquest).
const WAR_TENSION: Tension = 80;
/// Conquest-minded wars (ExpandBorders) last at least this many months.
const WAR_MIN_MONTHS_CONQUEST: u8 = 4;
/// Limited wars (prestige / faith / trade) resolve sooner — no need to occupy all land.
const WAR_MIN_MONTHS_LIMITED: u8 = 2;
/// Fame swing on peace (session delta, not seed profile).
const FAME_PEACE: i16 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiplomacyEdge {
    pub realm_a: usize,
    pub realm_b: usize,
    pub tension: Tension,
    pub kind: RelationKind,
    pub at_war: bool,
    pub war_months: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Embargo {
    /// Realm imposing the embargo.
    pub from: usize,
    /// Target realm.
    pub against: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RealmDiplomacyMap {
    pub realm_count: usize,
    pub edges: Vec<DiplomacyEdge>,
    /// Parallel to `TerritoryMap.owners`; `u16::MAX` = no occupation.
    /// De jure owner stays in territory; this is the de facto claimant (stage B).
    pub occupation: Vec<u16>,
    pub occupation_cols: u32,
    pub occupation_rows: u32,
    pub embargoes: Vec<Embargo>,
    /// Per-realm fame adjustment from wars (session-only).
    pub fame_delta: Vec<i16>,
    /// Latest player-visible event (war declared / peace).
    pub last_event: Option<String>,
}

impl RealmDiplomacyMap {
    pub fn from_realms(realms: &[Realm]) -> Self {
        let mut edges = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for realm in realms {
            for rel in &realm.relations {
                let a = realm.id.min(rel.other_realm_id);
                let b = realm.id.max(rel.other_realm_id);
                if a == b || !seen.insert((a, b)) {
                    continue;
                }
                let tension = tension_from_kind(rel.kind);
                edges.push(DiplomacyEdge {
                    realm_a: a,
                    realm_b: b,
                    tension,
                    kind: kind_from_tension(tension),
                    at_war: false,
                    war_months: 0,
                });
            }
        }
        edges.sort_by_key(|e| (e.realm_a, e.realm_b));
        Self {
            realm_count: realms.len(),
            edges,
            occupation: Vec::new(),
            occupation_cols: 0,
            occupation_rows: 0,
            embargoes: Vec::new(),
            fame_delta: vec![0; realms.len()],
            last_event: None,
        }
    }

    pub fn needs_rebuild(&self, realms: &[Realm]) -> bool {
        self.realm_count != realms.len()
            || (self.edges.is_empty() && realms.iter().any(|r| !r.relations.is_empty()))
    }

    pub fn edge(&self, a: usize, b: usize) -> Option<&DiplomacyEdge> {
        let (lo, hi) = (a.min(b), a.max(b));
        self.edges
            .iter()
            .find(|e| e.realm_a == lo && e.realm_b == hi)
    }

    pub fn kind_between(&self, a: usize, b: usize) -> Option<RelationKind> {
        self.edge(a, b).map(|e| e.kind)
    }

    pub fn tension_between(&self, a: usize, b: usize) -> Option<Tension> {
        self.edge(a, b).map(|e| e.tension)
    }

    pub fn at_war(&self, a: usize, b: usize) -> bool {
        self.edge(a, b).map(|e| e.at_war).unwrap_or(false)
    }

    pub fn has_embargo(&self, from: usize, against: usize) -> bool {
        self.embargoes
            .iter()
            .any(|e| e.from == from && e.against == against)
    }

    /// Trade suspended if the pair is at war or either side embargoes the other.
    pub fn trade_blocked(&self, a: usize, b: usize) -> bool {
        if self.at_war(a, b) {
            return true;
        }
        self.has_embargo(a, b) || self.has_embargo(b, a)
    }

    pub fn fame_of(&self, realm: &Realm) -> i16 {
        let d = self
            .fame_delta
            .get(realm.id)
            .copied()
            .unwrap_or(0);
        (realm.fame as i16 + d).clamp(0, 100)
    }

    pub fn occupied_count(&self, occupier: usize) -> usize {
        let id = occupier as u16;
        self.occupation.iter().filter(|&&c| c == id).count()
    }

    pub fn ensure_occupation_grid(&mut self, territory: &TerritoryMap) {
        let n = territory.owners.len();
        if self.occupation.len() != n
            || self.occupation_cols != territory.cols
            || self.occupation_rows != territory.rows
        {
            self.occupation = vec![u16::MAX; n];
            self.occupation_cols = territory.cols;
            self.occupation_rows = territory.rows;
        }
        if self.fame_delta.len() != self.realm_count {
            self.fame_delta.resize(self.realm_count, 0);
        }
    }

    /// Advance one in-game month. Returns `true` if de jure `owners` changed (stage C).
    pub fn tick_month(
        &mut self,
        realms: &[Realm],
        territory: &mut TerritoryMap,
        month_index: u32,
    ) -> bool {
        if realms.len() != self.realm_count {
            return false;
        }
        self.ensure_occupation_grid(territory);

        for edge in &mut self.edges {
            let Some(ra) = realms.get(edge.realm_a) else {
                continue;
            };
            let Some(rb) = realms.get(edge.realm_b) else {
                continue;
            };
            let delta = monthly_delta(ra, rb, edge, month_index);
            edge.tension = (edge.tension as i32 + delta as i32).clamp(-100, 100) as Tension;
            if !edge.at_war {
                edge.kind = kind_from_tension(edge.tension);
            }
        }

        let settled = self.update_wars(realms, territory, month_index);
        self.grow_occupation(realms, territory, month_index);
        self.refresh_embargoes(realms);
        settled
    }

    fn update_wars(
        &mut self,
        realms: &[Realm],
        territory: &mut TerritoryMap,
        month_index: u32,
    ) -> bool {
        let mut events = Vec::new();
        let mut peaces = Vec::new();
        let mut any_settled = false;

        // 1) Fresh wars from tension (feud / faith / trade / rivalry — not only conquest).
        for edge in &mut self.edges {
            if !edge.at_war && edge.tension >= WAR_TENSION {
                edge.at_war = true;
                edge.war_months = 0;
                edge.kind = RelationKind::BorderFeud;
                if let (Some(a), Some(b)) = (realms.get(edge.realm_a), realms.get(edge.realm_b)) {
                    events.push(format!("Wybuchła wojna między {} a {}.", a.name, b.name));
                }
            }
        }

        // 2) Opportunists may pile onto already-warring neighbors.
        self.try_opportunistic_wars(realms, month_index, &mut events);

        // 3) Age wars; limited conflicts end without needing full occupation.
        for edge in &mut self.edges {
            if !edge.at_war {
                continue;
            }
            edge.war_months = edge.war_months.saturating_add(1);
            let conquest = war_seeks_conquest(realms, edge.realm_a, edge.realm_b);
            let min_m = if conquest {
                WAR_MIN_MONTHS_CONQUEST
            } else {
                WAR_MIN_MONTHS_LIMITED
            };
            let extra = ((month_index
                .wrapping_mul(13)
                .wrapping_add(edge.realm_a as u32 * 7)
                .wrapping_add(edge.realm_b as u32 * 5))
                % 3) as u8;
            if edge.war_months >= min_m + extra {
                peaces.push((edge.realm_a, edge.realm_b));
            }
        }

        for (a, b) in peaces {
            if self.resolve_peace(realms, territory, a, b, month_index, &mut events) {
                any_settled = true;
            }
        }
        if let Some(ev) = events.pop() {
            self.last_event = Some(ev);
        }
        any_settled
    }

    /// Third party sees a distracted neighbor and may declare war (one try / month).
    fn try_opportunistic_wars(
        &mut self,
        realms: &[Realm],
        month_index: u32,
        events: &mut Vec<String>,
    ) {
        let active: Vec<(usize, usize)> = self
            .edges
            .iter()
            .filter(|e| e.at_war)
            .map(|e| (e.realm_a, e.realm_b))
            .collect();
        if active.is_empty() {
            return;
        }

        let mut candidates: Vec<(usize, usize, u32)> = Vec::new();
        for c in 0..self.realm_count {
            let Some(rc) = realms.get(c) else {
                continue;
            };
            for &(a, b) in &active {
                for &target in &[a, b] {
                    if c == target || self.at_war(c, target) {
                        continue;
                    }
                    let Some(edge) = self.edge(c, target) else {
                        continue;
                    };
                    let Some(rt) = realms.get(target) else {
                        continue;
                    };
                    let other = if target == a { b } else { a };
                    let stronger = rc.military_strength as i16 + 8 >= rt.military_strength as i16;
                    let hostile = is_hostile(edge.kind) || edge.tension >= 35;
                    let ambitious = matches!(
                        rc.ambition,
                        RulerAmbition::ExpandBorders
                            | RulerAmbition::DominateTrade
                            | RulerAmbition::EnrichTreasury
                    );
                    let backs_rival = self
                        .kind_between(c, other)
                        .map(is_friendly)
                        .unwrap_or(false);
                    if !(stronger && (hostile || ambitious || backs_rival)) {
                        continue;
                    }
                    let score = month_index
                        .wrapping_mul(23)
                        .wrapping_add(c as u32 * 11)
                        .wrapping_add(target as u32 * 7);
                    if score % 5 != 0 {
                        continue;
                    }
                    candidates.push((c, target, score));
                }
            }
        }
        if candidates.is_empty() {
            return;
        }
        candidates.sort_by_key(|&(c, t, s)| (s, c, t));
        let (c, target, _) = candidates[0];
        let Some(edge) = self
            .edges
            .iter_mut()
            .find(|e| e.realm_a == c.min(target) && e.realm_b == c.max(target))
        else {
            return;
        };
        edge.at_war = true;
        edge.war_months = 0;
        edge.tension = edge.tension.max(WAR_TENSION);
        edge.kind = RelationKind::BorderFeud;
        if let (Some(rc), Some(rt)) = (realms.get(c), realms.get(target)) {
            events.push(format!(
                "{} wykorzystuje okazję i wypowiada wojnę {}.",
                rc.name, rt.name
            ));
        }
    }

    /// Stage C: winner annexes occupied loser cells into `owners`; clear pair claims.
    /// Returns whether any owner cell changed.
    fn resolve_peace(
        &mut self,
        realms: &[Realm],
        territory: &mut TerritoryMap,
        a: usize,
        b: usize,
        month_index: u32,
        events: &mut Vec<String>,
    ) -> bool {
        let Some(ra) = realms.get(a) else {
            return false;
        };
        let Some(rb) = realms.get(b) else {
            return false;
        };

        let jitter = ((month_index.wrapping_mul(19).wrapping_add(a as u32 * 3 + b as u32)) % 11)
            as i16
            - 5;
        let score_a = ra.military_strength as i16 + jitter;
        let score_b = rb.military_strength as i16 - jitter;
        let (winner, loser, wname, lname) = if score_a >= score_b {
            (a, b, ra.name.clone(), rb.name.clone())
        } else {
            (b, a, rb.name.clone(), ra.name.clone())
        };

        if let Some(d) = self.fame_delta.get_mut(winner) {
            *d = (*d + FAME_PEACE).clamp(-50, 50);
        }
        if let Some(d) = self.fame_delta.get_mut(loser) {
            *d = (*d - FAME_PEACE).clamp(-50, 50);
        }

        {
            let Some(edge) = self
                .edges
                .iter_mut()
                .find(|e| e.realm_a == a.min(b) && e.realm_b == a.max(b))
            else {
                return false;
            };
            edge.at_war = false;
            edge.war_months = 0;
            edge.tension = 30;
            edge.kind = kind_from_tension(edge.tension);
        }

        let annexed = annex_occupation(self, territory, winner as u16, loser as u16);
        if annexed > 0 {
            events.push(format!(
                "Ugoda: {wname} anektuje {annexed} komórek od {lname}. Granice prawne ujednolicone."
            ));
        } else {
            events.push(format!(
                "Rozejm: {wname} i {lname} — bez zmiany granic (brak okupacji)."
            ));
        }
        annexed > 0
    }

    fn grow_occupation(
        &mut self,
        realms: &[Realm],
        territory: &TerritoryMap,
        month_index: u32,
    ) {
        let wars: Vec<(usize, usize)> = self
            .edges
            .iter()
            .filter(|e| e.at_war)
            .map(|e| (e.realm_a, e.realm_b))
            .collect();
        for (a, b) in wars {
            let Some(ra) = realms.get(a) else {
                continue;
            };
            let Some(rb) = realms.get(b) else {
                continue;
            };
            // Occupation is optional: only conquest-minded attackers push cells.
            // Limited wars (faith / trade / rivalry) may end with zero annexation.
            let a_conq = ra.ambition == RulerAmbition::ExpandBorders;
            let b_conq = rb.ambition == RulerAmbition::ExpandBorders;
            if !a_conq && !b_conq {
                continue;
            }
            let jitter = ((month_index.wrapping_add(a as u32 * 9 + b as u32 * 4)) % 7) as i16;
            let (attacker, defender) = if a_conq && !b_conq {
                (a, b)
            } else if b_conq && !a_conq {
                (b, a)
            } else if ra.military_strength as i16 + jitter >= rb.military_strength as i16 {
                (a, b)
            } else {
                (b, a)
            };
            try_occupy_one(self, territory, attacker as u16, defender as u16, month_index);
        }
    }

    fn refresh_embargoes(&mut self, realms: &[Realm]) {
        let mut embargoes = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let wars: Vec<(usize, usize)> = self
            .edges
            .iter()
            .filter(|e| e.at_war)
            .map(|e| (e.realm_a, e.realm_b))
            .collect();

        for (a, b) in &wars {
            for c in 0..self.realm_count {
                if c == *a || c == *b {
                    continue;
                }
                let friend_a = self.kind_between(c, *a).map(is_friendly).unwrap_or(false);
                let friend_b = self.kind_between(c, *b).map(is_friendly).unwrap_or(false);
                let hostile_a = self.kind_between(c, *a).map(is_hostile).unwrap_or(false);
                let hostile_b = self.kind_between(c, *b).map(is_hostile).unwrap_or(false);

                if friend_a && !friend_b {
                    push_embargo(&mut embargoes, &mut seen, c, *b);
                }
                if friend_b && !friend_a {
                    push_embargo(&mut embargoes, &mut seen, c, *a);
                }
                // Hostile third party piles on even without an ally.
                if hostile_a && !friend_a {
                    push_embargo(&mut embargoes, &mut seen, c, *a);
                }
                if hostile_b && !friend_b {
                    push_embargo(&mut embargoes, &mut seen, c, *b);
                }
            }
        }

        // Outside war: high tension + trade ambition can embargo a neighbor.
        for edge in &self.edges {
            if edge.at_war || edge.tension < 55 {
                continue;
            }
            let Some(ra) = realms.get(edge.realm_a) else {
                continue;
            };
            let Some(rb) = realms.get(edge.realm_b) else {
                continue;
            };
            if ra.ambition == RulerAmbition::DominateTrade
                || ra.ambition == RulerAmbition::ExpandBorders
            {
                push_embargo(&mut embargoes, &mut seen, edge.realm_a, edge.realm_b);
            }
            if rb.ambition == RulerAmbition::DominateTrade
                || rb.ambition == RulerAmbition::ExpandBorders
            {
                push_embargo(&mut embargoes, &mut seen, edge.realm_b, edge.realm_a);
            }
        }

        embargoes.sort_by_key(|e| (e.from, e.against));
        self.embargoes = embargoes;
    }

    /// Cell indices on a high-tension border (for sparse hatch).
    pub fn tension_border_cells(&self, territory: &TerritoryMap) -> Vec<usize> {
        let cols = territory.cols as usize;
        let rows = territory.rows as usize;
        if cols == 0 || rows == 0 || territory.owners.len() != cols * rows {
            return Vec::new();
        }
        let tense: std::collections::HashSet<(usize, usize)> = self
            .edges
            .iter()
            .filter(|e| e.tension >= 40 || e.at_war)
            .map(|e| (e.realm_a, e.realm_b))
            .collect();
        if tense.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let owners = &territory.owners;
        for y in 0..rows {
            for x in 0..cols {
                let i = y * cols + x;
                let o = owners[i];
                if o == u16::MAX {
                    continue;
                }
                for (nx, ny) in [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ] {
                    if nx >= cols || ny >= rows {
                        continue;
                    }
                    let j = ny * cols + nx;
                    let n = owners[j];
                    if n == u16::MAX || n == o {
                        continue;
                    }
                    let lo = o.min(n) as usize;
                    let hi = o.max(n) as usize;
                    if tense.contains(&(lo, hi)) {
                        out.push(i);
                        break;
                    }
                }
            }
        }
        out
    }
}

fn push_embargo(
    out: &mut Vec<Embargo>,
    seen: &mut std::collections::HashSet<(usize, usize)>,
    from: usize,
    against: usize,
) {
    if from == against || !seen.insert((from, against)) {
        return;
    }
    out.push(Embargo { from, against });
}

fn is_friendly(kind: RelationKind) -> bool {
    matches!(
        kind,
        RelationKind::AncientAlliance | RelationKind::TradePact
    )
}

fn is_hostile(kind: RelationKind) -> bool {
    matches!(kind, RelationKind::Rivalry | RelationKind::BorderFeud)
}

fn war_seeks_conquest(realms: &[Realm], a: usize, b: usize) -> bool {
    realms
        .get(a)
        .map(|r| r.ambition == RulerAmbition::ExpandBorders)
        .unwrap_or(false)
        || realms
            .get(b)
            .map(|r| r.ambition == RulerAmbition::ExpandBorders)
            .unwrap_or(false)
}

/// Winner keeps cells they occupy on loser's de jure land; clear all pair occupation.
fn annex_occupation(
    map: &mut RealmDiplomacyMap,
    territory: &mut TerritoryMap,
    winner: u16,
    loser: u16,
) -> u32 {
    if map.occupation.len() != territory.owners.len() {
        return 0;
    }
    let mut annexed = 0u32;
    for i in 0..map.occupation.len() {
        let occ = map.occupation[i];
        if occ == u16::MAX {
            continue;
        }
        if occ != winner && occ != loser {
            continue;
        }
        if occ == winner && territory.owners[i] == loser {
            territory.owners[i] = winner;
            annexed += 1;
        }
        map.occupation[i] = u16::MAX;
    }
    annexed
}

fn try_occupy_one(
    map: &mut RealmDiplomacyMap,
    territory: &TerritoryMap,
    attacker: u16,
    defender: u16,
    month_index: u32,
) {
    let cols = territory.cols as usize;
    let rows = territory.rows as usize;
    if cols == 0 || rows == 0 || map.occupation.len() != territory.owners.len() {
        return;
    }
    let owners = &territory.owners;
    let mut candidates = Vec::new();
    for y in 0..rows {
        for x in 0..cols {
            let i = y * cols + x;
            if owners[i] != defender {
                continue;
            }
            if map.occupation[i] != u16::MAX && map.occupation[i] != attacker {
                continue;
            }
            if map.occupation[i] == attacker {
                continue; // already held
            }
            let mut abut = false;
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx >= cols || ny >= rows {
                    continue;
                }
                let j = ny * cols + nx;
                if owners[j] == attacker || map.occupation[j] == attacker {
                    abut = true;
                    break;
                }
            }
            if abut {
                candidates.push(i);
            }
        }
    }
    if candidates.is_empty() {
        return;
    }
    candidates.sort_unstable();
    let pick = ((month_index as usize)
        .wrapping_mul(31)
        .wrapping_add(attacker as usize * 17)
        .wrapping_add(defender as usize * 13))
        % candidates.len();
    map.occupation[candidates[pick]] = attacker;
}

pub fn tension_from_kind(kind: RelationKind) -> Tension {
    match kind {
        RelationKind::AncientAlliance => -60,
        RelationKind::TradePact => -30,
        RelationKind::ColdPeace => 0,
        RelationKind::Rivalry => 40,
        RelationKind::BorderFeud => 70,
    }
}

pub fn kind_from_tension(tension: Tension) -> RelationKind {
    if tension <= -50 {
        RelationKind::AncientAlliance
    } else if tension <= -15 {
        RelationKind::TradePact
    } else if tension < 25 {
        RelationKind::ColdPeace
    } else if tension < 55 {
        RelationKind::Rivalry
    } else {
        RelationKind::BorderFeud
    }
}

fn has_trade_route(a: &Realm, b_id: usize) -> bool {
    a.trade_routes.iter().any(|r| r.partner_realm_id == b_id)
}

fn ambition_pressure(ambition: RulerAmbition, has_trade: bool, other_expand: bool) -> i16 {
    match ambition {
        RulerAmbition::ExpandBorders => {
            if other_expand {
                6
            } else {
                4
            }
        }
        RulerAmbition::FortifyHomeland => -2,
        RulerAmbition::DominateTrade => {
            if has_trade {
                -2
            } else {
                3
            }
        }
        RulerAmbition::EnrichTreasury => {
            if has_trade {
                -1
            } else {
                1
            }
        }
        RulerAmbition::SecureFaith => 1,
    }
}

fn monthly_delta(a: &Realm, b: &Realm, edge: &DiplomacyEdge, month_index: u32) -> i16 {
    let trade = has_trade_route(a, b.id) || has_trade_route(b, a.id);
    let a_exp = a.ambition == RulerAmbition::ExpandBorders;
    let b_exp = b.ambition == RulerAmbition::ExpandBorders;

    let mut delta = 0i16;
    delta += ambition_pressure(a.ambition, trade, b_exp);
    delta += ambition_pressure(b.ambition, trade, a_exp);

    if a.religion != b.religion
        && (a.ambition == RulerAmbition::SecureFaith || b.ambition == RulerAmbition::SecureFaith)
    {
        delta += 2;
    }

    delta += match edge.kind {
        RelationKind::BorderFeud | RelationKind::Rivalry => 1,
        RelationKind::AncientAlliance | RelationKind::TradePact => -1,
        RelationKind::ColdPeace => 0,
    };

    if edge.at_war {
        delta += 2;
    }

    if a_exp || b_exp {
        let diff = a.military_strength as i16 - b.military_strength as i16;
        if a_exp && diff > 10 {
            delta += 2;
        }
        if b_exp && diff < -10 {
            delta += 2;
        }
    }

    let wobble = ((month_index
        .wrapping_mul(17)
        .wrapping_add(edge.realm_a as u32 * 3)
        .wrapping_add(edge.realm_b as u32 * 11))
        % 3) as i16
        - 1;
    delta + wobble
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::borders::{
        CultureKind, GovernmentKind, ReligionKind, RealmCommodity, RealmRelation, RealmTradeRoute,
    };
    use crate::world::goods::Good;

    fn stub_realm(
        id: usize,
        ambition: RulerAmbition,
        religion: ReligionKind,
        military: u8,
        relations: Vec<RealmRelation>,
        trade_with: Option<usize>,
    ) -> Realm {
        let trade_routes = trade_with
            .map(|partner| {
                vec![RealmTradeRoute {
                    partner_realm_id: partner,
                    good: Good::Grain,
                    exporting: true,
                }]
            })
            .unwrap_or_default();
        Realm {
            id,
            name: format!("R{id}"),
            capital_idx: 0,
            settlement_idxs: vec![0],
            ruler_name: "Władca".into(),
            government: GovernmentKind::Monarchy,
            culture: CultureKind::Agrarian,
            religion,
            ambition,
            fame: 40,
            manpower: 100,
            military_strength: military,
            goals: Vec::new(),
            relations,
            exports: vec![RealmCommodity {
                good: Good::Grain,
                amount: 10,
            }],
            imports: Vec::new(),
            trade_routes,
            laws: Vec::new(),
        }
    }

    fn tiny_territory(owners: Vec<u16>, cols: u32, rows: u32) -> TerritoryMap {
        TerritoryMap {
            cols,
            rows,
            step: 1.0,
            owners,
        }
    }

    #[test]
    fn kind_roundtrip_thresholds() {
        assert_eq!(kind_from_tension(-60), RelationKind::AncientAlliance);
        assert_eq!(kind_from_tension(-30), RelationKind::TradePact);
        assert_eq!(kind_from_tension(0), RelationKind::ColdPeace);
        assert_eq!(kind_from_tension(40), RelationKind::Rivalry);
        assert_eq!(kind_from_tension(70), RelationKind::BorderFeud);
    }

    #[test]
    fn from_realms_dedupes_pairs() {
        let realms = vec![
            stub_realm(
                0,
                RulerAmbition::FortifyHomeland,
                ReligionKind::SolarCult,
                40,
                vec![RealmRelation {
                    other_realm_id: 1,
                    kind: RelationKind::ColdPeace,
                }],
                None,
            ),
            stub_realm(
                1,
                RulerAmbition::FortifyHomeland,
                ReligionKind::SolarCult,
                40,
                vec![RealmRelation {
                    other_realm_id: 0,
                    kind: RelationKind::ColdPeace,
                }],
                None,
            ),
        ];
        let map = RealmDiplomacyMap::from_realms(&realms);
        assert_eq!(map.edges.len(), 1);
        assert_eq!(map.edges[0].tension, 0);
        assert_eq!(map.edges[0].kind, RelationKind::ColdPeace);
    }

    #[test]
    fn expand_borders_raises_tension_over_months() {
        let realms = vec![
            stub_realm(
                0,
                RulerAmbition::ExpandBorders,
                ReligionKind::SolarCult,
                70,
                vec![RealmRelation {
                    other_realm_id: 1,
                    kind: RelationKind::ColdPeace,
                }],
                None,
            ),
            stub_realm(
                1,
                RulerAmbition::ExpandBorders,
                ReligionKind::OldGods,
                30,
                vec![RealmRelation {
                    other_realm_id: 0,
                    kind: RelationKind::ColdPeace,
                }],
                None,
            ),
        ];
        let mut territory = tiny_territory(vec![0, 1], 2, 1);
        let mut map = RealmDiplomacyMap::from_realms(&realms);
        let start = map.edges[0].tension;
        for m in 0..8 {
            map.tick_month(&realms, &mut territory, m);
        }
        assert!(
            map.edges[0].tension > start,
            "expected tension to rise under mutual ExpandBorders"
        );
    }

    #[test]
    fn tick_month_is_deterministic() {
        let realms = vec![
            stub_realm(
                0,
                RulerAmbition::DominateTrade,
                ReligionKind::SeaChurch,
                50,
                vec![RealmRelation {
                    other_realm_id: 1,
                    kind: RelationKind::TradePact,
                }],
                Some(1),
            ),
            stub_realm(
                1,
                RulerAmbition::EnrichTreasury,
                ReligionKind::SeaChurch,
                45,
                vec![RealmRelation {
                    other_realm_id: 0,
                    kind: RelationKind::TradePact,
                }],
                Some(0),
            ),
        ];
        let mut territory_a = tiny_territory(vec![0, 1], 2, 1);
        let mut territory_b = tiny_territory(vec![0, 1], 2, 1);
        let mut a = RealmDiplomacyMap::from_realms(&realms);
        let mut b = RealmDiplomacyMap::from_realms(&realms);
        for m in 0..12 {
            a.tick_month(&realms, &mut territory_a, m);
            b.tick_month(&realms, &mut territory_b, m);
        }
        assert_eq!(a, b);
        assert_eq!(territory_a, territory_b);
    }

    #[test]
    fn high_tension_starts_war_and_blocks_trade() {
        let realms = vec![
            stub_realm(
                0,
                RulerAmbition::ExpandBorders,
                ReligionKind::SolarCult,
                80,
                vec![RealmRelation {
                    other_realm_id: 1,
                    kind: RelationKind::BorderFeud,
                }],
                Some(1),
            ),
            stub_realm(
                1,
                RulerAmbition::FortifyHomeland,
                ReligionKind::OldGods,
                20,
                vec![RealmRelation {
                    other_realm_id: 0,
                    kind: RelationKind::BorderFeud,
                }],
                Some(0),
            ),
        ];
        let mut territory = tiny_territory(vec![0, 1], 2, 1);
        let mut map = RealmDiplomacyMap::from_realms(&realms);
        map.edges[0].tension = WAR_TENSION;
        map.tick_month(&realms, &mut territory, 1);
        assert!(map.edges[0].at_war);
        assert!(map.trade_blocked(0, 1));
    }

    #[test]
    fn war_grows_occupation_without_changing_owners() {
        let realms = vec![
            stub_realm(
                0,
                RulerAmbition::ExpandBorders,
                ReligionKind::SolarCult,
                90,
                vec![RealmRelation {
                    other_realm_id: 1,
                    kind: RelationKind::BorderFeud,
                }],
                None,
            ),
            stub_realm(
                1,
                RulerAmbition::FortifyHomeland,
                ReligionKind::OldGods,
                10,
                vec![RealmRelation {
                    other_realm_id: 0,
                    kind: RelationKind::BorderFeud,
                }],
                None,
            ),
        ];
        // 0 | 1  side by side
        let mut territory = tiny_territory(vec![0, 1], 2, 1);
        let owners_before = territory.owners.clone();
        let mut map = RealmDiplomacyMap::from_realms(&realms);
        map.edges[0].tension = WAR_TENSION;
        map.edges[0].at_war = true;
        map.ensure_occupation_grid(&territory);
        map.tick_month(&realms, &mut territory, 2);
        assert_eq!(territory.owners, owners_before);
        assert!(
            map.occupation.iter().any(|&c| c == 0),
            "stronger side should occupy a defender cell"
        );
    }

    #[test]
    fn peace_annexes_occupation_into_owners() {
        let realms = vec![
            stub_realm(
                0,
                RulerAmbition::ExpandBorders,
                ReligionKind::SolarCult,
                90,
                vec![RealmRelation {
                    other_realm_id: 1,
                    kind: RelationKind::BorderFeud,
                }],
                None,
            ),
            stub_realm(
                1,
                RulerAmbition::FortifyHomeland,
                ReligionKind::OldGods,
                10,
                vec![RealmRelation {
                    other_realm_id: 0,
                    kind: RelationKind::BorderFeud,
                }],
                None,
            ),
        ];
        let mut territory = tiny_territory(vec![0, 1], 2, 1);
        let mut map = RealmDiplomacyMap::from_realms(&realms);
        map.ensure_occupation_grid(&territory);
        map.occupation[1] = 0; // realm 0 occupies realm 1's cell
        map.edges[0].at_war = true;
        map.edges[0].war_months = 10;
        let settled = map.tick_month(&realms, &mut territory, 5);
        assert!(settled);
        assert!(!map.edges[0].at_war);
        assert_eq!(territory.owners[1], 0);
        assert_eq!(map.occupation[1], u16::MAX);
    }

    #[test]
    fn ally_embargoes_enemy_during_war() {
        let realms = vec![
            stub_realm(
                0,
                RulerAmbition::ExpandBorders,
                ReligionKind::SolarCult,
                70,
                vec![
                    RealmRelation {
                        other_realm_id: 1,
                        kind: RelationKind::BorderFeud,
                    },
                    RealmRelation {
                        other_realm_id: 2,
                        kind: RelationKind::AncientAlliance,
                    },
                ],
                None,
            ),
            stub_realm(
                1,
                RulerAmbition::FortifyHomeland,
                ReligionKind::OldGods,
                40,
                vec![
                    RealmRelation {
                        other_realm_id: 0,
                        kind: RelationKind::BorderFeud,
                    },
                    RealmRelation {
                        other_realm_id: 2,
                        kind: RelationKind::ColdPeace,
                    },
                ],
                None,
            ),
            stub_realm(
                2,
                RulerAmbition::DominateTrade,
                ReligionKind::SolarCult,
                50,
                vec![
                    RealmRelation {
                        other_realm_id: 0,
                        kind: RelationKind::AncientAlliance,
                    },
                    RealmRelation {
                        other_realm_id: 1,
                        kind: RelationKind::ColdPeace,
                    },
                ],
                Some(1),
            ),
        ];
        let mut territory = tiny_territory(vec![0, 1, 2], 3, 1);
        let mut map = RealmDiplomacyMap::from_realms(&realms);
        // Keep C friendly with A in live map.
        if let Some(e) = map.edges.iter_mut().find(|e| e.realm_a == 0 && e.realm_b == 2) {
            e.tension = -60;
            e.kind = RelationKind::AncientAlliance;
        }
        map.edges
            .iter_mut()
            .find(|e| e.realm_a == 0 && e.realm_b == 1)
            .unwrap()
            .at_war = true;
        map.tick_month(&realms, &mut territory, 3);
        assert!(
            map.has_embargo(2, 1),
            "ally of A should embargo B during A↔B war"
        );
        assert!(map.trade_blocked(2, 1));
    }

    #[test]
    fn limited_war_does_not_occupy_cells() {
        let realms = vec![
            stub_realm(
                0,
                RulerAmbition::SecureFaith,
                ReligionKind::SolarCult,
                70,
                vec![RealmRelation {
                    other_realm_id: 1,
                    kind: RelationKind::Rivalry,
                }],
                None,
            ),
            stub_realm(
                1,
                RulerAmbition::DominateTrade,
                ReligionKind::OldGods,
                60,
                vec![RealmRelation {
                    other_realm_id: 0,
                    kind: RelationKind::Rivalry,
                }],
                None,
            ),
        ];
        let mut territory = tiny_territory(vec![0, 1], 2, 1);
        let mut map = RealmDiplomacyMap::from_realms(&realms);
        map.edges[0].at_war = true;
        map.ensure_occupation_grid(&territory);
        map.tick_month(&realms, &mut territory, 2);
        assert!(
            map.occupation.iter().all(|&c| c == u16::MAX),
            "wars without ExpandBorders should not push occupation"
        );
    }

    #[test]
    fn opportunist_can_declare_on_belligerent() {
        let realms = vec![
            stub_realm(
                0,
                RulerAmbition::FortifyHomeland,
                ReligionKind::SolarCult,
                40,
                vec![
                    RealmRelation {
                        other_realm_id: 1,
                        kind: RelationKind::BorderFeud,
                    },
                    RealmRelation {
                        other_realm_id: 2,
                        kind: RelationKind::ColdPeace,
                    },
                ],
                None,
            ),
            stub_realm(
                1,
                RulerAmbition::FortifyHomeland,
                ReligionKind::OldGods,
                35,
                vec![
                    RealmRelation {
                        other_realm_id: 0,
                        kind: RelationKind::BorderFeud,
                    },
                    RealmRelation {
                        other_realm_id: 2,
                        kind: RelationKind::Rivalry,
                    },
                ],
                None,
            ),
            stub_realm(
                2,
                RulerAmbition::ExpandBorders,
                ReligionKind::SolarCult,
                90,
                vec![
                    RealmRelation {
                        other_realm_id: 0,
                        kind: RelationKind::ColdPeace,
                    },
                    RealmRelation {
                        other_realm_id: 1,
                        kind: RelationKind::Rivalry,
                    },
                ],
                None,
            ),
        ];
        let mut territory = tiny_territory(vec![0, 1, 2], 3, 1);
        let mut map = RealmDiplomacyMap::from_realms(&realms);
        map.edges
            .iter_mut()
            .find(|e| e.realm_a == 0 && e.realm_b == 1)
            .unwrap()
            .at_war = true;
        // month_index=2 → score for (2→1) divisible by 5 (see try_opportunistic_wars).
        map.tick_month(&realms, &mut territory, 2);
        assert!(
            map.at_war(2, 1) || map.at_war(2, 0),
            "opportunist should open a front against a belligerent"
        );
    }
}
