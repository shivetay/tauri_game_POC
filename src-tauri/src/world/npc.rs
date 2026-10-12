use serde::{Deserialize, Serialize};

use crate::world::prng::unit_noise;
use crate::world::settlement::{DistrictKind, Settlement, SettlementKind};

/// Dedicated PRNG channel — do not reuse settlement layout/name channels.
const CHANNEL_NPC: u64 = 49;
/// Trait / appearance / skill rolls — must not overlap name/role/count channels (0..2).
const CHANNEL_TRAITS: u64 = CHANNEL_NPC + 10;

/// Sentinel district index for settlement-wide NPCs (no districts).
const SETTLEMENT_WIDE_DI: usize = 0;

const GIVEN_NAMES: &[&str] = &[
    "Antek", "Basia", "Czesław", "Danuta", "Ewa", "Franciszek", "Grażyna", "Henryk",
    "Irena", "Jan", "Kasia", "Lech", "Magda", "Norbert", "Olga", "Piotr", "Ryszard",
    "Stefania", "Tomek", "Urszula", "Witek", "Zofia", "Adam", "Beata", "Darek",
];

const AGE_BANDS: &[&str] = &["młody", "w sile wieku", "starszy"];
const BUILDS: &[&str] = &["szczupły", "krępy", "wysoki", "niski"];
const FEATURES: &[&str] = &[
    "blizna na policzku",
    "siwa broda",
    "bystre oczy",
    "szorstkie dłonie",
    "cicha mowa",
    "głośny śmiech",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LifeGoal {
    Wealth,
    Safety,
    Status,
    Craft,
    Faith,
    Wander,
}

impl LifeGoal {
    pub fn label(self) -> &'static str {
        match self {
            LifeGoal::Wealth => "Wealth",
            LifeGoal::Safety => "Safety",
            LifeGoal::Status => "Status",
            LifeGoal::Craft => "Craft",
            LifeGoal::Faith => "Faith",
            LifeGoal::Wander => "Wander",
        }
    }

    const ALL: [LifeGoal; 6] = [
        LifeGoal::Wealth,
        LifeGoal::Safety,
        LifeGoal::Status,
        LifeGoal::Craft,
        LifeGoal::Faith,
        LifeGoal::Wander,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkillId {
    Bargaining,
    Crafting,
    Farming,
    Fighting,
    Healing,
    Lore,
    Navigation,
    Persuasion,
    Stealth,
    Survival,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcSkill {
    pub id: SkillId,
    pub level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionKind {
    HelpStranger,
    OfferTrade,
    ShareLocalInfo,
    TakeRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Npc {
    pub name: String,
    pub role: String,
    pub settlement_index: usize,
    pub district_index: usize,
    pub courage: u8,
    pub honesty: u8,
    pub kindness: u8,
    pub impulsiveness: u8,
    pub intellect: u8,
    pub ambition: u8,
    pub openness_to_strangers: u8,
    pub life_goal: LifeGoal,
    pub appearance: String,
    pub skills: Vec<NpcSkill>,
}

impl Npc {
    /// Inclination toward a decision in `[0.0, 1.0]` from traits only (no session state).
    pub fn inclination(&self, kind: DecisionKind) -> f32 {
        let c = |v: u8| v as f32 / 100.0;
        let raw = match kind {
            DecisionKind::HelpStranger => {
                0.45 * c(self.kindness) + 0.40 * c(self.openness_to_strangers)
                    - 0.15 * c(self.impulsiveness)
                    + 0.15
            }
            DecisionKind::OfferTrade => {
                0.35 * c(self.ambition)
                    + 0.30 * c(self.openness_to_strangers)
                    + 0.20 * c(self.intellect)
                    + 0.15 * c(self.honesty)
            }
            DecisionKind::ShareLocalInfo => {
                0.40 * c(self.kindness)
                    + 0.35 * c(self.openness_to_strangers)
                    + 0.25 * c(self.honesty)
                    - 0.10 * c(self.ambition)
            }
            DecisionKind::TakeRisk => {
                0.45 * c(self.courage) + 0.30 * c(self.ambition) + 0.25 * c(self.impulsiveness)
                    - 0.15 * c(self.intellect)
            }
        };
        raw.clamp(0.0, 1.0)
    }

    pub fn preview_line(&self) -> String {
        format!(
            "{} — {} · cel:{} · int:{} · amb:{}",
            self.name,
            self.role,
            self.life_goal.label(),
            self.intellect,
            self.ambition
        )
    }
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
                npcs.push(build_npc(
                    seed,
                    key,
                    (*given).to_string(),
                    (*role).to_string(),
                    si,
                    SETTLEMENT_WIDE_DI,
                ));
            }
            continue;
        }
        for (di, district) in settlement.districts.iter().enumerate() {
            let count = district_npc_count(seed, si, di, district.kind);
            for i in 0..count {
                let key = npc_key(si, di, i);
                let given = pick(GIVEN_NAMES, seed, CHANNEL_NPC, key);
                let role = pick(roles_for(district.kind), seed, CHANNEL_NPC + 1, key);
                npcs.push(build_npc(
                    seed,
                    key,
                    (*given).to_string(),
                    (*role).to_string(),
                    si,
                    di,
                ));
            }
        }
    }
    NpcMap { npcs }
}

fn build_npc(
    seed: u64,
    key: u64,
    name: String,
    role: String,
    settlement_index: usize,
    district_index: usize,
) -> Npc {
    Npc {
        name,
        role: role.clone(),
        settlement_index,
        district_index,
        courage: roll_trait(seed, key, 0),
        honesty: roll_trait(seed, key, 1),
        kindness: roll_trait(seed, key, 2),
        impulsiveness: roll_trait(seed, key, 3),
        intellect: roll_trait(seed, key, 4),
        ambition: roll_trait(seed, key, 5),
        openness_to_strangers: roll_trait(seed, key, 6),
        life_goal: roll_life_goal(seed, key, &role),
        appearance: roll_appearance(seed, key),
        skills: roll_skills(seed, key, &role),
    }
}

fn roll_trait(seed: u64, key: u64, slot: u64) -> u8 {
    let t = unit_noise(seed, CHANNEL_TRAITS + slot, key);
    ((t * 101.0).floor() as u8).min(100)
}

fn roll_life_goal(seed: u64, key: u64, role: &str) -> LifeGoal {
    let preferred = preferred_life_goal(role);
    let t = unit_noise(seed, CHANNEL_TRAITS + 7, key);
    if t < 0.55 {
        preferred
    } else {
        let i = ((t - 0.55) / 0.45 * LifeGoal::ALL.len() as f64).floor() as usize;
        LifeGoal::ALL[i.min(LifeGoal::ALL.len() - 1)]
    }
}

fn preferred_life_goal(role: &str) -> LifeGoal {
    match role {
        "Kupiec" | "Handlarz" | "Przekupień" | "Wagomistrz" => LifeGoal::Wealth,
        "Kapłan" | "Mnich" | "Zakrystian" => LifeGoal::Faith,
        "Kowal" | "Cieśla" | "Tkacz" | "Rzemieślnik" => LifeGoal::Craft,
        "Burmistrz" | "Sołtys" | "Urzędnik" | "Szlachcic" | "Zarządca" | "Kapitan" => {
            LifeGoal::Status
        }
        "Wędrowiec" | "Marynarz" | "Przewoźnik" => LifeGoal::Wander,
        _ => LifeGoal::Safety,
    }
}

fn roll_appearance(seed: u64, key: u64) -> String {
    let age = pick(AGE_BANDS, seed, CHANNEL_TRAITS + 8, key);
    let build = pick(BUILDS, seed, CHANNEL_TRAITS + 9, key);
    let feature = pick(FEATURES, seed, CHANNEL_TRAITS + 10, key);
    format!("{age}, {build}, {feature}")
}

fn roll_skills(seed: u64, key: u64, role: &str) -> Vec<NpcSkill> {
    let ids = skills_for_role(role);
    ids.iter()
        .enumerate()
        .map(|(i, &id)| {
            let t = unit_noise(seed, CHANNEL_TRAITS + 11 + i as u64, key);
            let level = (25.0 + t * 76.0).floor() as u8;
            NpcSkill {
                id,
                level: level.min(100),
            }
        })
        .collect()
}

fn skills_for_role(role: &str) -> &'static [SkillId] {
    match role {
        "Kupiec" | "Handlarz" | "Przekupień" | "Wagomistrz" => &[
            SkillId::Bargaining,
            SkillId::Persuasion,
            SkillId::Lore,
        ],
        "Kowal" | "Cieśla" | "Tkacz" | "Rzemieślnik" => {
            &[SkillId::Crafting, SkillId::Bargaining, SkillId::Lore]
        }
        "Chłop" | "Pastuch" | "Gospodarz" => {
            &[SkillId::Farming, SkillId::Survival, SkillId::Crafting]
        }
        "Strażnik" | "Żołnierz" | "Kapitan" | "Wartownik" => {
            &[SkillId::Fighting, SkillId::Persuasion, SkillId::Survival]
        }
        "Kapłan" | "Mnich" | "Zakrystian" => {
            &[SkillId::Lore, SkillId::Healing, SkillId::Persuasion]
        }
        "Leśniczy" | "Myśliwy" | "Zielarz" => {
            &[SkillId::Survival, SkillId::Stealth, SkillId::Healing]
        }
        "Marynarz" | "Przewoźnik" | "Rybak" | "Celnik" => {
            &[SkillId::Navigation, SkillId::Survival, SkillId::Bargaining]
        }
        "Wędrowiec" => &[SkillId::Survival, SkillId::Stealth, SkillId::Navigation],
        "Burmistrz" | "Sołtys" | "Urzędnik" | "Szlachcic" | "Zarządca" => {
            &[SkillId::Persuasion, SkillId::Lore, SkillId::Bargaining]
        }
        "Karczmarz" => &[SkillId::Bargaining, SkillId::Persuasion, SkillId::Lore],
        "Sługa" | "Mieszkaniec" | "Mieszczanin" => {
            &[SkillId::Crafting, SkillId::Persuasion, SkillId::Survival]
        }
        _ => &[SkillId::Survival, SkillId::Persuasion, SkillId::Lore],
    }
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

    fn seed6_npcs() -> NpcMap {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        generate(6, &map.settlements)
    }

    #[test]
    fn same_seed_same_npcs() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let a = generate(6, &map.settlements);
        let b = generate(6, &map.settlements);
        assert_eq!(a.npcs.len(), b.npcs.len());
        for (x, y) in a.npcs.iter().zip(b.npcs.iter()) {
            assert_eq!(x.name, y.name);
            assert_eq!(x.role, y.role);
            assert_eq!(x.settlement_index, y.settlement_index);
            assert_eq!(x.district_index, y.district_index);
            assert_eq!(x.courage, y.courage);
            assert_eq!(x.honesty, y.honesty);
            assert_eq!(x.kindness, y.kindness);
            assert_eq!(x.impulsiveness, y.impulsiveness);
            assert_eq!(x.intellect, y.intellect);
            assert_eq!(x.ambition, y.ambition);
            assert_eq!(x.openness_to_strangers, y.openness_to_strangers);
            assert_eq!(x.life_goal, y.life_goal);
            assert_eq!(x.appearance, y.appearance);
            assert_eq!(x.skills.len(), y.skills.len());
            for (sx, sy) in x.skills.iter().zip(y.skills.iter()) {
                assert_eq!(sx.id, sy.id);
                assert_eq!(sx.level, sy.level);
            }
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

    #[test]
    fn every_npc_has_traits_appearance_goal_and_skills() {
        let npcs = seed6_npcs();
        assert!(!npcs.npcs.is_empty());
        for n in &npcs.npcs {
            assert!(n.courage <= 100);
            assert!(n.honesty <= 100);
            assert!(n.kindness <= 100);
            assert!(n.impulsiveness <= 100);
            assert!(n.intellect <= 100);
            assert!(n.ambition <= 100);
            assert!(n.openness_to_strangers <= 100);
            assert!(!n.appearance.is_empty());
            assert!(n.skills.len() >= 3);
            for s in &n.skills {
                assert!(s.level <= 100);
            }
        }
    }

    #[test]
    fn same_seed_same_inclinations() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let a = generate(6, &map.settlements);
        let b = generate(6, &map.settlements);
        let kinds = [
            DecisionKind::HelpStranger,
            DecisionKind::OfferTrade,
            DecisionKind::ShareLocalInfo,
            DecisionKind::TakeRisk,
        ];
        for (x, y) in a.npcs.iter().zip(b.npcs.iter()) {
            for kind in kinds {
                let ix = x.inclination(kind);
                let iy = y.inclination(kind);
                assert!((ix - iy).abs() < f32::EPSILON);
                assert!((0.0..=1.0).contains(&ix));
            }
        }
    }

    #[test]
    fn different_seed_can_change_inclinations() {
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
        let changed = a.npcs.iter().zip(b.npcs.iter()).any(|(x, y)| {
            (x.inclination(DecisionKind::HelpStranger)
                - y.inclination(DecisionKind::HelpStranger))
            .abs()
                > 1e-6
                || x.intellect != y.intellect
                || x.ambition != y.ambition
        });
        assert!(changed || a.npcs.len() != b.npcs.len());
    }
}
