//! Realm law catalogue and seed-stable generation (stage A).
//! Effects are stored but not yet applied to trade/UI (stage C).

use serde::{Deserialize, Serialize};

use crate::world::borders::{
    CultureKind, GovernmentKind, ReligionKind, Realm, RulerAmbition,
};
use crate::world::building::{BuildingKind, BuildingMap};
use crate::world::goods::Good;
use crate::world::prng::unit_noise;
use crate::world::settlement::{DistrictKind, Settlement, SettlementKind};

/// Dedicated PRNG channel — do not reuse settlement / building / NPC channels.
const CHANNEL_LAW: u64 = 73;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[repr(u8)]
pub enum LawDomain {
    Tax,
    Trade,
    Faith,
    Access,
    Labor,
    Security,
}

impl LawDomain {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tax => "Podatki",
            Self::Trade => "Handel",
            Self::Faith => "Wiara",
            Self::Access => "Dostęp",
            Self::Labor => "Praca",
            Self::Security => "Bezpieczeństwo",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LawStrength {
    Custom,
    Statute,
    Edict,
}

impl LawStrength {
    pub fn label(self) -> &'static str {
        match self {
            Self::Custom => "Zwyczaj",
            Self::Statute => "Ustawa",
            Self::Edict => "Edykt",
        }
    }

    fn capacity_need(self) -> u8 {
        match self {
            Self::Custom => 0,
            Self::Statute => 1,
            Self::Edict => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[repr(u8)]
pub enum LawId {
    MarketTithe,
    MerchantLevy,
    BorderToll,
    ContrabandSalt,
    SacredDayRest,
    TemplePrivilege,
    GuildPriority,
    MusterDuty,
    ClanHospitality,
    AncestralTaboo,
}

impl LawId {
    pub fn label(self) -> &'static str {
        match self {
            Self::MarketTithe => "Dziesięcina rynkowa",
            Self::MerchantLevy => "Opłata kupiecka",
            Self::BorderToll => "Cło graniczne",
            Self::ContrabandSalt => "Zakaz wybranych minerałów",
            Self::SacredDayRest => "Dzień święty",
            Self::TemplePrivilege => "Przywilej świątynny",
            Self::GuildPriority => "Pierwszeństwo cechów",
            Self::MusterDuty => "Obowiązek pospolitego ruszenia",
            Self::ClanHospitality => "Gościnność klanowa",
            Self::AncestralTaboo => "Tabu przodków",
        }
    }

    fn domain(self) -> LawDomain {
        match self {
            Self::MarketTithe | Self::MerchantLevy => LawDomain::Tax,
            Self::BorderToll | Self::ContrabandSalt => LawDomain::Trade,
            Self::SacredDayRest | Self::TemplePrivilege | Self::AncestralTaboo => LawDomain::Faith,
            Self::ClanHospitality => LawDomain::Access,
            Self::GuildPriority => LawDomain::Labor,
            Self::MusterDuty => LawDomain::Security,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LawSource {
    Religion,
    Culture,
    Government,
    Ambition,
}

impl LawSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Religion => "Religia",
            Self::Culture => "Kultura",
            Self::Government => "Ustrój",
            Self::Ambition => "Ambicja",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LawEffect {
    TaxRate { degree: u8 },
    Tariff { degree: u8 },
    GoodBan { good: Good },
    DistrictAccess {
        district: DistrictKind,
        restricted: bool,
    },
    PriceBias { good: Good, higher: bool },
    /// No numeric effect yet — shown in UI later.
    DisplayOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealmLaw {
    pub id: LawId,
    pub domain: LawDomain,
    pub strength: LawStrength,
    pub effect: LawEffect,
    pub sources: Vec<LawSource>,
    pub enforced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCapacity {
    pub tax: u8,
    pub trade: u8,
    pub faith: u8,
    pub access: u8,
    pub labor: u8,
    pub security: u8,
}

impl AdminCapacity {
    pub fn get(self, domain: LawDomain) -> u8 {
        match domain {
            LawDomain::Tax => self.tax,
            LawDomain::Trade => self.trade,
            LawDomain::Faith => self.faith,
            LawDomain::Access => self.access,
            LawDomain::Labor => self.labor,
            LawDomain::Security => self.security,
        }
    }
}

/// Capacity from buildings on the realm's territory (spec §5.3).
pub fn admin_capacity_for_realm(
    realm: &Realm,
    settlements: &[Settlement],
    buildings: &BuildingMap,
) -> AdminCapacity {
    let mut cap = AdminCapacity::default();
    let mut barracks_total = 0u8;
    let mut barracks_in_capital = false;

    for &si in &realm.settlement_idxs {
        let Some(settlement) = settlements.get(si) else {
            continue;
        };
        let is_capital = si == realm.capital_idx;
        let is_cityish = matches!(
            settlement.kind,
            SettlementKind::Town | SettlementKind::City
        );

        for b in buildings.in_settlement(si) {
            match b.kind {
                BuildingKind::Hall => {
                    if is_capital {
                        cap.tax = cap.tax.saturating_add(2);
                        cap.access = cap.access.saturating_add(1);
                    } else if is_cityish {
                        cap.tax = cap.tax.saturating_add(1);
                    }
                }
                BuildingKind::Temple => {
                    cap.faith = cap.faith.saturating_add(1);
                }
                BuildingKind::Dock | BuildingKind::Warehouse => {
                    if is_capital {
                        cap.trade = cap.trade.saturating_add(1);
                    }
                }
                BuildingKind::Barracks => {
                    barracks_total = barracks_total.saturating_add(1);
                    if is_capital {
                        barracks_in_capital = true;
                    }
                }
                BuildingKind::MarketHall => {
                    if is_capital {
                        cap.labor = cap.labor.saturating_add(1);
                    }
                }
                _ => {}
            }
        }
    }

    if realm.government == GovernmentKind::Theocracy {
        cap.faith = cap.faith.saturating_add(1);
    }
    if barracks_in_capital || barracks_total >= 2 {
        cap.security = cap.security.saturating_add(1);
    }
    cap
}

/// Fill `realm.laws` for every realm (overwrites). Deterministic for `(seed, profile, buildings)`.
pub fn fill_realm_laws(
    seed: u64,
    settlements: &[Settlement],
    buildings: &BuildingMap,
    realms: &mut [Realm],
) {
    for realm in realms.iter_mut() {
        let capacity = admin_capacity_for_realm(realm, settlements, buildings);
        let hall_in_capital = buildings
            .in_settlement(realm.capital_idx)
            .iter()
            .any(|b| b.kind == BuildingKind::Hall);
        realm.laws = generate_realm_laws(seed, realm, &capacity, hall_in_capital);
        apply_enforced_law_effects(realm);
    }
}

/// Apply enforced law effects to realm trade data (stage C / E2).
/// Currently: `GoodBan` removes the good from exports and exporting trade routes.
pub fn apply_enforced_law_effects(realm: &mut Realm) {
    let bans: Vec<Good> = realm
        .laws
        .iter()
        .filter(|l| l.enforced)
        .filter_map(|l| match l.effect {
            LawEffect::GoodBan { good } => Some(good),
            _ => None,
        })
        .collect();
    if bans.is_empty() {
        return;
    }
    realm.exports.retain(|c| !bans.contains(&c.good));
    realm
        .trade_routes
        .retain(|r| !(r.exporting && bans.contains(&r.good)));
}

pub fn generate_realm_laws(
    seed: u64,
    realm: &Realm,
    capacity: &AdminCapacity,
    hall_in_capital: bool,
) -> Vec<RealmLaw> {
    let mut candidates: Vec<Candidate> = Vec::new();
    push_unique(&mut candidates, religion_law(realm.religion), LawSource::Religion);
    push_unique(&mut candidates, culture_law(realm.culture), LawSource::Culture);
    push_unique(
        &mut candidates,
        government_law(realm.government),
        LawSource::Government,
    );
    push_unique(
        &mut candidates,
        ambition_law(realm.ambition),
        LawSource::Ambition,
    );

    // Ambition wins conflicts on the same LawId / opposing tax style.
    resolve_ambition_priority(&mut candidates);

    let mut laws: Vec<RealmLaw> = candidates
        .into_iter()
        .map(|c| finalize_law(seed, realm, capacity, hall_in_capital, c))
        .collect();

    // Always keep at least the religion custom if somehow empty.
    if laws.is_empty() {
        let c = religion_law(realm.religion);
        laws.push(finalize_law(
            seed,
            realm,
            capacity,
            hall_in_capital,
            Candidate {
                id: c.id,
                effect: c.effect,
                preferred_strength: c.preferred_strength,
                sources: vec![LawSource::Religion],
            },
        ));
    }

    select_stable(seed, realm.id as u64, laws, 5, 3)
}

struct Candidate {
    id: LawId,
    effect: LawEffect,
    preferred_strength: LawStrength,
    sources: Vec<LawSource>,
}

struct LawSeed {
    id: LawId,
    effect: LawEffect,
    preferred_strength: LawStrength,
}

fn push_unique(out: &mut Vec<Candidate>, seed: LawSeed, source: LawSource) {
    if let Some(existing) = out.iter_mut().find(|c| c.id == seed.id) {
        if !existing.sources.contains(&source) {
            existing.sources.push(source);
        }
        // Keep the stronger preferred strength.
        if strength_rank(seed.preferred_strength) > strength_rank(existing.preferred_strength) {
            existing.preferred_strength = seed.preferred_strength;
            existing.effect = seed.effect;
        }
        return;
    }
    out.push(Candidate {
        id: seed.id,
        effect: seed.effect,
        preferred_strength: seed.preferred_strength,
        sources: vec![source],
    });
}

fn strength_rank(s: LawStrength) -> u8 {
    match s {
        LawStrength::Custom => 0,
        LawStrength::Statute => 1,
        LawStrength::Edict => 2,
    }
}

fn resolve_ambition_priority(candidates: &mut Vec<Candidate>) {
    let amb_id = candidates
        .iter()
        .find(|c| c.sources.contains(&LawSource::Ambition))
        .map(|c| c.id);
    let Some(amb_id) = amb_id else {
        return;
    };
    // EnrichTreasury (MarketTithe) overrides mercantile low-tax levy.
    if amb_id == LawId::MarketTithe {
        candidates.retain(|c| c.id != LawId::MerchantLevy);
    }
}

fn finalize_law(
    seed: u64,
    realm: &Realm,
    capacity: &AdminCapacity,
    hall_in_capital: bool,
    c: Candidate,
) -> RealmLaw {
    let domain = c.id.domain();
    let mut strength = c.preferred_strength;

    // Theocracy upgrades faith customs.
    if domain == LawDomain::Faith && realm.government == GovernmentKind::Theocracy {
        if strength_rank(strength) < strength_rank(LawStrength::Statute) {
            strength = LawStrength::Statute;
        }
    }
    // Monarchy can issue edicts when capacity and capital hall allow.
    if matches!(
        realm.government,
        GovernmentKind::Monarchy | GovernmentKind::Principality
    ) && hall_in_capital
        && capacity.get(domain) >= 2
        && c.sources.contains(&LawSource::Ambition)
    {
        strength = LawStrength::Edict;
    }

    // Downgrade if capacity cannot support preferred strength.
    while strength_rank(strength) > 0 && capacity.get(domain) < strength.capacity_need() {
        strength = match strength {
            LawStrength::Edict => LawStrength::Statute,
            LawStrength::Statute => LawStrength::Custom,
            LawStrength::Custom => LawStrength::Custom,
        };
    }
    if strength == LawStrength::Edict && !hall_in_capital {
        strength = if capacity.get(domain) >= 1 {
            LawStrength::Statute
        } else {
            LawStrength::Custom
        };
    }

    let enforced = capacity.get(domain) >= strength.capacity_need()
        && (strength != LawStrength::Edict || hall_in_capital);

    // Tiny seed-stable degree jitter for tax/tariff when degree not fixed.
    let effect = match c.effect {
        LawEffect::TaxRate { degree } if degree == 0 => LawEffect::TaxRate {
            degree: degree_roll(seed, realm.id as u64, c.id, 1, 3),
        },
        LawEffect::Tariff { degree } if degree == 0 => LawEffect::Tariff {
            degree: degree_roll(seed, realm.id as u64, c.id, 1, 3),
        },
        other => other,
    };

    RealmLaw {
        id: c.id,
        domain,
        strength,
        effect,
        sources: c.sources,
        enforced,
    }
}

fn degree_roll(seed: u64, realm_id: u64, id: LawId, lo: u8, hi: u8) -> u8 {
    let key = realm_id
        .wrapping_mul(1009)
        .wrapping_add(id as u8 as u64)
        .wrapping_add(17);
    let t = unit_noise(seed, CHANNEL_LAW, key);
    let span = (hi - lo + 1) as f64;
    lo + ((t * span).floor() as u8).min(hi - lo)
}

fn select_stable(
    seed: u64,
    realm_id: u64,
    mut laws: Vec<RealmLaw>,
    max_total: usize,
    max_enforced: usize,
) -> Vec<RealmLaw> {
    // Stable order: domain then LawId discriminant.
    laws.sort_by_key(|l| (l.domain as u8, l.id as u8));
    laws.dedup_by_key(|l| l.id);

    let mut enforced_n = 0usize;
    let mut out = Vec::new();
    for law in laws {
        if out.len() >= max_total {
            break;
        }
        if law.enforced {
            if enforced_n >= max_enforced {
                // Keep as unenforced copy rather than drop religion entirely.
                let mut weak = law;
                weak.enforced = false;
                if weak.strength != LawStrength::Custom {
                    weak.strength = LawStrength::Custom;
                }
                out.push(weak);
                continue;
            }
            enforced_n += 1;
        }
        out.push(law);
    }

    // Tie-break drop using PRNG only if somehow over cap after dedup (shouldn't).
    if out.len() > max_total {
        let t = unit_noise(seed, CHANNEL_LAW + 1, realm_id);
        let drop_at = ((t * out.len() as f64).floor() as usize).min(out.len() - 1);
        out.remove(drop_at);
    }
    out
}

fn religion_law(religion: ReligionKind) -> LawSeed {
    match religion {
        ReligionKind::SolarCult => LawSeed {
            id: LawId::SacredDayRest,
            effect: LawEffect::PriceBias {
                good: Good::Grain,
                higher: true,
            },
            preferred_strength: LawStrength::Custom,
        },
        ReligionKind::ForestFaith => LawSeed {
            id: LawId::AncestralTaboo,
            effect: LawEffect::DistrictAccess {
                district: DistrictKind::Forest,
                restricted: true,
            },
            preferred_strength: LawStrength::Custom,
        },
        ReligionKind::AncestorRites => LawSeed {
            id: LawId::AncestralTaboo,
            effect: LawEffect::GoodBan { good: Good::Herbs },
            preferred_strength: LawStrength::Custom,
        },
        ReligionKind::SeaChurch => LawSeed {
            id: LawId::TemplePrivilege,
            effect: LawEffect::Tariff { degree: 1 },
            preferred_strength: LawStrength::Custom,
        },
        ReligionKind::OldGods => LawSeed {
            id: LawId::AncestralTaboo,
            effect: LawEffect::GoodBan { good: Good::Ore },
            preferred_strength: LawStrength::Statute,
        },
    }
}

fn culture_law(culture: CultureKind) -> LawSeed {
    match culture {
        CultureKind::Agrarian => LawSeed {
            id: LawId::MarketTithe,
            effect: LawEffect::TaxRate { degree: 2 },
            preferred_strength: LawStrength::Statute,
        },
        CultureKind::Mercantile => LawSeed {
            id: LawId::MerchantLevy,
            effect: LawEffect::TaxRate { degree: 1 },
            preferred_strength: LawStrength::Statute,
        },
        CultureKind::Martial => LawSeed {
            id: LawId::MusterDuty,
            effect: LawEffect::DisplayOnly,
            preferred_strength: LawStrength::Custom,
        },
        CultureKind::Pastoral => LawSeed {
            id: LawId::ClanHospitality,
            effect: LawEffect::DistrictAccess {
                district: DistrictKind::Residential,
                restricted: false,
            },
            preferred_strength: LawStrength::Custom,
        },
        CultureKind::Scholarly => LawSeed {
            id: LawId::GuildPriority,
            effect: LawEffect::PriceBias {
                good: Good::Tools,
                higher: true,
            },
            preferred_strength: LawStrength::Statute,
        },
    }
}

fn government_law(government: GovernmentKind) -> LawSeed {
    match government {
        GovernmentKind::Monarchy => LawSeed {
            id: LawId::MarketTithe,
            effect: LawEffect::TaxRate { degree: 2 },
            preferred_strength: LawStrength::Statute,
        },
        GovernmentKind::Principality => LawSeed {
            id: LawId::MarketTithe,
            effect: LawEffect::TaxRate { degree: 2 },
            preferred_strength: LawStrength::Custom,
        },
        GovernmentKind::CityRepublic => LawSeed {
            id: LawId::MerchantLevy,
            effect: LawEffect::TaxRate { degree: 1 },
            preferred_strength: LawStrength::Statute,
        },
        GovernmentKind::Theocracy => LawSeed {
            id: LawId::TemplePrivilege,
            effect: LawEffect::DistrictAccess {
                district: DistrictKind::Temple,
                restricted: false,
            },
            preferred_strength: LawStrength::Statute,
        },
        GovernmentKind::ClanFederation => LawSeed {
            id: LawId::ClanHospitality,
            effect: LawEffect::DistrictAccess {
                district: DistrictKind::Residential,
                restricted: false,
            },
            preferred_strength: LawStrength::Custom,
        },
    }
}

fn ambition_law(ambition: RulerAmbition) -> LawSeed {
    match ambition {
        RulerAmbition::ExpandBorders => LawSeed {
            id: LawId::MusterDuty,
            effect: LawEffect::DisplayOnly,
            preferred_strength: LawStrength::Statute,
        },
        RulerAmbition::EnrichTreasury => LawSeed {
            id: LawId::MarketTithe,
            effect: LawEffect::TaxRate { degree: 3 },
            preferred_strength: LawStrength::Edict,
        },
        RulerAmbition::SecureFaith => LawSeed {
            id: LawId::SacredDayRest,
            effect: LawEffect::PriceBias {
                good: Good::Herbs,
                higher: true,
            },
            preferred_strength: LawStrength::Statute,
        },
        RulerAmbition::DominateTrade => LawSeed {
            id: LawId::BorderToll,
            effect: LawEffect::Tariff { degree: 2 },
            preferred_strength: LawStrength::Edict,
        },
        RulerAmbition::FortifyHomeland => LawSeed {
            id: LawId::BorderToll,
            effect: LawEffect::Tariff { degree: 1 },
            preferred_strength: LawStrength::Statute,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::building::{
        ensure_capital_halls, generate as generate_buildings,
    };
    use crate::world::config::WorldConfig;
    use crate::world::settlement::generate as generate_settlements;

    fn laws_for_seed(seed: u64) -> Vec<Vec<RealmLaw>> {
        let map = generate_settlements(WorldConfig {
            seed,
            ..WorldConfig::default()
        });
        let mut buildings = generate_buildings(seed, &map.settlements);
        let mut realms = map.realms.clone();
        let capitals: Vec<usize> = realms.iter().map(|r| r.capital_idx).collect();
        ensure_capital_halls(seed, &map.settlements, &capitals, &mut buildings);
        fill_realm_laws(seed, &map.settlements, &buildings, &mut realms);
        realms.into_iter().map(|r| r.laws).collect()
    }

    #[test]
    fn same_seed_same_laws() {
        let a = laws_for_seed(6);
        let b = laws_for_seed(6);
        assert_eq!(a, b);
        assert!(!a.is_empty());
        for realm_laws in &a {
            assert!(!realm_laws.is_empty());
            assert!(realm_laws.len() <= 5);
            assert!(realm_laws.iter().filter(|l| l.enforced).count() <= 3);
        }
    }

    #[test]
    fn different_seeds_can_differ() {
        let a = laws_for_seed(6);
        let b = laws_for_seed(7);
        // Profiles differ with seed; laws almost certainly differ somewhere.
        assert!(a != b || a.iter().any(|laws| !laws.is_empty()));
    }

    #[test]
    fn enforced_good_ban_removes_export() {
        use crate::world::borders::{
            CultureKind, GovernmentKind, ReligionKind, Realm, RulerAmbition,
        };
        let mut realm = Realm {
            id: 0,
            name: "T".into(),
            capital_idx: 0,
            settlement_idxs: vec![0],
            ruler_name: "W".into(),
            government: GovernmentKind::Monarchy,
            culture: CultureKind::Agrarian,
            religion: ReligionKind::AncestorRites,
            ambition: RulerAmbition::FortifyHomeland,
            fame: 40,
            manpower: 10,
            military_strength: 10,
            goals: Vec::new(),
            relations: Vec::new(),
            exports: vec![
                crate::world::borders::RealmCommodity {
                    good: Good::Herbs,
                    amount: 5,
                },
                crate::world::borders::RealmCommodity {
                    good: Good::Grain,
                    amount: 3,
                },
            ],
            imports: Vec::new(),
            trade_routes: vec![crate::world::borders::RealmTradeRoute {
                partner_realm_id: 1,
                good: Good::Herbs,
                exporting: true,
            }],
            laws: vec![RealmLaw {
                id: LawId::AncestralTaboo,
                domain: LawDomain::Faith,
                strength: LawStrength::Statute,
                effect: LawEffect::GoodBan { good: Good::Herbs },
                sources: vec![LawSource::Religion],
                enforced: true,
            }],
        };
        apply_enforced_law_effects(&mut realm);
        assert!(!realm.exports.iter().any(|c| c.good == Good::Herbs));
        assert!(realm.exports.iter().any(|c| c.good == Good::Grain));
        assert!(realm.trade_routes.is_empty());
    }

    #[test]
    fn capital_hall_grants_tax_capacity() {
        let map = generate_settlements(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let mut buildings = generate_buildings(6, &map.settlements);
        let capitals: Vec<usize> = map.realms.iter().map(|r| r.capital_idx).collect();
        ensure_capital_halls(6, &map.settlements, &capitals, &mut buildings);
        for realm in &map.realms {
            let has_hall = buildings
                .in_settlement(realm.capital_idx)
                .iter()
                .any(|b| b.kind == BuildingKind::Hall);
            assert!(has_hall, "capital of {} must have Hall", realm.name);
            let cap = admin_capacity_for_realm(realm, &map.settlements, &buildings);
            assert!(
                cap.tax >= 1,
                "realm {} tax capacity after Hall guarantee",
                realm.name
            );
        }
    }

}
