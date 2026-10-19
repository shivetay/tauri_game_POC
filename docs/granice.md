# Granice miast i państw — plan wdrożenia (issue #9)

**Iteracja:** 1 (pierwsza iteracja planu)
**Utworzono:** 07.10.2026
**Ostatnia aktualizacja:** 07.10.2026 — uwzględniono założenia z arkusza wizji gry
**Status:** plan do akceptacji, bez zmian w kodzie.

## Cel

Pokazać na mapie granice miast i państw jako warstwę odrębną od dróg, dzielnic i murów. Ten sam seed i te same parametry dają te same granice.

## Założenia z arkusza wizji gry

Źródło: arkusz projektowy gry (rpg/roguelike, świat generowany z seeda i symulowany w czasie).

- Państwa i królestwa mają własne tereny, ambicje oraz ustroje (pacyfistyczne, anarchistyczne, militarne itd.). W tej iteracji powstaje tylko **geometria terytoriów**; ambicje i ustroje są poza zakresem.
- Mogą istnieć **wolne państwa-miasta**. Iteracja 1 to uwzględnia: `City` bez osiągalnej stolicy lub oddalone ponad próg kosztu tworzy własne państwo z jednej osady.
- Frakcje ustanawiają prawa i regulują świat. Granice są późniejszą podstawą przypisania prawa do terytorium, ale same prawa nie powstają w tej iteracji.
- Populacja jest dynamiczna i może migrować, a świat jest symulowany w czasie. Granice w iteracji 1 są **statyczne**, a ich dynamika (rozrost, zmiany po symulacji) to kolejne iteracje.
- Biomy, zasoby, drogi, rzeki i morza wpływają na rozwój osad. Dlatego koszt terytorium uwzględnia wodę i rzeki. Zasoby i biomy mogą wejść do kosztu w kolejnej iteracji.
- Gracz poznaje zmiany na świecie od innych. Docelowo granice mogą być odkrywane stopniowo. W iteracji 1 przełącznik „Granice” pokazuje je w całości (POC).
- Fantastyczne rasy w arkuszu nie wpływają na iterację 1.

## Iteracje

| Iteracja | Zakres |
| --- | --- |
| **1 (ten dokument)** | Statyczne granice miast i państw z seeda, wolne państwa-miasta, rysowanie na LOD i przełącznik UI. |
| 2 | Profil państwa i prawa przypisane do terytorium. |
| 3 | Relacje między państwami i stopniowe odkrywanie granic. |
| 4 | Zmiany granic w sesji. |

Etapy poniżej (sekcja „Etapy”) dotyczą iteracji 1. Iteracje 2–4 są propozycją, utworzoną 07.10.2026, i wymagają osobnej akceptacji.

## Stan wyjściowy

- `Settlement` (`world/settlement.rs`) ma pozycję, `radius`, dzielnice, ulice i flagę `walled`.
- `SettlementMap` zawiera tylko `settlements` i `roads`.
- Brak modelu przynależności osad do państw oraz brak przebiegu granic.
- [panstwa.md](./panstwa.md) zaleca na początku lokalne frakcje bez globalnych granic. Ten plan świadomie wyprzedza ten krok i dodaje wyłącznie statyczną, podglądową warstwę granic, bez dyplomacji, podatków, konfliktów i rozrostu terytoriów.

## Ustalenia

| # | Temat | Decyzja |
| --- | --- | --- |
| 1 | Kształt granicy miasta | Wielokąt, niekoniecznie regularny. Wierzchołki na promieniach wokół osady z deterministycznym zaszumieniem długości. |
| 2 | Podział na państwa | Stolice wybierane spośród `City` (w razie braku spośród `Town`). Pozostałe osady trafiają do najbliższej stolicy według kosztu wzdłuż grafu dróg. Osady bez drogi przypisywane po odległości euklidesowej. |
| 3 | Terytorium państwa | Wariant Voronoi z wagą kosztu na siatce. Granica państwa to kontur między komórkami różnych państw. |
| 8 | UI | Przełącznik „Granice”, domyślnie wyłączony. Na etapie POC użytkownik włącza go ręcznie. |

### Do potwierdzenia (domyślnie wg rekomendacji)

| # | Temat | Propozycja |
| --- | --- | --- |
| 4 | Woda | Granice nie przechodzą przez ocean. Rzeki podnoszą koszt przejścia, więc granica może na nich się układać. |
| 5 | LOD | Świat: granice państw. Region: państwa i miasta. Chunk: tylko granica miasta. |
| 6 | Wygląd | Państwo: gruba linia przerywana w kolorze państwa i półprzezroczysty wash terytorium. Miasto: cienka linia punktowa. Różni się od dróg (ciągłe linie), dzielnic (wypełnienia) i murów (grube, ciemne). |
| 7 | Nazwy i kolory | Nazwy z istniejącego generatora nazw, kolor z palety po indeksie państwa. |
| 9 | Archipelag i kontynenty | Państwo może obejmować kilka lądów. Terytorium liczone po koszcie, a wody traktowane jako nieprzekraczalne. Osady bez osiągalnej stolicy tworzą własne państwo; w szczególności samodzielne miasto jest wolnym państwem-miastem. |
| 10 | Hamlet i Village | Nie mają własnej granicy miasta. Należą do państwa. Własną granicę mają `Town` i `City`. |
| 11 | Wolne państwa-miasta | `City` oddalone od stolic ponad próg kosztu tworzy własne państwo jednoosadowe. Próg dobierany deterministycznie w testach. |

## Zasady determinizmu

- Granice są czystą funkcją `(seed, TerrainGenParams)` i nie zawierają stanu sesji.
- Generowanie używa osobnego strumienia PRNG z własną solą. Nie zmienia kolejności losowań w istniejących osadach i drogach.
- Po zmianie punktem odniesienia jest seed `6` (Ellipse). Osady i drogi muszą pozostać identyczne. Wartości `seed % 6` zachowują ten sam profil.

## Etapy

### Etap 1 — model danych
Plik: nowy `world/borders.rs`, rejestracja w `world/mod.rs`.

- `CityBorder { settlement_idx, polygon }`.
- `Realm { id, name, capital_idx, settlement_idxs }`.
- `RealmBorder { realm_a, realm_b: Option<usize>, polyline }`, przy czym `None` oznacza granicę z wodą lub krawędzią świata.
- Serializacja serde w camelCase. Nowe pola w `SettlementMap` z `#[serde(default)]`.

**Kryterium:** istniejące pola struktur się nie zmieniają, a dane serializują się poprawnie.

### Etap 2 — generowanie
Pliki: `world/borders.rs`, wywołanie w `generate_sampled` (`world/settlement.rs`) po budowie dróg.

1. Granice miast: wielokąt wokół osady z promieniem `radius × współczynnik typu` i szumem. Przycięty do lądu oraz do połowy odległości do sąsiedniej osady.
2. Wybór stolic: liczba państw zależy od liczby `City`/`Town`, w przedziale 2–6.
3. Przypisanie osad do stolic według kosztu po grafie dróg.
4. Terytorium: wielokrotne źródła Voronoi z wagą kosztu terenu (woda nieprzekraczalna, rzeka droższa) na siatce.
5. Kontury między komórkami różnych państw, wygładzone i uproszczone. Drobne enklawy poniżej progu zostają wchłonięte.

**Kryteria:** każda osada należy do dokładnie jednego państwa. Granice nie przecinają oceanu. Wielokąty miast są zamknięte i leżą na lądzie.

### Etap 3 — testy (`cargo test --lib`)
- Determinizm: dwa generowania z tym samym seedem dają równe granice.
- Seed `6`: osady i drogi bez zmian względem stanu sprzed zmiany.
- Inwarianty z etapu 2 oraz profile `seed % 6` bez zmian.
- Przypadki skrajne: brak `City`, jedna osada, archipelag.

### Etap 4 — renderowanie
Pliki: nowy `borders_draw.rs` (lub rozszerzenie `settlements_draw.rs`), `render.rs`, `session.rs`.

- Rysowanie według ustaleń 5–6. Warstwa leży nad ekologią i terenem, pod drogami i budynkami.
- `session.rs` udostępnia dane granic dla bieżącego LOD tak jak `view_settlements`. Granice są liczone raz i cache'owane razem z `settlements`.

**Kryterium:** granice miast i państw da się odróżnić od dróg, dzielnic i murów na każdym LOD.

### Etap 5 — UI
Plik: `app.rs`.

- Przełącznik „Granice” w panelu, domyślnie wyłączony. Zmiana przełącznika odświeża teksturę bez regeneracji świata.
- Opcjonalnie nazwa państwa w panelu osady (do potwierdzenia, ustalenie 7).

### Etap 6 — weryfikacja i dokumentacja
- `cargo test --lib` oraz `cargo run` na seedach 6 i 0–5 oraz kilku dodatkowych. Kontrola wizualna wszystkich trzech LOD z włączonym i wyłączonym przełącznikiem.
- Aktualizacja [aktualny_stan_prac.md](./aktualny_stan_prac.md) oraz adnotacja w [panstwa.md](./panstwa.md) o statycznych granicach.

## Lista plików

| Plik | Zmiana |
| --- | --- |
| `src-tauri/src/world/borders.rs` | nowy |
| `src-tauri/src/world/mod.rs` | rejestracja modułu |
| `src-tauri/src/world/settlement.rs` | nowe pola w `SettlementMap`, wywołanie generatora |
| `src-tauri/src/borders_draw.rs` lub `settlements_draw.rs` | rysowanie |
| `src-tauri/src/render.rs`, `session.rs` | przekazanie danych i kompozycja |
| `src-tauri/src/app.rs` | przełącznik |
| `docs/aktualny_stan_prac.md`, `docs/panstwa.md` | aktualizacja |

## Ryzyka

- Zmiana kolejności losowań w istniejących osadach zmieniłaby świat, dlatego granice używają osobnego strumienia PRNG.
- Archipelag i kontynenty wymagają osobnej obsługi wielu lądów.
- Koszt obliczeń konturów na dużych mapach. Cache razem z osadami ogranicza go do jednego razu na seed.
- Rozbieżność z zaleceniem w [panstwa.md](./panstwa.md) o braku globalnych granic. Ten plan dotyczy tylko statycznego podglądu.

## Kolejne iteracje (propozycja, 07.10.2026)

### Iteracja 2 — profil państwa i prawa

Zależności: granice z iteracji 1 oraz minimalny system frakcji z [panstwa.md](./panstwa.md).

| Etap | Zakres | Kryterium |
| --- | --- | --- |
| 2.1 Profil państwa | Ustrój (np. militarne, pacyfistyczne, anarchistyczne, wolne miasto) i cechy ambicji w `Realm`, z osobnego strumienia PRNG. | Ten sam seed daje ten sam profil. |
| 2.2 Prawa startowe | 1–2 prawa na państwo, np. stawka podatku i dostęp do dzielnic, obowiązujące w jego osadach. | Dla każdej osady da się ustalić obowiązujące prawo. |
| 2.3 Wpływ na cenę lub dostęp | Jedno prawo modyfikuje konkretną cenę, opłatę lub dostęp do zlecenia. | Zgodne z minimalnym zakresem z `panstwa.md`. |
| 2.4 UI | Panel osady pokazuje państwo, ustrój i prawo. | Gracz wie, kto ustala reguły w osadzie. |
| 2.5 Testy i dokumentacja | Determinizm, seed `6` bez zmian, aktualizacja dokumentów. | `cargo test --lib` przechodzi. |

### Iteracja 3 — relacje i odkrywanie granic

| Etap | Zakres | Kryterium |
| --- | --- | --- |
| 3.1 Relacje | Macierz relacji sąsiednich państw (neutralne, sojusz, napięcie) z ustrojów i sąsiedztwa. | Deterministyczna dla seeda. |
| 3.2 Odkrywanie | Granica widoczna po odwiedzeniu osady lub chunka po obu stronach albo po informacji od NPC. Przełącznik z POC zostaje jako tryb debugowania. | Granice poza zasięgiem poznania są ukryte. |
| 3.3 Informacje od NPC | Role NPC przekazują wiedzę o państwie i granicy. | Rozmowa odsłania granicę. |
| 3.4 Skutki na drodze | Przekroczenie granicy daje efekt, np. opłatę celną lub kontrolę, zależny od prawa i relacji. | Gracz widzi konsekwencję przejścia. |
| 3.5 Testy i dokumentacja | Testy odkrywania i relacji, aktualizacja dokumentów. | Seed `6` bez zmian. |

### Iteracja 4 — zmiany granic w sesji

Zależności: czas sesji, pętla ticków. Raster `owners` z seeda jest punktem startu; **sesja może go mutować** po ugodzie wojennej (etap C dyplomacji). Pełny zapis/wczytanie nakładki — osobno.

| Etap | Zakres | Kryterium |
| --- | --- | --- |
| 4.1 Warstwa nakładki | `settlement_overlays` w sesji z różnicami względem granic z seeda. | Baza z seeda nie jest modyfikowana. |
| 4.2 Rozrost miast | Wzrost populacji i migracje zmieniają promień granicy miasta. | Zmiana widoczna i zapisana w historii. |
| 4.3 Zmiany terytorium | **Częściowo w kodzie:** ugoda po wojnie anektuje okupowane komórki do `TerritoryMap.owners` i przebudowuje `realm_borders` (`rebuild_realm_borders_from_owners`). Bez zmiany przynależności osad w v0. | Wash + linie granic po rozejmie. |
| 4.4 Konflikty | Wojna abstrakcyjna + okupacja (claim) + hatch — zob. [panstwa.md](./panstwa.md). | Gracz widzi skutek na mapie. |
| 4.5 Zapis i wczytanie | Zapis nakładki z sesją, odtwarzanie z `seed + nakładka`. | Wczytanie daje identyczne granice. |
| 4.6 Testy i dokumentacja | Determinizm baza + nakładka, odtwarzanie zapisu. | Seed `6` bez zmian. |

Poza zakresem wszystkich iteracji: pełna dyplomacja, wojny z armiami, zmiana ustroju w trakcie gry, ekspansja kolonialna i rasy jako czynnik granic.

## Poza zakresem iteracji 1

Rozrost i zmiany granic (kolejne iteracje), ustroje i ambicje państw, migracje, konflikty terytorialne, dyplomacja, podatki, cła, prawa oraz zapis zmian granic w sesji. Wchodzą one w kolejnych iteracjach.
