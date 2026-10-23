# Prawa państwowe — plan wdrożenia MVP

Dokument jest **specyfikacją wdrożeniową**: ma wystarczyć, by później napisać działający MVP bez zgadywania modelu, kolejności ani kryteriów sukcesu. Nie opisuje stanu kodu — kod praw jeszcze nie istnieje. Stan państw i granic: [panstwa.md](./panstwa.md), [granice.md](./granice.md). Budynki: [budynki-stan.md](./budynki-stan.md), [budynki.md](./budynki.md).

---

## 1. Cel

Każde państwo (`Realm`) ma **własny, czytelny zestaw praw**. Prawa:

1. **wynikają** z religii, kultury, struktury społecznej (ustroju) i ambicji władzy;
2. są **generowane i egzekwowane przez aparat administracyjny** — budynki i role NPC w obszarze prawa publicznego;
3. dają **wymierny skutek** w osadach należących do państwa (cena, opłata, dostęp, legalność towaru).

**Łańcuch docelowy:**

```
profil Realm (ustrój, kultura, religia, ambicja)
    + budynki administracyjne na terytorium
    → pojemność prawna (co wolno uchwalić / egzekwować)
    → zestaw praw Realm
    → skutki w osadach (handel, dostęp, cła, zlecenia)
```

---

## 2. Założenia (jawne)

| # | Założenie | Uzasadnienie |
| --- | --- | --- |
| A1 | **Struktura społeczna** w MVP = `GovernmentKind` już obecny w `Realm` (monarchia, księstwo, republika miejska, teokracja, związek klanów). Osobna warstwa kast/klas — poza MVP. | Unika nowego systemu demograficznego przed działającymi prawami. |
| A2 | Bazowy zestaw praw jest **deterministyczny** z `(seed, realm_id, profil)`. Ten sam seed → te same prawa. | Zgodność z kontraktem generatora. |
| A3 | Raster granic i przynależność osad z seeda to punkt startu; **zmiana prawa w trakcie gry** to warstwa sesji (jak dyplomacja), nie regeneracja świata. | Spójność z A–E w [plans.md](./plans.md). |
| A4 | Prawo bez budynków administracyjnych **nie działa w pełni**: może istnieć jako zwyczaj / deklaracja, ale egzekucja (kara, cło, zakaz) wymaga pojemności. | Wymusza powiązanie z budynkami — żądanie użytkownika. |
| A5 | MVP **nie** wymaga pełnego handlu gracza ani questów. Wystarczy jeden mierzalny efekt na istniejących danych (np. modyfikator ceny towaru w dossier / panelu osady, albo flaga legalności eksportu). | Minimalny vertical slice. |
| A6 | Osobny kanał PRNG dla praw — **nie** mieszać z kanałami osad / NPC / budynków, żeby nie złamać seeda `6`. | Jak przy granicach i budynkach. |

Jeśli któreś założenie ma się zmienić przed kodem — poprawić ten dokument, nie „dopasowywać po drodze”.

---

## 3. Stan wyjściowy (leverage)

| Element | Stan | Rola dla praw |
| --- | --- | --- |
| `Realm` + profil | w kodzie | źródła prawa: `government`, `culture`, `religion`, `ambition` |
| Cele `goals` | stringi tylko do odczytu | później mogą być spięte z domenami praw; w MVP nie mutować |
| Handel realm | statyczny eksport/import + szlaki | pierwszy konsument skutków prawnych (cło, zakaz towaru) |
| Dyplomacja / wojna | sesja v0 | prawa celne / embargo mogą później czytać `RelationKind`; poza MVP |
| Budynki | `Hall`, `Temple`, `Barracks`, `Warehouse`, `Dock`, … | szkielet aparatu; brakuje typów stricte sądowych / celnych |
| Role NPC | Burmistrz, Urzędnik, Kapłan, Celnik, … | egzekutorzy; bez powiązania z prawem |
| Granice iter. 2.2–2.3 | plan w [granice.md](./granice.md) | ten dokument **rozszerza i zastępuje** zakres 2.2–2.4 jako kanon MVP praw |

---

## 4. Model domenowy (docelowy, wystarczający do MVP)

### 4.1. Encje

```text
LawId          — stabilny identyfikator katalogowy (enum lub &'static str)
LawDomain      — Tax | Trade | Faith | Access | Labor | Security
LawStrength    — Custom (zwyczaj, słaby) | Statute (uchwała) | Edict (edikt władzy)
LawEffect      — jeden konkretny skutek (patrz 4.3)
RealmLaw       — { law_id, domain, strength, effect, source_tags[] }
AdminBuilding  — budynek z rolą administracyjną (patrz §5)
AdminCapacity  — punkty / sloty per domena na Realm (suma z budynków stolicy + kluczowych miast)
```

### 4.2. `RealmLaw` (propozycja pól)

| Pole | Typ (koncepcja) | Opis |
| --- | --- | --- |
| `law_id` | enum katalogowy | np. `MarketTithe`, `SacredDayRest`, `BorderToll` |
| `domain` | `LawDomain` | grupa UI i pojemności |
| `strength` | `LawStrength` | czy wymaga egzekucji budynkiem |
| `effect` | `LawEffect` | jedyny skutek runtime w MVP |
| `sources` | flagi / tagi | które czynniki profilu je „uchwaliły” (Religia, Kultura, Ustrój, Ambicja) |
| `enforced` | bool | `capacity[domain] >= wymóg` — inaczej prawo jest „martwe” (widoczne, bez skutku lub ze skutkiem osłabionym) |

### 4.3. Skutki MVP (`LawEffect`) — zamknięta lista

Żeby MVP było implementowalne, **tylko te efekty**:

| Efekt | Parametr | Gdzie widać skutek |
| --- | --- | --- |
| `TaxRate` | `0..=3` (stopnie, nie procenty float) | panel osady / dossier: „podatek rynkowy” |
| `Tariff` | `0..=3` na szlakach / przy przekroczeniu granicy (flaga) | handel realm lub przyszłe cło gracza |
| `GoodBan` | jeden `Good` zakazany w eksporcie lub sprzedaży | lista eksportu / legalność w dossier |
| `DistrictAccess` | utrudniony dostęp do jednej `DistrictKind` (np. `Temple`, `Noble`) | tekst w panelu osady; później reputacja |
| `PriceBias` | `Good` + kierunek drożej/taniej o 1 stopień | gdy pojawi się handel gracza — już dziś można pokazać w UI jako „prawo rynkowe” |

**Zasada:** jedno prawo = jeden efekt. Łączenie efektów = osobne wpisy w katalogu, nie „dzikie” OR-y.

### 4.4. Katalog praw startowych (MVP — 8–12 pozycji)

Katalog jest **wspólny dla świata**; każde państwo wybiera podzbiór według profilu i pojemności.

| `law_id` | Domena | Typowe źródła | Efekt |
| --- | --- | --- | --- |
| `MarketTithe` | Tax | Kultura Agrarna / Ambicja EnrichTreasury / Ustrój Monarchia | `TaxRate` |
| `MerchantLevy` | Tax | Kultura Kupiecka / Republika miejska | `TaxRate` (niższy) lub `Tariff` |
| `BorderToll` | Trade | Ambicja DominateTrade / FortifyHomeland | `Tariff` |
| `ContrabandSalt` | Trade | Religia / Kultura (przykład zakazu) | `GoodBan(Salt)` — lub inny `Good` z katalogu |
| `SacredDayRest` | Faith | Religia (wszystkie) + Teokracja wzmacnia | `PriceBias` na wybrany towar „świąteczny” lub flaga dnia |
| `TemplePrivilege` | Faith | Teokracja / SecureFaith | `DistrictAccess(Temple)` otwarty dla wiernych; później opłata dla obcych |
| `GuildPriority` | Labor | Kultura Kupiecka / Scholarly | `PriceBias` na towary Craft |
| `MusterDuty` | Security | Kultura Martial / ExpandBorders / FortifyHomeland | tekst + ewentualnie bias do manpower (tylko display w MVP) |
| `ClanHospitality` | Access | Związek klanów / Pastoral | `DistrictAccess` złagodzony dla podróżnych |
| `AncestralTaboo` | Faith | AncestorRites / OldGods | `GoodBan` lub `DistrictAccess(Forest)` |

Dokładne mapowanie „profil → które `law_id`” jest w §6 (macierze). Nazwy PL w UI: osobna tabela etykiet przy implementacji (jak `GovernmentKind::label`).

---

## 5. Aparat administracyjny (budynki → prawa)

### 5.1. Idea

**Administracja nie jest abstrakcyjnym mnożnikiem** — to budynki na terytorium państwa, które:

- **generują pojemność** (`AdminCapacity`) w domenach;
- **obsadzają** role egzekucyjne (NPC);
- **lokalizują** miejsce, w którym gracz później „czyta prawo” lub płaci cło.

Bez budynków państwo może mieć co najwyżej 1 prawo typu `Custom` (zwyczaj z religii/kultury), bez `Tariff` / `GoodBan` egzekwowanych.

### 5.2. Funkcje administracyjne (warstwa logiczna)

| Funkcja | Domena praw | Budynki (obecne → docelowe) | Role NPC |
| --- | --- | --- | --- |
| **Władza lokalna** | Tax, Access | `Hall` (jest) → Ratusz / kancelaria w stolicy | Burmistrz, Urzędnik |
| **Skarb / waga** | Tax, Trade | Waga miejska (plan) / `Warehouse` częściowo | Wagomistrz, Urzędnik |
| **Kult** | Faith | `Temple` (jest) → świątynia właściwa religii | Kapłan, Mnich |
| **Granica / cło** | Trade | `Dock` + przyszła **Komora celna**; posterunki przy granicy | Celnik |
| **Sąd / edykty** | wszystkie (egzekucja) | brak → **Sąd / izba wyroków** (nowy typ, po MVP-0) | przyszły Sędzia; MVP: Urzędnik |
| **Siła** | Security | `Barracks` | Kapitan, Strażnik |

### 5.3. Pojemność (`AdminCapacity`) — reguła MVP

Dla każdego `Realm` po wygenerowaniu budynków:

```text
capacity[Tax]      = f( Hall w stolicy + Hall w miastach )
capacity[Faith]    = f( Temple na terytorium, bonus jeśli government == Theocracy )
capacity[Trade]    = f( Dock | Warehouse | przyszła Customs w osadach przy granicy/porcie )
capacity[Security] = f( Barracks )
capacity[Access]   = f( Hall )  // te same budynki co Tax na start
capacity[Labor]    = f( MarketHall | Stall w stolicy )  // cechy v0
```

**Przelicznik startowy (prosty, całkowity):**

- stolica: `Hall` → +2 Tax, +1 Access;
- każde inne miasto (`Town`/`City`) z `Hall` → +1 Tax;
- każdy `Temple` → +1 Faith (+1 ekstra w teokracji);
- port/`Dock` lub `Warehouse` w stolicy → +1 Trade;
- `Barracks` w stolicy lub ≥2 na terytorium → +1 Security;
- `MarketHall` w stolicy → +1 Labor.

**Wymogi egzekucji:**

| Strength | Wymóg |
| --- | --- |
| `Custom` | `capacity[domain] >= 0` (zawsze „jest”, skutek osłabiony ×½ lub tylko tekst) |
| `Statute` | `capacity[domain] >= 1` |
| `Edict` | `capacity[domain] >= 2` **oraz** budynek władzy w stolicy (`Hall`) |

Państwo z samymi wsiami bez `Hall` w stolicy: maksymalnie zwyczaje religijne/kulturowe.

### 5.4. Generacja budynków administracyjnych (kolejność względem praw)

Prawa **nie mogą** być liczone przed budynkami, jeśli egzekucja zależy od pojemności.

**Pipeline:**

```text
1. Terrain + settlements + districts
2. Buildings (istniejące)
2b. [MVP-1] Uzupełnienie puli administracyjnej zależne od Realm
      — gwarantowany Hall w stolicy (jeśli brak)
      — Temple w stolicy lub mieście z dzielnicą Temple (jeśli religia „mocna”)
      — opcjonalnie Customs w porcie stolicy (nowy BuildingKind — etap B)
3. Realms (już są) — profil znany
4. AdminCapacity z budynków na settlement_idxs realm'u
5. RealmLaws z profilu + capacity
6. Sesja: ewolucja napięć / wojen (bez zmiany praw w v0)
```

**Determinizm:** krok 2b używa `CHANNEL_ADMIN` / `CHANNEL_LAW` (nowe stałe), nigdy kanałów settlement/NPC.

### 5.5. Minimalny zestaw budynków „dla prawa” per państwo (MVP)

| Priorytet | Budynek | Gdzie | Po co |
| --- | --- | --- | --- |
| P0 | `Hall` | stolica — **gwarantowany** | Tax/Access/Edict |
| P0 | `Temple` | stolica lub największe miasto z dzielnicą Temple | Faith |
| P1 | Komora celna (`Customs`) — nowy kind | stolica z Port **lub** osada przy granicy realm | Trade / Tariff |
| P1 | Waga / kancelaria skarbowa | Center stolicy (może być flaga na `Hall`, bez nowego mesha) | Tax |
| P2 | Sąd | Noble/Center stolicy | pełna egzekucja kar (po MVP) |

W MVP **wolno** reprezentować „kancelarię” jako wariant nazwy `Hall` (`name` z seeda), bez nowego mesha — byle pojemność była policzalna.

---

## 6. Generacja praw z profilu

### 6.1. Algorytm (pseudokod)

```text
fn generate_realm_laws(seed, realm, capacity) -> Vec<RealmLaw>:
    candidates = []

    // 1) religia — zawsze 1 kandydat Faith (Custom lub Statute)
    candidates += religion_laws(realm.religion)

    // 2) kultura — 1 kandydat Tax/Trade/Labor
    candidates += culture_laws(realm.culture)

    // 3) ustrój — 1 kandydat Access/Tax/Faith
    candidates += government_laws(realm.government)

    // 4) ambicja — 1 kandydat Trade/Security/Tax/Faith
    candidates += ambition_laws(realm.ambition)

    // 5) deterministyczny wybór i wzmocnienie
    for c in candidates:
        c.strength = base_strength(c)
        if capacity[c.domain] >= need(c.strength):
            c.enforced = true
        else:
            c.enforced = false  // pozostaje w liście jako zwyczaj / martwe prawo
        // opcjonalnie: upgrade Custom→Statute gdy capacity wysoka i teokracja/ambicja zgodna

    // 6) limit MVP: max 3 egzekwowane + dowolna liczba nieegzekwowanych Custom (cap 5 łącznie)
    return select_stable(seed, realm.id, candidates, max_total=5, max_enforced=3)
```

### 6.2. Macierz religia → prawa

| `ReligionKind` | Prawo bazowe | Efekt |
| --- | --- | --- |
| `SolarCult` | `SacredDayRest` | `PriceBias` / flaga dnia |
| `ForestFaith` | `AncestralTaboo` (las) | `DistrictAccess(Forest)` lub ban na nadmierny wyrąb (tekst + przyszła ekologia) |
| `AncestorRites` | `AncestralTaboo` | `GoodBan` lekki lub Access |
| `SeaChurch` | `TemplePrivilege` + bias morski | `Tariff` niższy na ryby / wyższy na obcych |
| `OldGods` | `AncestralTaboo` mocniejszy | `GoodBan` lub Faith Edict w teokracji |

### 6.3. Macierz kultura → prawa

| `CultureKind` | Prawo | Efekt |
| --- | --- | --- |
| `Agrarian` | `MarketTithe` | `TaxRate` średni |
| `Mercantile` | `MerchantLevy` + szansa `BorderToll` | niski podatek, wyższe cło |
| `Martial` | `MusterDuty` | Security (display) |
| `Pastoral` | `ClanHospitality` | Access złagodzony |
| `Scholarly` | `GuildPriority` | `PriceBias` Craft |

### 6.4. Macierz ustrój → prawa / mnożniki

| `GovernmentKind` | Wpływ |
| --- | --- |
| `Monarchy` | łatwiej `Edict`; preferuje `MarketTithe` |
| `Principality` | jak monarchia, mniejsza pojemność (mniej miast) — naturalnie mniej Statute |
| `CityRepublic` | preferuje `MerchantLevy`, `GuildPriority`; Edict rzadszy |
| `Theocracy` | +1 Faith capacity; `TemplePrivilege` / `SacredDayRest` → Statute/Edict |
| `ClanFederation` | preferuje `ClanHospitality`; słabsze Tax |

### 6.5. Macierz ambicja → prawa

| `RulerAmbition` | Prawo | Efekt |
| --- | --- | --- |
| `ExpandBorders` | `MusterDuty` | Security |
| `EnrichTreasury` | `MarketTithe` lub podbicie `TaxRate` | Tax |
| `SecureFaith` | wzmocnienie prawa religijnego | Faith Statute |
| `DominateTrade` | `BorderToll` | Tariff |
| `FortifyHomeland` | `BorderToll` lekki + `MusterDuty` | Trade/Security |

### 6.6. Konflikty i priorytety

Gdy dwa źródła pchają sprzeczne efekty (np. niski podatek kupiecki vs ambicja skarbca):

1. **Ambicja władcy** wygrywa przy `Edict` (jeśli jest `Hall` i capacity).
2. Inaczej wygrywa **ustrój** (struktura).
3. Religia nie obniża podatków kupieckich — może dodać osobny zakaz towaru.
4. Przy remisie — stabilny wybór z PRNG(`seed`, `realm_id`, `law_slot`).

---

## 7. Wpływ na inne systemy

| System | MVP | Później |
| --- | --- | --- |
| Dossier państwa (`app.rs`) | lista praw: nazwa, domena, egzekwowane? | historia zmian |
| Panel osady | „obowiązujące prawo państwa X”: 1–2 skutki | reputacja / kary |
| Handel realm | filtr `GoodBan` na eksporcie; adnotacja `Tariff` na szlakach | mutacja cen w sesji |
| Dyplomacja | brak | embargo czyta prawa Trade |
| NPC | brak | Celnik / Urzędnik odmawia usługi gdy ban |
| Gracz | brak wymogu | opłata przy granicy, dostęp do dzielnicy |
| Questy | brak | złamanie prawa → zlecenie / grzywna |

---

## 8. Etapy wdrożenia (od dokumentu do działającego MVP)

Każdy etap ma **kryterium weryfikacji**. Nie przechodzić dalej bez spełnienia.

### Etap 0 — Dokument i kontrakt (ten plik)

- [x] Specyfikacja modelu, macierzy, budynków, pipeline.
- [x] Akceptacja założeń A1–A6 (wdrożenie etapu A).
- **Verify:** można wskazać max 3 prawa egzekwowane i 1 budynek P0 bez dopytywania o model.

### Etap A — Dane i generacja (bez UI skutków) — **DONE**

**Zakres:**

- Nowy moduł `world/realm_laws.rs` (nazwa robocza).
- Katalog `LawId` + generacja `Vec<RealmLaw>` na `Realm`.
- Pojemność z **istniejących** budynków (`Hall`, `Temple`, `Barracks`, `Dock`/`Warehouse`).
- Gwarancja `Hall` w stolicy (minimalna ingerencja w `building.rs`).
- Testy determinizmu: ten sam seed → te same `law_id` i `enforced`.

**Pliki (oczekiwane):**

| Plik | Zmiana |
| --- | --- |
| `src-tauri/src/world/realm_laws.rs` | nowy |
| `src-tauri/src/world/mod.rs` | rejestracja |
| `src-tauri/src/world/borders.rs` lub miejsce składania `Realm` | wywołanie po buildings |
| `src-tauri/src/world/building.rs` | gwarancja Hall stolicy; ewentualnie nazwy |
| `src-tauri/src/world/settlement.rs` | kolejność generacji jeśli wymagana |
| testy w `realm_laws.rs` | determinizm |

**Nie ruszać:** dyplomacji, renderu granic, ekonomii gracza.

**Verify:** `cargo test --lib`; seed `6` bez zmiany profilu kształtu; dossier może jeszcze nie pokazywać praw.

### Etap B — Aparat (budynki administracyjne v1)

**Zakres:**

- Opcjonalnie `BuildingKind::Customs` (komora celna) w porcie stolicy / przy granicy — tylko jeśli etap A pokazał brak Trade capacity w typowych seedach.
- Spis funkcji budynku → `AdminCapacity` w jednym miejscu (tabela w kodzie = tabela §5.3).
- Role: bez nowej AI; wystarczy, że budynki istnieją i liczą się do capacity.

**Verify:** dla seeda `6` każde państwo z miastem ma `capacity[Tax] >= 1` jeśli ma stolicę z Hall; teokracja ma wyższy Faith niż monarchia przy tej samej liczbie świątyń (albo równy, gdy brak Temple — wtedy dopisać gwarancję Temple).

### Etap C — Skutki minimalne (MVP playable w podglądzie)

**Zakres:**

- Podpięcie **jednego** efektu twardego: np. `GoodBan` filtruje `realm.exports` **albo** `Tariff` adnotuje `trade_routes`.
- Drugi efekt miękki: `TaxRate` tylko w UI.
- UI: dossier państwa — sekcja „Prawa”; panel osady — 1 linia „Prawo: …”.

**Verify:**

1. Seed `6` → otwórz dossier ≥1 państwa → widać 1–5 praw z oznaczeniem egzekwowane/nie.
2. Jeśli prawo `GoodBan` egzekwowane — towar znika z eksportu lub jest oznaczony jako zakazany.
3. Regeneracja z tym samym seedem → identyczna lista.

### Etap D — Sesja (poza ścisłym MVP, zaplanowane)

- Zmiana prawa co rok / po wydarzeniu (ambicja, wojna, przewrót) jako delta sesji.
- Egzekucja przy przekroczeniu granicy przez gracza.
- Kary i reputacja.

**Verify:** save/load (gdy save powstanie) odtwarza delty praw.

---

## 9. Definicja MVP (Definition of Done)

MVP uznajemy za ukończone, gdy **łącznie**:

1. Każde `Realm` ma wygenerowany, deterministyczny zestaw 1–5 praw z tagami źródeł (religia/kultura/ustrój/ambicja).
2. Egzekucja zależy od budynków administracyjnych (przynajmniej `Hall` + `Temple` w logice capacity).
3. Co najmniej **jeden** skutek zmienia dane widoczne poza samym tekstem prawa (eksport, szlak lub cena/stopień podatku w UI powiązany z danymi).
4. UI dossier (i idealnie panel osady) pokazuje prawa.
5. `cargo test --lib` przechodzi; seed `6` = Ellipse; brak regresji profili `seed % 6`.
6. Zaktualizowane: ten dokument (sekcja „Stan w kodzie”), [panstwa.md](./panstwa.md), [aktualny_stan_prac.md](./aktualny_stan_prac.md).

**Świadomie poza MVP:** pełny kodeks, procesy sądowe, głosowania w republice, zmiana ustroju, AI piszące prawa, kary więzienia, multi-efektowe ustawyy, wpływ na ścieżkę questów.

---

## 10. Szkic API (dla implementacji)

Tylko kontrakt — nie implementować w tym zadaniu dokumentacyjnym.

```rust
// world/realm_laws.rs (szkic)
pub enum LawDomain { Tax, Trade, Faith, Access, Labor, Security }
pub enum LawStrength { Custom, Statute, Edict }
pub enum LawId { MarketTithe, /* … */ }

pub struct LawEffect { /* enum TaxRate | Tariff | GoodBan | … */ }

pub struct RealmLaw {
    pub id: LawId,
    pub domain: LawDomain,
    pub strength: LawStrength,
    pub effect: LawEffect,
    pub enforced: bool,
    // sources as bitflags or small vec of SourceKind
}

pub struct AdminCapacity { /* per-domain u8 */ }

pub fn admin_capacity_for_realm(
    realm: &Realm,
    settlements: &[Settlement],
    buildings: &[Building],
) -> AdminCapacity;

pub fn generate_realm_laws(
    seed: u64,
    realm: &Realm,
    capacity: &AdminCapacity,
) -> Vec<RealmLaw>;
```

Wywołanie: po buildings, przy finalizacji `Realm` / `SettlementMap` (jedno miejsce — uniknąć podwójnego PRNG).

---

## 11. Ryzyka

| Ryzyko | Mitygacja |
| --- | --- |
| Zmiana kolejności PRNG łamie świat | osobne kanały; gwarancja Hall tylko gdy brak — ostrożnie z jitterem pozycji |
| Państwa bez miast → puste prawa | zawsze ≥1 Custom z religii |
| Zbyt wiele praw nieczytelne | cap 5; UI grupuje po domenie |
| Duplikacja z celami `goals` | goals zostają narracją; prawa = mechanika; ewentualnie później generować goals z LawId |
| Customs za wcześnie | etap A bez nowego kind; etap B tylko jeśli Trade capacity ≈ 0 |

---

## 12. Mapowanie na istniejące plany

| Dokument | Relacja |
| --- | --- |
| [panstwa.md](./panstwa.md) | prawa jako dostawca reguł dla osad; ten plik = szczegół wdrożenia |
| [granice.md](./granice.md) § Iteracja 2.2–2.4 | zastąpione / uszczegółowione przez etapy A–C tutaj |
| [budynki.md](./budynki.md) / [profesje-budynki.md](./profesje-budynki.md) | Ratusz, komora celna, świątynia = aparat |
| [plans.md](./plans.md) filar D / polityka lokalna | prawa realm = poziom powyżej cechów miejskich |
| [plan-zaleznosci-swiata.md](./plan-zaleznosci-swiata.md) | wpływ polityczny gracza — po MVP |

---

## 13. Checklist implementacyjna (do odhaczania przy kodzie)

```
Etap A
- [x] CHANNEL_LAW / CHANNEL_ADMIN
- [x] LawId, LawDomain, LawStrength, LawEffect, RealmLaw
- [x] admin_capacity_for_realm
- [x] generate_realm_laws + macierze §6
- [x] podpięcie do Realm (pole `laws: Vec<RealmLaw>`)
- [x] gwarancja Hall w stolicy
- [x] testy determinizmu (min. 2 seedy + seed 6)

Etap B
- [ ] (opc) BuildingKind::Customs + pula Port/Center
- [ ] capacity Trade z Customs
- [ ] test: teokracja vs monarchia (Faith)

Etap C
- [x] aplikacja GoodBan na eksporcie/szlakach (`apply_enforced_law_effects`)
- [x] UI dossier — sekcja Prawa
- [x] UI osada — Państwo + Prawo przy kliku
- [x] aktualizacja docs (aneksja E1+E2)
```

---

## 14. Stan w kodzie

**Etap A + C (MVP skutków)** (23.10.2026). Aneksja osad: [aneksja-osad.md](./aneksja-osad.md) E1+E2.

| Element | Miejsce |
| --- | --- |
| Typy + generacja + capacity | `src-tauri/src/world/realm_laws.rs` |
| `Realm.laws` | `borders.rs` |
| Gwarancja `Hall` w stolicy | `building::ensure_capital_halls` (tylko przy świeżej generacji budynków) |
| Pipeline | `session::generate_job` → buildings → (halls) → trade → **laws** (+ GoodBan) → npcs |
| UI | dossier „Prawa”; klik osady → Państwo + Prawo |
| Testy | m.in. `enforced_good_ban_removes_export`, `same_seed_same_laws` |

Świadomie niezaimplementowane: `ContrabandSalt` w katalogu (rezerwa), `Tariff` jako mutacja cen, Customs (etap B).
