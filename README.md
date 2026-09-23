# map_tests

Podgląd proceduralnej mapy świata (Tauri 2 + React + Rust). Teren, rzeki, osady i ekologia liczone są w Rust; frontend tylko rysuje. **Ten sam seed i te same parametry zawsze dają ten sam świat.**

```bash
pnpm install
pnpm tauri dev
```

Aplikacja działa wyłącznie w oknie Tauri — nie otwieraj `localhost:1420` w przeglądarce.

## Szybki start

1. Wpisz seed (nieujemna liczba całkowita) albo kliknij **Losuj seed**.
2. Opcjonalnie ustaw suwaki (0–1) — zapisują się dopiero po **Regenerate**.
3. Kliknij **Regenerate**.
4. Klik w teren: świat → region → obszar. Przycisk wstecz wraca wyżej. Klik w osadę pokazuje opis, bez zmiany LOD.

Domyślny seed: `6` → profil **Ellipse**.

## Seed i kształt lądu

Seed to `u64` / liczba. Profil kształtu: `seed % 6`.

| `seed % 6` | Profil | Opis |
| --- | --- | --- |
| 0 | Radial | Jedna okrągła wyspa |
| 1 | Ellipse | Jedna wyspa, spłaszczona w poziomie lub pionie |
| 2 | SquareBump | Ląd przy krawędziach, woda w centrum |
| 3 | Irregular | Jedna wyspa z zatokami i półwyspami |
| 4 | Archipelago | 3–6 osobnych wysp |
| 5 | Continents | 2–3 większe masywy |

Przykłady: `0` Radial, `6` Ellipse, `2` SquareBump, `11` Continents (`11 % 6 = 5`).

Ten sam seed steruje też wysokością, wilgotnością, pasmami gór, wybrzeżem, rzekami, osadami i życiem. Suwaki tylko skalują siłę efektu (0 = wyłączone / minimalne, 1 = domyślna pełnia).

| Suwak | Efekt |
| --- | --- |
| Ukształtowanie | Pasma gór i depresje |
| Wilgotność | Rozrzut biomów |
| Detal micro | Drobny relief w zbliżeniu |
| Falistość terenu | Bazowe wzniesienia |
| Powierzchnia lądu | Ile lądu względem oceanu |
| Nieregularność brzegu | Zatoki i półwyspy |

## Mapa (LOD)

Świat: 512×512 jednostek. Region: 64×64. Obszar (chunk): 8×8.

| Poziom | Co widać |
| --- | --- |
| Świat | Biomy, główne drogi, miasta i miasteczka |
| Region | Biomy, wszystkie osady, drogi, warstwa siedlisk |
| Obszar | Biomy, obrys miasta z dzielnicami, drogi z mostami, flora i fauna |

## Warstwy świata

- **Teren / biomy** — wysokość (m, poziom morza = 0), wilgotność, klasyfikacja biomu (woda, piasek, pustynia, trawa, las, góry, śnieg itd.).
- **Rzeki** — sieć źródła → spływ → dopływy / delty / jeziora; koryto wycinane w teren (widać jako wodę); brzegi wilgotniejsze; wpływ na lokalizację osad i mosty na drogach.
- **Osady** — Hamlet / Village / Town / City (polskie nazwy, populacja); większe mają dzielnice i ulice.
- **Drogi** — Highway / Secondary / Local, nawierzchnia i mosty nad rzekami.
- **Ekologia** — w regionie: potencjał siedliska; w obszarze: konkretne gatunki flory i fauny.

## Architektura (skrót)

- Generacja: `src-tauri/src/world/`
- Komendy Tauri: `src-tauri/src/lib.rs`
- Wywołania z UI: `src/api/world.ts`
- Widoki LOD: `GlobalMapView` → `RegionMapView` → `ChunkMapView`
