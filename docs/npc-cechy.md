# NPC — cechy, aktywność i podróż (stan wdrożenia)

Dokument opisuje to, co jest wdrożone w kodzie, oraz kolejne kroki i sugestie. Plan zależności: [npc.md](./npc.md). Drogi: [drogi.md](./drogi.md).

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
- Wiedza o okolicy: `known_settlement_indices` (sąsiedzi po grafie dróg, 1 hop, max 1–3, ze seeda); w sesji można nauczyć się kolejnych osad.
- Symulacja **wszystkich** NPC w tle (`npc_runtimes` w sesji):
  - w osadzie gracza: ruch offroad `2` WU/h między budynkami;
  - między osadami: abstrakcyjny ETA po `settlement_road_graph` przy `5` WU/h (bez polilinii i markerów).
- Brak lokalnego zapasu → `TravelRemote` → `AtRemote` (kupno, mutacja stock) → `ReturnRemote`; jeśli brak znanej osady z towarem → `SeekingInfo` (NPC→NPC, rzut vs `ShareLocalInfo`, nauka osady w sesji).
- Dom NPC (`settlement_index`) nie zmienia się; lokalizacja bieżąca żyje tylko w runtime.
- Panel Lokalizacja: statusy m.in. `pyta o drogę`, `w drodze do …`, `w …`, `wraca z podróży`; budynki: [budynki-stan.md](./budynki-stan.md).
- Kod: `src-tauri/src/world/npc.rs`, `settlement_road_graph` / `settlement_road_distance` w `settlement.rs`, `session.rs`, UI w `app.rs`.

## Czego świadomie nie robi

- Dialogu gracza, handlu z graczem, zleceń ani questów.
- Markerów NPC na teksturze mapy.
- Trwałej migracji (wyjazd jest tymczasowym zaopatrzeniem).
- Harmonogramów ani frakcji.
- Relacji z graczem zapisanych w sesji.
- Wpływu reputacji / historii postaci na `inclination`.

## Dalsze kroki rozwoju

1. Dialog gracza (informator) korzystający z `ShareLocalInfo` / wiedzy NPC.
2. Ofertę handlu zależną od skłonności oraz stanu osady i reputacji gracza.
3. Zapisywać relacje i wyniki rozmów/zleceń w warstwie sesji.
4. Markery / widoczny ruch NPC na drodze (LOD).
5. Ujednolicić zawody z budynkami i usługami osady.

## Sugestie (poza obecnym modelem)

- Motywacje krótkoterminowe (głód, strach, chciwość) jako osobne tagi lub skale sesyjne.
- Frakcja / lojalność wobec osady lub rodu.
- Prosty status: zamożność, zdrowie (bez pełnej symulacji survivalu).
- Osobna skala charyzmy, jeśli wygląd ma wpływać na interakcje społeczne.
- Identyfikator stabilny NPC (`id` z seeda), gdy pojawią się zapisy relacji niezależne od indeksu na liście.

## Kryterium przejścia do dialogu z graczem

Gracz w osadzie może spotkać co najmniej jedną osobę, której reakcja (pomoc / handel / informacja) wynika z `inclination` i daje widoczny skutek w sesji (np. poznanie nowej osady albo oferta handlu).
