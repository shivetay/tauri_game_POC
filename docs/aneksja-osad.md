# Aneksja osad po wojnie — plan wdrożenia

Dokument jest **specyfikacją wdrożeniową** kolejnego etapu polityki państw. Opisuje stan wyjściowy, założenia, etapy E1–E5 oraz kryteria sukcesu. Kod aneksji osad / buntu / secesji **jeszcze nie istnieje**.

Powiązane: [panstwa.md](./panstwa.md), [prawa.md](./prawa.md), [granice.md](./granice.md). Ticket: [tauri_game_POC#57](https://github.com/shivetay/tauri_game_POC/issues/57).

---

## 1. Cel

Po wygranej wojnie (ugoda):

1. **Osady** na anektowanej ziemi przechodzą do zwycięzcy (nie tylko wash granic).
2. **Prawa zwycięzcy** są egzekwowane albo napotykają opór.
3. Osady mogą **zaakceptować**, **stawiać opór** lub **się buntować**.
4. Bunt może wymagać **interwencji zbrojnej** (abstrakcyjnej).
5. Nieudana pacyfikacja może doprowadzić do **powstania nowego państwa**.

**Łańcuch:**

```
ugoda (okupacja → owners)
  → przepisanie osad (settlement_idxs)
  → reakcja osady: Accept | Resist | Rebel
  → egzekucja praw zwycięzcy (gdy Accept / po pacyfikacji)
  → opc. interwencja (manpower, miesiące)
  → opc. secesja → nowe Realm
```

---

## 2. Stan wyjściowy (kod, 19.10.2026)

| Element | Stan |
| --- | --- |
| Ugoda v0 | Zwycięzca anektuje **okupowane komórki** do `TerritoryMap.owners`; przebudowa polilinii granic |
| Osady | **Nie** zmieniają `Realm.settlement_idxs` / stolicy |
| Prawa | `Realm.laws` z seeda + capacity; widoczne w dossier; **bez** skutków na handel |
| Dyplomacja | Sesja: napięcie, wojna, embargo, okupacja (przy `ExpandBorders`) |

Luka: wash mówi „ziemia X”, a osada nadal należy do starego państwa.

---

## 3. Założenia (propozycja do akceptacji)

| # | Założenie | Uwaga |
| --- | --- | --- |
| A1 | Przynależność osady po ugodzie = właściciel komórki **centroidu** osady w `owners` (po aneksji). | Alternatywa: majority footprint — poza MVP. |
| A2 | Reakcja osady deterministyczna z `(seed, month_index, settlement_idx, winner_id, loser_id)` — osobny kanał PRNG sesji. | Nie mieszać z kanałami generatora świata. |
| A3 | Po aneksji obowiązują **prawa zwycięzcy**; pełna egzekucja tylko przy `Accept` lub po udanej pacyfikacji. | `Resist` = prawa „martwe” / osłabione. |
| A4 | Nowe państwo tylko z buntu **nie stłumionego** w limicie miesięcy. | Nie przy każdej ugodzie. |
| A5 | Interwencja bez armii na mapie: koszt `manpower` + miesiące + event w dossier. | |
| A6 | Secesja tworzy **jedno** nowe `Realm` z klastra zbuntowanych osad (nie osobne państwo na każdą wieś). | Stolice = największa osada klastra. |

**Do rozstrzygnięcia przed E1:** czy wystarczy centroid (A1), czy wymagany jest inny próg.

---

## 4. Stany osady (sesja)

```text
SettlementAllegiance {
  settlement_idx,
  realm_id,           // de jure po przepisaniu
  stance: Accept | Resist | Rebel,
  months_in_stance,
  pacify_months_left, // gdy trwa interwencja
}
```

| Stance | Prawa zwycięzcy | Następny krok |
| --- | --- | --- |
| `Accept` | egzekwowane | stabilne |
| `Resist` | nieskuteczne / połowa | timer → `Accept` (decay) albo `Rebel` |
| `Rebel` | brak | wymaga interwencji; fail → secesja |

**Score reakcji (MVP):**

```text
mismatch = różnica kultury/religii vs zwycięzca
score = f(mismatch, population, dist_to_winner_capital, loser_fame)
→ Accept | Resist | Rebel
```

---

## 5. Etapy wdrożenia

### E0 — Dokument (ten plik)

- [x] Specyfikacja łańcucha i etapów.
- [x] Akceptacja założeń A1–A6 (slice E1+E2; A3 tymczasowo = wszystkie przepisane jak Accept).
- **Verify:** można wskazać E1 bez dopytywania o model.

### E1 — Przepisanie osad przy ugodzie — **DONE**

**Zakres:**

- Po `annex_occupation` / `resolve_peace`: dla każdej osady, jeśli centroid jest na ziemi zwycięzcy → przenieś `settlement_idx` do `winner.settlement_idxs`, usuń z `loser`.
- Jeśli stolica przegrana traci stolicę → wybierz nową stolicę (największa pozostała osada / City).
- Przebudowa granic już jest; upewnić się, że dossier / etykiety używają zaktualizowanych list.

**Verify:** test — po peace z aneksją komórki ze osadą, osada jest w `winner.settlement_idxs`; `cargo test --lib`.

**Pliki:** `realm_diplomacy.rs`, `realm_annex.rs`.

### E2 — Egzekucja praw zwycięzcy na anektowanych — **DONE (MVP)**

**Zakres:**

- Anektowane osady w `Accept` (lub wszystkie tymczasowo, zanim E3) podlegają prawom zwycięzcy.
- Min. **jeden** twardy skutek: np. `GoodBan` / `Tariff` na eksporcie realm **albo** linia w panelu osady „Prawo: … (państwo X)”.
- Dossier zwycięzcy bez zmian katalogu praw (to nadal prawa realm — osady tylko „wchodzą pod parasol”).

**Verify:** po aneksji panel/dossier pokazuje spójne państwo + prawo; determinizm.

**W kodzie:** `apply_enforced_law_effects` (`GoodBan` na eksport/szlaki); klik osady → Państwo + Prawo.

### E3 — Reakcja osady (Accept / Resist / Rebel)

**Zakres:**

- Przy przepisaniu liczony `stance` (score §4).
- Event w `last_event` / lista w dossier („Osada Y stawia opór”).
- `Resist` / `Rebel`: prawa nie w pełni egzekwowane.

**Verify:** ten sam seed + miesiąc → te same stance; UI czytelne.

### E4 — Interwencja zbrojna

**Zakres:**

- Dla `Rebel` (i opc. twardego `Resist`): automatyczna lub warunek ambicji/manpower — start pacyfikacji.
- Koszt: `manpower` zwycięzcy; czas: N miesięcy.
- Sukces → `Accept` + egzekucja praw; porażka → eskalacja do secesji (E5).

**Verify:** bez manpower interwencja nie startuje / fail; event widoczny.

### E5 — Secesja / nowe państwo

**Zakres:**

- Klaster `Rebel` po fail pacyfikacji → nowe `Realm` (nazwa, profil z lokalnej kultury/religii, stolica = największa osada).
- Przepisanie `settlement_idxs`, `owners` komórek osad secesyjnych, rebuild granic.
- Relacje: `BorderFeud` / `ColdPeace` ze starym zwycięzcą i ewentualnie z przegranym.

**Verify:** liczba realmów ↑; stare państwo traci osady; teren seeda nietknięty.

---

## 6. Sugerowana kolejność

```
E0 (docs) → E1 + E2 (minimalny vertical slice)
         → E3 (opór)
         → E4 (interwencja)
         → E5 (nowe państwo)
```

Równolegle / niezależnie z prawami: dokończenie etapu C z [prawa.md](./prawa.md) (skutek `GoodBan`/`Tariff`) mocno pomaga E2.

---

## 7. Pliki (oczekiwane)

| Plik | Rola |
| --- | --- |
| `docs/aneksja-osad.md` | ten kontrakt |
| `src-tauri/src/world/realm_diplomacy.rs` | wywołanie po `resolve_peace` |
| `src-tauri/src/world/realm_annex.rs` | nowy — stances, przepisanie, secesja (opc.) |
| `src-tauri/src/world/borders.rs` | mutacja list osad / nowe Realm w sesji |
| `src-tauri/src/world/realm_laws.rs` | egzekucja po aneksji (E2) |
| `src-tauri/src/app.rs` | eventy / status w panelu osady lub dossier |
| `docs/panstwa.md`, `aktualny_stan_prac.md` | sync po wdrożeniu |

**Świadomie poza zakresem:** armie na mapie, questy gracza „idź pacyfikować”, zmiana ustroju, pełny save/load (gdy brak — stan tylko w żywej sesji).

---

## 8. Ryzyka

| Ryzyko | Mitygacja |
| --- | --- |
| Cache LOD trzyma stare `realms` | przepisywać w tym samym miejscu co `owners` w ticku dyplomacji |
| Secesja bez `owners` = rozjazd wash vs osady | E5 zawsze aktualizuje raster + listy |
| Zbyt częste nowe państwa | jeden klaster / ugoda; próg populacji |
| Łamanie determinizmu seeda | tylko kanał sesji; brak regeneracji osad z seeda |

---

## 9. Definition of Done (całość E1–E5)

1. Ugoda z aneksją ziemi przepisuje osady do zwycięzcy.
2. Prawa zwycięzcy obowiązują anektowane osady (z ograniczeniem przy oporze).
3. Widoczne stany Accept / Resist / Rebel + eventy.
4. Interwencja ma koszt i wynik.
5. Nieudana pacyfikacja może utworzyć nowe państwo.
6. `cargo test --lib`; brak regresji profilu terenu seeda `6`.

---

## 10. Stan w kodzie

**E1 + E2 wdrożone** (23.10.2026). E3–E5 jeszcze nie.

| Element | Miejsce |
| --- | --- |
| Przepisanie osad po ugodzie | `realm_annex.rs` + `resolve_peace` |
| `GoodBan` na eksporcie | `realm_laws::apply_enforced_law_effects` |
| UI osada | `app.rs` — Państwo + Prawo przy kliku |
| Testy | `peace_transfers_settlement_on_annexed_cell`, `enforced_good_ban_removes_export` |
