# NPC — cechy i skłonności (etap 1)

Dokument opisuje to, co jest wdrożone w kodzie, oraz kolejne kroki i sugestie. Nie zastępuje ogólnego planu zależności w [npc.md](./npc.md).

## Co jest wdrożone

- Deterministyczna generacja NPC nadal wiąże osobę z osadą/dzielnicą: `name`, `role` (zawód), `settlement_index`, `district_index`.
- Każdy NPC ma skale `u8` 0–100: `courage`, `honesty`, `kindness`, `impulsiveness`, `intellect`, `ambition`, `openness_to_strangers`.
- Cel życia: enum `LifeGoal` (`Wealth`, `Safety`, `Status`, `Craft`, `Faith`, `Wander`), z preferencją zależną od zawodu i losem z seeda.
- Wygląd: krótki opis tekstowy (`appearance`) z wieku, budowy i wyróżnika.
- Umiejętności: co najmniej trzy wpisy `NpcSkill` (`SkillId` + poziom 0–100), zestaw zależny od zawodu.
- Cienkie API decyzji: `Npc::inclination(DecisionKind) -> f32` w `[0, 1]` wyłącznie z cech (bez reputacji i stanu sesji).
- `DecisionKind`: `HelpStranger`, `OfferTrade`, `ShareLocalInfo`, `TakeRisk`.
- Potrzeby z roli (`needs`: Buy/Sell + Good) oraz minimalny `personal_stock` dla Sell.
- Przypisanie do budynku (`home_building_index`) i cel zaopatrzenia (`supply_target_index` ≠ home).
- W osadzie gracza: symulacja ruchu w sesji (prędkość offroad jak gracz, `2` WU/h) oraz statusy (`w drodze`, `kupuje`, `zbiera`, `sprzedaje`, `pracuje`, `w domu`) — **bez** markerów na mapie.
- Panel Lokalizacja: podgląd domu, celu i statusu; budynki: [budynki-stan.md](./budynki-stan.md).
- Kod: `src-tauri/src/world/npc.rs`, sesja `npc_runtimes` w `session.rs`, UI w `src-tauri/src/app.rs`.

## Czego świadomie nie robi

- Dialogu, handlu, zleceń ani questów w grze.
- Markerów NPC na teksturze mapy.
- Zużycia zapasów w czasie.
- Harmonogramów ani frakcji.
- Relacji z graczem zapisanych w sesji.
- Wpływu reputacji / historii postaci na `inclination`.

## Dalsze kroki rozwoju

1. Nadać rolom proste funkcje (informator, sprzedawca, zleceniodawca) korzystające z `inclination`.
2. Dialog / ofertę zależną od skłonności oraz stanu osady i reputacji gracza.
3. Zapisywać relacje i wyniki rozmów/zleceń w warstwie sesji.
4. Harmonogramy i dostępność tylko dla aktywnej okolicy; poza nią — agregaty populacji.
5. Ujednolicić zawody z budynkami i usługami osady.

## Sugestie (poza obecnym modelem)

- Motywacje krótkoterminowe (głód, strach, chciwość) jako osobne tagi lub skale sesyjne.
- Frakcja / lojalność wobec osady lub rodu.
- Prosty status: zamożność, zdrowie (bez pełnej symulacji survivalu).
- Osobna skala charyzmy, jeśli wygląd ma wpływać na interakcje społeczne.
- Identyfikator stabilny NPC (`id` z seeda), gdy pojawią się zapisy relacji niezależne od indeksu na liście.

## Kryterium przejścia do etapu 2

Gracz w osadzie może spotkać co najmniej jedną osobę, której reakcja (pomoc / handel / informacja) wynika z `inclination` i daje widoczny skutek w sesji.
