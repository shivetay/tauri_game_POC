use serde::{Deserialize, Serialize};

use crate::world::building::{
    BuildingKind, BuildingMap, BuildingNeed, BuildingStock, Good, NeedIntent,
};
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

/// Session activity while the NPC is simulated in the player's settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcActivity {
    AtHome,
    TravelToSupply,
    Buying,
    Gathering,
    Selling,
    TravelHome,
    Working,
}

impl NpcActivity {
    pub fn label(self) -> &'static str {
        match self {
            NpcActivity::AtHome => "w domu",
            NpcActivity::TravelToSupply => "w drodze (zaopatrzenie)",
            NpcActivity::Buying => "kupuje",
            NpcActivity::Gathering => "zbiera",
            NpcActivity::Selling => "sprzedaje",
            NpcActivity::TravelHome => "wraca do domu",
            NpcActivity::Working => "pracuje",
        }
    }
}

/// Activity performed on arrival at the supply building.
pub fn activity_at_supply(role: &str, needs: &[BuildingNeed]) -> NpcActivity {
    if needs.iter().any(|n| n.intent == NeedIntent::Buy) {
        return NpcActivity::Buying;
    }
    match role {
        "Myśliwy" | "Leśniczy" | "Zielarz" | "Chłop" | "Pastuch" | "Rybak" => {
            NpcActivity::Gathering
        }
        _ => NpcActivity::Selling,
    }
}

/// Activity while staying at the home building.
pub fn activity_at_home(role: &str) -> NpcActivity {
    match role {
        "Myśliwy" | "Leśniczy" | "Zielarz" | "Chłop" | "Pastuch" | "Rybak" => {
            NpcActivity::Gathering
        }
        "Kowal" | "Cieśla" | "Tkacz" | "Rzemieślnik" | "Karczmarz" | "Kupiec"
        | "Handlarz" | "Przekupień" => NpcActivity::Working,
        _ => NpcActivity::AtHome,
    }
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
    pub needs: Vec<BuildingNeed>,
    pub personal_stock: Vec<BuildingStock>,
    /// Index into `BuildingMap.buildings` (workplace / home).
    pub home_building_index: Option<usize>,
    /// Other building in the same settlement that sells a needed good (logical only).
    pub supply_target_index: Option<usize>,
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

    pub fn preview_line(&self, buildings: &BuildingMap, live_supply: Option<usize>) -> String {
        let home = self
            .home_building_index
            .and_then(|i| buildings.buildings.get(i))
            .map(|b| b.name.as_str())
            .unwrap_or("—");
        let supply_i = live_supply.or(self.supply_target_index);
        let (target, want) = match supply_i.and_then(|i| {
            let b = buildings.buildings.get(i)?;
            let good = primary_buy_good(&self.needs)?;
            Some((b.name.as_str(), good))
        }) {
            Some((name, good)) => (name, good.label()),
            None => ("—", "—"),
        };
        format!(
            "{} — {} · dom:{} · po {}:{} · cel:{}",
            self.name,
            self.role,
            home,
            want,
            target,
            self.life_goal.label()
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
/// Assigns each NPC to a home building and a logical supply target (≠ home).
pub fn generate(seed: u64, settlements: &[Settlement], buildings: &BuildingMap) -> NpcMap {
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
                    buildings,
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
                    buildings,
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
    buildings: &BuildingMap,
) -> Npc {
    let needs = needs_for_role(&role);
    let personal_stock = roll_personal_stock(seed, key, &needs);
    let home_building_index =
        assign_home(buildings, settlement_index, district_index, &role, seed, key);
    let supply_target_index =
        assign_supply_target(buildings, settlement_index, home_building_index, &needs);
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
        needs,
        personal_stock,
        home_building_index,
        supply_target_index,
    }
}

fn needs_for_role(role: &str) -> Vec<BuildingNeed> {
    let mut needs = vec![BuildingNeed {
        good: Good::Food,
        intent: NeedIntent::Buy,
    }];
    match role {
        "Myśliwy" | "Leśniczy" => needs.push(BuildingNeed {
            good: Good::Hides,
            intent: NeedIntent::Sell,
        }),
        "Chłop" | "Pastuch" | "Gospodarz" => needs.push(BuildingNeed {
            good: Good::Grain,
            intent: NeedIntent::Sell,
        }),
        "Kowal" => {
            needs.push(BuildingNeed {
                good: Good::Iron,
                intent: NeedIntent::Buy,
            });
            needs.push(BuildingNeed {
                good: Good::Tools,
                intent: NeedIntent::Sell,
            });
        }
        "Cieśla" | "Tkacz" | "Rzemieślnik" => needs.push(BuildingNeed {
            good: Good::Goods,
            intent: NeedIntent::Sell,
        }),
        "Kupiec" | "Handlarz" | "Przekupień" => needs.push(BuildingNeed {
            good: Good::Goods,
            intent: NeedIntent::Sell,
        }),
        "Rybak" | "Marynarz" => needs.push(BuildingNeed {
            good: Good::Fish,
            intent: NeedIntent::Sell,
        }),
        "Zielarz" => needs.push(BuildingNeed {
            good: Good::Herbs,
            intent: NeedIntent::Sell,
        }),
        "Karczmarz" => needs.push(BuildingNeed {
            good: Good::Food,
            intent: NeedIntent::Sell,
        }),
        _ => {}
    }
    needs
}

fn roll_personal_stock(seed: u64, key: u64, needs: &[BuildingNeed]) -> Vec<BuildingStock> {
    let mut stock = Vec::new();
    for (i, need) in needs.iter().enumerate() {
        if need.intent != NeedIntent::Sell {
            continue;
        }
        let t = unit_noise(seed, CHANNEL_TRAITS + 20 + i as u64, key);
        let amount = (4.0 + t * 17.0).round() as u8;
        stock.push(BuildingStock {
            good: need.good,
            amount: amount.clamp(1, 40),
        });
    }
    stock
}

fn preferred_building_kinds(role: &str) -> &'static [BuildingKind] {
    match role {
        "Kowal" => &[BuildingKind::Smithy, BuildingKind::Warehouse],
        "Cieśla" | "Tkacz" | "Rzemieślnik" => &[BuildingKind::Carpenter, BuildingKind::Smithy],
        "Kupiec" | "Handlarz" | "Przekupień" | "Wagomistrz" => {
            &[BuildingKind::Stall, BuildingKind::MarketHall, BuildingKind::Warehouse]
        }
        "Karczmarz" => &[BuildingKind::Inn, BuildingKind::Hall],
        "Kapłan" | "Mnich" | "Zakrystian" => &[BuildingKind::Temple, BuildingKind::House],
        "Żołnierz" | "Kapitan" | "Wartownik" | "Strażnik" => {
            &[BuildingKind::Barracks, BuildingKind::Hall]
        }
        "Myśliwy" | "Leśniczy" | "Zielarz" => &[BuildingKind::ForestLodge, BuildingKind::House],
        "Rybak" | "Marynarz" | "Przewoźnik" | "Celnik" => {
            &[BuildingKind::Dock, BuildingKind::FishMarket, BuildingKind::Warehouse]
        }
        "Chłop" | "Pastuch" | "Gospodarz" => {
            &[BuildingKind::Farmstead, BuildingKind::Mill, BuildingKind::House]
        }
        "Burmistrz" | "Sołtys" | "Urzędnik" | "Szlachcic" | "Zarządca" => {
            &[BuildingKind::Hall, BuildingKind::House]
        }
        "Sługa" | "Mieszkaniec" | "Mieszczanin" | "Wędrowiec" => {
            &[BuildingKind::House, BuildingKind::Inn, BuildingKind::Warehouse]
        }
        _ => &[BuildingKind::House, BuildingKind::Hall, BuildingKind::Warehouse],
    }
}

fn assign_home(
    buildings: &BuildingMap,
    settlement_index: usize,
    district_index: usize,
    role: &str,
    seed: u64,
    key: u64,
) -> Option<usize> {
    let preferred = preferred_building_kinds(role);
    let indices: Vec<usize> = buildings
        .buildings
        .iter()
        .enumerate()
        .filter(|(_, b)| b.settlement_index == settlement_index)
        .map(|(i, _)| i)
        .collect();
    if indices.is_empty() {
        return None;
    }
    let same_district: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|&i| buildings.buildings[i].district_index == district_index)
        .collect();
    let pool = if same_district.is_empty() {
        indices
    } else {
        same_district
    };
    let ranked: Vec<usize> = {
        let mut matched: Vec<usize> = pool
            .iter()
            .copied()
            .filter(|&i| preferred.contains(&buildings.buildings[i].kind))
            .collect();
        if matched.is_empty() {
            pool
        } else {
            matched.sort_by_key(|&i| {
                preferred
                    .iter()
                    .position(|k| *k == buildings.buildings[i].kind)
                    .unwrap_or(99)
            });
            matched
        }
    };
    let pick_i = (unit_noise(seed, CHANNEL_TRAITS + 30, key) * ranked.len() as f64).floor() as usize;
    Some(ranked[pick_i.min(ranked.len() - 1)])
}

/// Find a building in the settlement that can supply a Buy need (stock > 0, ≠ home).
/// `Food` may be satisfied by `Grain`.
pub fn find_supply_building(
    buildings: &BuildingMap,
    settlement_index: usize,
    home: Option<usize>,
    needs: &[BuildingNeed],
) -> Option<usize> {
    let want = needs
        .iter()
        .find(|n| n.intent == NeedIntent::Buy)
        .map(|n| n.good)?;
    buildings.buildings.iter().enumerate().find_map(|(i, b)| {
        if b.settlement_index != settlement_index {
            return None;
        }
        if Some(i) == home {
            return None;
        }
        if b.stock_for_need(want) == 0 {
            return None;
        }
        Some(i)
    })
}

fn assign_supply_target(
    buildings: &BuildingMap,
    settlement_index: usize,
    home: Option<usize>,
    needs: &[BuildingNeed],
) -> Option<usize> {
    find_supply_building(buildings, settlement_index, home, needs)
}

/// Primary Buy good the NPC is trying to obtain.
pub fn primary_buy_good(needs: &[BuildingNeed]) -> Option<Good> {
    needs
        .iter()
        .find(|n| n.intent == NeedIntent::Buy)
        .map(|n| n.good)
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
    use crate::world::building::generate as generate_buildings;
    use crate::world::config::WorldConfig;
    use crate::world::settlement::generate as generate_settlements;

    fn seed6() -> (NpcMap, BuildingMap) {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let buildings = generate_buildings(6, &map.settlements);
        let npcs = generate(6, &map.settlements, &buildings);
        (npcs, buildings)
    }

    #[test]
    fn same_seed_same_npcs() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let buildings = generate_buildings(6, &map.settlements);
        let a = generate(6, &map.settlements, &buildings);
        let b = generate(6, &map.settlements, &buildings);
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
            assert_eq!(x.home_building_index, y.home_building_index);
            assert_eq!(x.supply_target_index, y.supply_target_index);
            assert_eq!(x.needs, y.needs);
            assert_eq!(x.personal_stock, y.personal_stock);
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
        let a_b = generate_buildings(6, &a_map.settlements);
        let b_b = generate_buildings(7, &b_map.settlements);
        let a = generate(6, &a_map.settlements, &a_b);
        let b = generate(7, &b_map.settlements, &b_b);
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
        let buildings = generate_buildings(6, &map.settlements);
        let npcs = generate(6, &map.settlements, &buildings);
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
        let buildings = generate_buildings(6, &map.settlements);
        let npcs = generate(6, &map.settlements, &buildings);
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
        let (npcs, _) = seed6();
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
            assert!(!n.needs.is_empty());
            for s in &n.skills {
                assert!(s.level <= 100);
            }
        }
    }

    #[test]
    fn every_npc_has_home_and_target_not_home_when_possible() {
        let (npcs, buildings) = seed6();
        assert!(!npcs.npcs.is_empty());
        for n in &npcs.npcs {
            assert!(
                n.home_building_index.is_some(),
                "{} should have a home building",
                n.name
            );
            let home = n.home_building_index.unwrap();
            assert_eq!(
                buildings.buildings[home].settlement_index,
                n.settlement_index
            );
            if let Some(target) = n.supply_target_index {
                assert_ne!(target, home);
                assert_eq!(
                    buildings.buildings[target].settlement_index,
                    n.settlement_index
                );
                let want = n
                    .needs
                    .iter()
                    .find(|need| need.intent == NeedIntent::Buy)
                    .map(|need| need.good)
                    .expect("buy need");
                assert!(
                    buildings.buildings[target].stock_for_need(want) > 0,
                    "target should stock {:?}",
                    want
                );
            }
        }
    }

    #[test]
    fn same_seed_same_inclinations() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let buildings = generate_buildings(6, &map.settlements);
        let a = generate(6, &map.settlements, &buildings);
        let b = generate(6, &map.settlements, &buildings);
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
        let a_b = generate_buildings(6, &a_map.settlements);
        let b_b = generate_buildings(7, &b_map.settlements);
        let a = generate(6, &a_map.settlements, &a_b);
        let b = generate(7, &b_map.settlements, &b_b);
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
