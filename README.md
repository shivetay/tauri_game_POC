# map_tests

Podgląd proceduralnej mapy świata (**Rust + egui**). Teren, rzeki, osady i ekologia liczone są w tym samym procesie co UI. **Ten sam seed i te same parametry zawsze dają ten sam świat.**

```bash
cd src-tauri
cargo run
```

Testy generatora:

```bash
cd src-tauri
cargo test --lib
```

## Szybki start

1. Wpisz seed (nieujemna liczba całkowita) albo kliknij **Losuj seed**.
2. Opcjonalnie ustaw suwaki (0–1) — zapisują się dopiero po **Regenerate**.
3. Kliknij **Regenerate** (lub **Pokaż opis**, by zobaczyć skrót o `seed % 6`).
4. Klik w teren: świat → region → obszar. Przycisk **← …** wraca wyżej.
5. Klik w osadę pokazuje opis (bez zmiany LOD). Na obszarze: klik w teren → biom + gatunki w zasięgu.

Domyślny seed: `6` → profil **Ellipse**.

## Seed i kształt lądu

Seed to `u64`. Profil kształtu: `seed % 6`.

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
| Świat | Biomy, szlaki główne, miasta i miasteczka (markery) |
| Region | Biomy, wash siedlisk, wszystkie osady (plany), drogi |
| Obszar | Biomy, wash życia, footprinty osad, drogi z mostami; klik → biom / gatunki |

## Warstwy świata

- **Teren / biomy** — wysokość (m, poziom morza = 0), wilgotność, klasyfikacja biomu.
- **Rzeki** — sieć źródła → spływ; koryto wycinane w teren; mosty na drogach.
- **Osady** — Hamlet / Village / Town / City (dzielnice, ulice); drogi Highway / Secondary / Local.
- **Ekologia** — w regionie: potencjał siedliska; w obszarze: flora i fauna (klik w zasięgu).

## Architektura

| Ścieżka | Rola |
| --- | --- |
| `src-tauri/src/world/` | Generacja (jedyna prawda terenu) |
| `src-tauri/src/app.rs` | Okno egui, seed/params, LOD, legendy |
| `src-tauri/src/render.rs` | Składanie tekstury mapy (teren + overlaye) |
| `src-tauri/src/colors.rs` | Kolory biomów |
| `src-tauri/src/settlements_draw.rs` | Rysowanie osad i dróg (RGBA) |
| `src-tauri/src/ecology_draw.rs` | Wash siedlisk + podsumowania życia |

Generacja działa w **osobnym wątku**; UI dostaje gotową siatkę i buduje teksturę lokalnie (bez IPC / JSON).

Więcej: `docs/seed.md` (seed), `docs/plans.md` (kierunek pod grę), `AGENTS.md` (kontrakt dla agentów).
