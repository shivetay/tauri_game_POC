# Plany rozbudowy — od mapy do gry

Dokument opisuje, jak obecny podgląd proceduralnej mapy (Rust + egui) może stać się częścią gry, w której:

- jesteś częścią świata i możesz zacząć w dowolnym miejscu na mapie,
- zaczynasz bez niczego,
- pniesz się w górę (zarabiasz, handlujesz itd.),
- losy świata wpływają na ciebie, a ty wpływasz na losy świata,
- śmierć bez potomka kończy linię gracza (system rodu).

Teren z seeda pozostaje **deterministyczny i niemutowalny**. Wszystko „żywe” (czas, ekonomia, delty, rody, frakcje, granice miast) to osobna **warstwa sesji**.

---

## Stan obecny (leverage)

| Warstwa | Stan | Pod grę |
| --- | --- | --- |
| Seed + determinizm | gotowe | ten sam świat = ta sama tożsamość save |
| LOD świat → region → obszar | podgląd | kamera, podróż, odkrywanie; **dual-sim** (detal przy graczu) |
| Biomy, wysokość, wilgotność | gotowe | produkcja surowców, ryzyka, ceny |
| Rzeki + mosty | gotowe | logistyka, porty, powodzie |
| Osady + dzielnice + drogi | statyczne | rynki, cechy, konflikty granic |
| Ekologia | podgląd | zbieractwo, sezon, overhunt |
| `TerrainDelta` (`delta.rs`) | stub | rozrost miasta, wycinka, klęski |

**Czego brakuje do wizji gry:** czas (ticki), postać/ród, ekonomia produkująca eventy, matryca relacji, mutowalne granice osad, zapis sesji.

**Zasada architektoniczna:** bazowy teren = immutable z `(seed, params)`; rozgrywka = save `{ seed, params, clock, player_lineage, economy, relations, settlement_overlays, world_deltas, … }`.

**Największy gotowy asset:** graf **osady + drogi + rzeki + biomy** + LOD obszarów. To naturalny szkielet pod A–E poniżej.

**Sugerowana kolejność (po analizie A–E):**

```
A  Czas + dual-sim (tick / dzień / sezon)     ← fundament wszystkiego
B  Ekonomia biomowa → braki → eventy/questy  ← silnik napędowy gracza
   + postać, handel, delty v0 (MVP playable)
C  Ród / dynastia (majątek + reputacja rodu) ← śmierć ≠ koniec, jeśli jest dziedzic
D  Cechy + matryca relacji + urazy pokoleń   ← pamięć społeczna
E  Rozrost granic miast + konflikty terytorialne ← geografia w ruchu
```

---

## Filar A–E — analiza względem obecnej mapy

Poniższe pięć filarów to kanoniczna oś rozbudowy. Reszta dokumentu (warstwy 1–3) je uszczegóławia.

### A. Upływ czasu (świat w ruchu)

**Teza:** zanim miasta „ożyją”, musi istnieć zegar gry (ticki → dni → sezony → lata). Nie symulować każdego mieszkańca w RT — **dual-sim**:

| Strefa | Co symulować | Skala mapy |
| --- | --- | --- |
| Przy graczu | detal (NPC, dialog, loot, sklepy, walka lite) | obszar / chunk, np. `(cx, cy)` w regionie |
| Reszta świata | tania matematyka (stoki, braki, migracje, tick ekonomii) | region / świat w tle (Rust backend) |

**Jak to spiąć z mapą:**

- LOD już dzieli świat → region → obszar — to gotowy podział „gdzie jest detal”.
- `WorldState` / stan aplikacji dziś trzyma seed, params i LOD — naturalne miejsce na `GameClock` + kolejkę ticków.
- Determinizm: tick N na seedzie S daje te same wyniki agregatów (PRNG kanałowy jak w `prng.rs`).

**ECS — werdykt:**

- Idea „entity + komponenty + systemy na tick” jest właściwa.
- **Nie** wymaga od razu Bevy/pełnego ECS w crate’cie mapy. Wystarczy lekki model w Rust: encje sesji (`SettlementEco`, `Stockpile`, `House`, `Agent`) + systemy `on_day` / `on_season` wywoływane z zegara.
- Pełny ECS rozważać dopiero, gdy liczba agentów przy graczu urośnie (NPC w dzielnicach).

**Ryzyka:** zbyt gęsty tick globalny zabije FPS; zbyt rzadki — świat „skacze”. Start: 1 tick = 1 godzina gry, agregaty ekonomii co dzień/tydzień.

**Miejsca rozwoju:** `app.rs` (stan sesji), nowy `world/clock.rs`, `world/sim.rs` (far), detal w UI tylko dla aktywnego chunka.

**Zależności:** A blokuje B–E. Bez czasu nie ma braków zimowych, dziedziczenia ani rozrostu granic.

---

### B. Ekonomia jako silnik wydarzeń i kariery gracza

**Teza:** świat **sam** generuje wydarzenia z niedoborów. Biomy + wielkość osady = produkcja / przetwórstwo.

| Rola | Źródło na mapie | Przykład |
| --- | --- | --- |
| Wieś / hamlet | biom wokół + ekologia | Forest → drewno; Grass → zboże; Water/rzeka → ryby; Desert → glina/sól (uprościć) |
| Miasto / city | dzielnice Craft / Market / Port | przetwarza surowce w towary, zgłasza popyt |
| Brak | stok poniżej progu przed zimą | event na mapie + ogłoszenie w tablicy questów |
| Gracz od 0 | luka cenowa / dostawa | kup tanio u producenta, sprzedaj w mieście z niedoborem |

**Pętla:**

```
biom → produkcja wioski → zapas
zapas niski → Need(drewno, zima)
Need → Event + Quest(handlarz/kurier)
gracz wypełnia lukę → złoto + reputacja
nie wypełnia nikt → głód / drożyzna / migracja (świat → gracz przy następnej wizycie)
```

**Jak to spiąć z mapą:**

- `SettlementKind` + `population` + `districts` (Craft, Market, Port) już opisują rolę osady.
- Biom komórki / okolicy z `TerrainSampler` → tabela surowców (obok `biome.rs` / `ecology.rs`).
- Drogi (`Road`) = koszt i czas dostawy; mosty / powódź = zerwanie łańcucha.
- Tablica ogłoszeń = UI przy kliku osady (dziś: info o mieście).

**Ryzyka:** ekonomia bez limitu surowców stanie się arbitrażem bez sensowności; trzeba soft-cap produkcji z biomu i zużycia sezonowego.

**Miejsca rozwoju:** `world/economy.rs`, rozszerzenie `settlement`, komendy `market_at` / `board_at`, overlay niedoborów na regionie.

**Zależności:** wymaga A (sezon/zima). Napędza questy MVP i późniejsze frakcje (D).

---

### C. System rodu (dynastii)

**Teza:** śmierć **bez potomka** = koniec gry (linii). Postać nie zbiera tylko złota osobistego, lecz **Majątek Rodowy** i **Reputację Rodu**.

| Pojęcie | Zachowanie |
| --- | --- |
| Majątek osobisty | ginie częściowo przy śmierci (strata, pogrzeb, rabunek) |
| Majątek rodowy | ziemia, skład, tytuły, kontrakty — przechodzi na dziedzica |
| Reputacja rodu | per region/osada; syn dziedziczy ułamek (np. 40–70%), nie 100% |
| Start dziedzica | mniej kapitału niż ojciec w szczycie, ale rozpoznawalność w „ojczyźnie” |

**Jak to spiąć z mapą:**

- Namegen osad (`settlement.rs`) → wzorzec nazw rodów / herbów.
- Reputacja startowa dziedzica silniejsza w regionie, gdzie ojciec handlował (LOD regionu).
- Save musi trzymać `Lineage { members, heir, house_wealth, house_rep[] }`, nie tylko `Player`.

**Ryzyka:** zbyt surowa utrata = frustracja; zbyt łagodna = śmierć bez znaczenia. Dziedzic powinien być **decyzją** (ślub, adopt, uznanie), nie automatem po 5 minutach.

**Miejsca rozwoju:** `world/lineage.rs`; UI „księga rodu”; zmiana reguły śmierci z soft-respawnu (warstwa 1) na hard-line gdy brak dziedzica.

**Zależności:** sensowne po B (jest co dziedziczyć). Matryca urazów (D) operuje na **rodzie**, nie na jednej postaci.

---

### D. Frakcje, ambicje, urazy generacyjne

**Teza:** społeczeństwo miast = **cechy** (Gildia Kupiecka, Cech Rzemieślników, Złodzieje, Kapituła przy Temple…). Relacje = **graf / matryca** liczbowa między rodami NPC, cechami i rodem gracza.

**Przykład pętli (widok obszaru z dzielnicami):**

1. Ród gracza (lub zleceniodawca) przejmuje wpływy w dzielnicy Craft.
2. Pokonany ród NPC zapisuje `Grudge { actor, target, cause, year, decay }`.
3. 50 lat później praprawnuk handluje w tym mieście → gorsze ceny / napad / odmowa wstępu — „grzechy dziadka”.

**Jak to spiąć z mapą:**

- `DistrictKind` (Craft, Market, Port, Noble, Temple…) = terytorium wpływów cechów.
- Osada = arena; region = zasięg plotki; świat = rzadkie echa wielkich urazów.
- Event z B („brak drewna”) może być **oprawiony** przez cech (kupcy płacą za dostawę, złodzieje chcą konwój).

**Reprezentacja:** rzadka macierz lub lista krawędzi `(id_a, id_b) → score` + log wydarzeń (nie pełna N² od dnia 1 — rody pojawiają się lazy).

**Ryzyka:** bez decay urazy wieczne i nieczytelne; bez logu przyczyn gracz nie rozumie kary. Każda uraza potrzebuje **jednego zdania** w kronice rodu.

**Miejsca rozwoju:** `world/relations.rs`, `world/guilds.rs`; UI matrycy uproszczonej; modyfikatory cen w `economy`.

**Zależności:** C (podmiot = ród), B (powód konfliktu = pieniądze/wpływy), A (lata na zegarze).

---

### E. Rozwój i granice miast

**Teza:** miasta rosną w czasie **rozrostem komórkowym** na kafelkach wokół footprintu. Spotkanie granic dwóch osad → konflikt o zasoby/terytorium → event (gracz jako żołnierz, najemnik, szpieg, negocjator).

**Jak to spiąć z mapą:**

- Dziś osady mają punkt + `radius` + dzielnice kątowe — statyczny footprint.
- Overlay sesji: bitmask / zbiór komórek `owned_by_settlement` narastający co sezon (nie przepisywać immutable terenu — tylko claim + ewentualna delta biomu przy zabudowie).
- Kolizja claimów na LOD regionu / świata (jak na mapie globalnej z wieloma miastami) = generator eventów.
- Rzeki i drogi ograniczają / kierują rozrost (preferencja wzdłuż drogi i brzegu).

**Algorytm (v1):** co sezon, dla każdej osady z rosnącą populacją / zapasem: zajmij N losowych (deterministycznych) sąsiadów lądu w zasięgu, unikaj wody głębokiej i claimów obcych; przy kontakcie z obcym claimem → `BorderTension` event.

**Ryzyka:** rozrost bez limitu zaleje mapę; trzeba soft-cap od żywności (B) i reliefu. Konflikty bez ekonomii będą puste — E po B.

**Miejsca rozwoju:** `world/claims.rs` + `TerrainDelta` przy zabudowie; overlay na `RegionMapView` / `GlobalMapView`; link do eventów (B/D).

**Zależności:** A (sezony), B (żywność → wzrost), później D (który cech naciska na ekspansję).

---

### Macierz zależności A–E

```
A (czas / dual-sim)
 └── B (ekonomia → eventy/questy) ──┬── C (ród)
                                    ├── E (granice / konflikty)
                                    └── D (cechy + urazy) ← wymaga C
```

**Implementacyjnie najpierw:** A → B (playable handlarz) → C → E → D (D jest najambitniejsze i najłatwiej się rozjeżdża bez solidnego B/C).

---

## 1. Podstawa (MVP gry w świecie mapy)

Cel: da się **żyć** na mapie — urodzić się, chodzić, mieć nic, zarobić pierwsze monety na luce rynkowej, zapisać grę. Zegar (A) i zalążek ekonomii (B) są częścią MVP; pełny ród/urazy/granice — później.

### 1.1. Postać gracza i spawn „gdziekolwiek”

- Wybór punktu na mapie świata / regionu (klik = start, nie tylko zoom).
- Walidacja: suchy ląd, nie koryto rzeki, nie śnieg wysokogórski (reuse scoringu biomu / `site_score`).
- Stan: pozycja `(x, y)`, aktywny region/chunk (kotwica dual-sim z A).
- Start: pusty ekwipunek, 0 monet, brak tytułu — „bez niczego”.
- **Gdzie:** `world/player` + stan w `MapApp` / sesji gry; spawn przez `TerrainSampler` + `RiverSample`.

### 1.2. Kamera i podróż (LOD = strefa detalu)

- Tryb „jestem tu”: środek widoku = pozycja gracza.
- Wejście w obszar `(cx, cy)` włącza symulację detaliczną; wyjście ją wyłącza (A).
- Koszt ruchu od biomu; drogi przyspieszają.
- Lekki fog of war: biom z daleka, nazwy osad po odkryciu.
- **Gdzie:** `app.rs` (mapa + marker gracza); ruch w Rust.

### 1.3. Zegar gry, ticki i zapis (filar A — minimum)

- Hierarchia: tick (godzina) → dzień → sezon → rok.
- Dual-sim: detal tylko w aktywnym obszarze; daleko — `on_day` / `on_season` na stockpile’ach osad.
- Save: `{ seed, params, clock, player, discoveries, inventory, world_deltas, economy_snapshot }`.
- Load: teren z seeda + nakładki sesji.
- Pauza / prędkość czasu w UI.
- **Gdzie:** `world/clock.rs`, `world/sim.rs`, stan sesji w aplikacji egui; persist JSON (plik lokalny).
- **Uwaga ECS:** systemy tickowe jako zwykłe funkcje Rust na start; pełny ECS opcjonalnie później.

### 1.4. Survival-lite: zbieractwo z biomu

- Loot z `TileType` + ecology chunka.
- Nazwy z list gatunków w `ecology.rs`.
- Limit slotów — start „z niczym” ma sens.
- Regeneracja wolna; pełne sezony po spięciu z A/B.
- **Gdzie:** `gather` / `loot` + UI ekwipunku.

### 1.5. Handel + pierwsze Needs z biomu (filar B — minimum)

- Wieś produkuje 1–2 surowce z biomu okolicy; miasto kupuje/przetwarza drożej.
- Ceny bazowe od `SettlementKind` + lokalnego stockpile.
- Gdy stockpile < próg → wpis na tablicy ogłoszeń („Brak drewna przed zimą”) + marker eventu na mapie regionu.
- Gracz od 0 zarabia na dostawie / arbitrażu.
- **Gdzie:** `world/economy.rs` (v0), `settlement` oferty; `market_at` / `board_at`.

### 1.6. Tablica ogłoszeń zamiast losowych jobów w próżni

- Quest = materializacja `Need` z ekonomii (B), nie ręczny spis misji.
- Typy v0: dostawa surowca, kurier między osadami na drodze.
- Nagroda: monety + mikro-reputacja lokalna (zalążek pod C/D).
- Pool odświeżany co dzień/tydzień z zegara (A).
- **Gdzie:** `world/quests.rs` czyta stan `economy`; UI przy osadzie.

### 1.7. Potrzeby ciała i śmierć (most do C)

- Głód / zmęczenie / zimno (tundra, śnieg, zima z A).
- **MVP śmierci:** soft — utrata części majątku osobistego, respawn przy sojuszniku / ostatniej wsi **albo** kontynuacja tylko jeśli ustawiono dziedzica (gdy C jeszcze nie gotowe: flaga „prototype heir”).
- Docelowo (C): brak dziedzica = **koniec linii** (game over ekran rodu).
- Bagno / pustynia = wyższy koszt utrzymania.
- **Gdzie:** stan gracza; tick; później `lineage`.

### 1.8. Odkrywanie mapy i dziennik / kronika

- Odkryte chunki; wpisy: osada, niedobór, pierwsza dostawa.
- Kronika = podstawa pod log urazów (D) i historię rodu (C).
- Szybka podróż po odkrytych drogach.
- **Gdzie:** bitset w save; overlay `GlobalMapView`.

### 1.9. Wpływ gracza v0 — delty terenu

- Wycinka / chata / pole → `TerrainDelta`.
- Delty w save; aplikacja w `cell_at` po bazie.
- Zalążek pod zabudowę z E (claim ≠ od razu zmiana biomu).
- **Gdzie:** `delta.rs` + sampler.

### 1.10. UI: Preview vs Play

- Preview = generator (jak dziś). Play = HUD + zegar + tablica + rynek.
- HUD: data/sezon (A), monety, głód, najbliższa osada, aktywne Needs.
- Suwaki orogenezy tylko przy **nowej grze**.
- **Gdzie:** `app.rs` (egui); panele `PlayHud`, `SettlementPanel`, `QuestBoard`.

**Uzasadnienie warstwy:** A daje ruch; B daje powód do zarabiania od zera na lukach rynku. Bez tego reszta filarów nie ma na czym stanąć.

---

## 2. Rozszerzona (żywy świat, handel i pierwsze konsekwencje)

Cel: ekonomia i czas napędzają eventy; pojawia się ród; granice i cechy w wersji lekkiej.

### 2.1. Gospodarka regionalna pełna (rozszerzenie B)

- Podaż/popyt per osada; karawany AI po `Road`.
- Sezonowe zużycie (zima pali drewno); klęski plonów z biomu/eventu.
- Sprzedaż gracza rusza cenami; brak dostaw → migracja / spadek populacji.
- Łańcuchy: wieś → miasto (Craft) → eksport Portem / Highway.
- **Gdzie:** `economy` tick tygodniowy; heatmap na regionie.

### 2.2. Cechy miejskie v1 (zalążek D)

- 2–4 cechy na City/Town: Kupcy, Rzemiosło, Port, Temple (mapowanie na `DistrictKind`).
- Cech wystawia ogłoszenia i płaci premie.
- Mikro-reputacja gracza vs cech (jeszcze nie pełna matryca rodów).
- Konflikt cechów o tę samą dostawę = wybór strony.
- **Gdzie:** `world/guilds.rs`; modyfikatory w economy/quests.

### 2.3. Kariery: handlarz, kurier, przewoźnik, zbieracz

- Skille lekkie; craft podstawowy z surowców biomu.
- Ładowność i czas na drogach/rzekach.
- Tytuły od majątku i reputacji lokalnej.
- **Gdzie:** `player.skills`; receptury obok ecology.

### 2.4. NPC w aktywnym obszarze (dual-sim detal)

- Spawn w dzielnicach tylko gdy gracz jest w obszarze.
- Role z cechów / dzielnic; harmonogram dzień/noc z zegara.
- Daleko: zero osobnych NPC — tylko liczby w stockpile.
- **Gdzie:** detal w chunk play; daleko w `sim.rs`.

### 2.5. Eventy świata z ekonomii i natury

- Needs (B), powódź (`river`), susza, bandyci na Local/Dirt, jarmark.
- Każdy event ma wpis w kronice (pod D).
- Marker na LOD regionu/świata.
- **Gdzie:** `world/events.rs`.

### 2.6. Majątek poza kieszenią + zalążek rodu (C v0)

- Skład / działka w Outskirts = majątek rodowy (nie ginie w całości przy śmierci).
- Uznanie dziedzica (NPC lub wygenerowany).
- Reputacja rodu per osada: dziedzic startuje z ułamkiem.
- Ekran „księga rodu” (2–3 pokolenia max na razie).
- **Gdzie:** `world/lineage.rs`; struktury + save.

### 2.7. Polityka lokalna lekka

- Cła, petycje (wycinka pod pastwiska → delta + napięcie ekologii).
- Embargo między osadami = zmiana grafu handlu.
- **Gdzie:** flagi na `Settlement` sesji.

### 2.8. Logistyka rzeczna i mosty

- Transport wodny; opłaty mostowe; zniszczenie mostu zimą/powodzią → Need + quest.
- **Gdzie:** `river` + `RoadCrossing::Bridge` HP/owner.

### 2.9. Ekologia pod produkcję

- Overhunt / wycinka obniża przyszłą produkcję wsi (feedback do B).
- **Gdzie:** overlay ecology w sesji.

### 2.10. Claims v0 — pierwsze granice (zalążek E)

- Co sezon osada z nadwyżką żywności zajmuje 0–2 kafelki wokół footprintu.
- Kontakt claimów → `BorderTension` (na razie tylko event + quest negocjacji/najmu, bez pełnej wojny).
- Overlay granic na mapie regionu.
- **Gdzie:** `world/claims.rs` + rysowanie w `RegionMapView`.

**Uzasadnienie warstwy:** domyka pętlę A+B, wprowadza C i E w formie grywalnej, przygotowuje grunt pod pełne urazy (D).

---

## 3. Bardzo zaawansowana (pamięć pokoleń i geografia w konflikcie)

Cel: urazy generacyjne, organiczny rozrost miast, demografia i państwa — silnik historii.

### 3.1. Demografia i awans osad

- Populacja z żywności (B), chorób, wojen, migracji.
- Hamlet → … → City lub ruiny; nowe osady na high `site_score` + drodze.
- **Gdzie:** ewolucja overlayu `SettlementMap` w dekadach.

### 3.2. Dynastia pełna (filar C)

- Genealogia NPC i gracza; małżeństwa; konflikty spadkowe.
- Majątek rodowy vs osobisty; reputacja rodu z decay i dziedziczeniem ułamkowym.
- Śmierć bez dziedzica = game over linii (z kroniką końcową).
- **Gdzie:** `lineage` + generator imion/herbów.

### 3.3. Matryca relacji i urazy pokoleń (filar D)

- Graf rodów × cechów × rodów; `Grudge` z datą, przyczyną, siłą, decay.
- Wpływ na ceny, napady, odmowę handlu dekady później.
- Kronika: „W roku X ród Y wyparł Z z dzielnicy Craft w …”.
- Lazy creation krawędzi — nie pełna macierz od startu.
- **Gdzie:** `world/relations.rs`; UI skrótu relacji w mieście.

### 3.4. Rozrost komórkowy i wojny granic (filar E)

- Pełny cellular growth z limitami żywności/reliefu/rzek.
- Spotkanie granic → wojna / arbitraż / najemnik (role gracza).
- Spalone ziemie = delty biomów + uchodźcy + urazy (D).
- **Gdzie:** `claims` + `events` + combat abstrakcyjny na grafie dróg.

### 3.5. Polityka i państwa

- Unie miast, granice na LOD świata, dyplomacja.
- Ścieżka: kupiec → wpływ w cechu → burmistrz → kanclerz.
- **Gdzie:** `polity` nad regionami.

### 3.6. Klimat, ery, katastrofy długie

- Epochy jako modyfikatory (bez reseedu); zmiana znaczenia szlaków rzecznych.
- **Gdzie:** `climate_epoch` w sesji.

### 3.7. Kultura i religia

- Temple, pielgrzymki, zakazy handlowe; czyny w pieśniach.
- **Gdzie:** tagi kulturowe; generator narracji z kroniki.

### 3.8. Questy narracyjne z geografii + relacji

- Planner: Need (B) + droga + uraza (D) + napięcie granic (E).
- Wątki wielopokoleniowe.
- **Gdzie:** planner na grafie osady × drogi × biomy × relations × claims.

### 3.9. Multigracz / shared seed (ostrożnie)

- Determinizm terenu; niedeterminizm społeczny osobno.
- **Gdzie:** nie blokuje single-player.

### 3.10. Narzędzia autora

- Podgląd stockpile, matrycy relacji, heatmap claimów, wymuszanie Needs.
- Preview generatora = laboratorium.
- **Gdzie:** tryb Dev.

**Uzasadnienie warstwy:** tu filary C–E osiągają pełną wizję „losy świata ↔ ty” w skali pokoleń. Bez stabilnego A+B to vaporware.

---

## Mapa miejsc w kodzie (skrót)

| Obszar | Dziś | Kierunek (A–E) |
| --- | --- | --- |
| `app.rs` (egui) | seed, params, LOD, legendy | Play, HUD, overlay Needs/claimów |
| `render.rs` / `*_draw.rs` | tekstura mapy (teren + osady + ekologia) | markery gracza, eventy, fog |
| `world/` + ewentualny stan sesji | generacja + podgląd | zegar, save, Play |
| nowy `world/clock.rs`, `sim.rs` | — | A: ticki, dual-sim |
| nowy `world/economy.rs` | — | B: stockpile, Needs, ceny |
| nowy `world/quests.rs` | — | B: tablica z Needs |
| `settlement.rs` | osady, drogi, dzielnice | rynki, cechy, claim anchor |
| `biome.rs` / `ecology.rs` | klasyfikacja / gatunki | tabele produkcji surowców |
| `river.rs` | hydrologia | logistyka, powodzie, mosty |
| `delta.rs` | stub | zabudowa, klęski |
| nowy `world/lineage.rs` | — | C: ród, dziedzic |
| nowy `world/guilds.rs`, `relations.rs` | — | D: cechy, urazy |
| nowy `world/claims.rs` | — | E: granice, tension |

**Stack UI:** czysty Rust (`eframe` / `egui`). Nie ma Reacta ani Tauri IPC — generacja i rysowanie w jednym procesie.

---

## Następny krok (vertical slice)

Ścieżka **handlarza** spięta z A+B (bez pełnego C–E):

1. Zegar (dzień/sezon) + save/load.
2. Spawn na lądzie; aktywny obszar = detal.
3. 2–3 surowce z biomu; wsie produkują, miasta kupują.
4. Sztuczny lub wyliczony **Need** („brak drewna”) → wpis na tablicy + marker.
5. Gracz dostarcza → złoto; zapis zachowuje clock, kasę, stockpile.

Success criteria: seed `6` → nowa gra → dotrzyj do osady z niedoborem → sprzedaj/dostarcz → save/load z tą samą datą, kasą i Need rozwiązanym (lub wygasłym).

**Świadomie później:** dziedzic (C), matryca urazów (D), cellular claims (E).
