# Seed w generatorze map

## Cel

Ten dokument opisuje, jak działa seed w **aktualnym** kodzie projektu (Rust + egui).

## Co to jest seed

Seed to wartość startowa generatora pseudolosowego. Przy tym samym seedzie, algorytmie i parametrach wynik jest zawsze ten sam — pozwala odtworzyć identyczny świat.

## Jak seed działa np. w Minecraft

1. Świat dostaje seed główny.
2. Na jego podstawie inicjalizowane są wewnętrzne generatory.
3. Generator łączy seed z położeniem, żeby policzyć teren, biomy itd.
4. Ten sam seed w tej samej wersji generatora daje ten sam świat.

Tekstowy seed (jeśli jest) najpierw zamienia się na liczbę (hash albo parse). Znaczenie słowa nie ma znaczenia — liczy się liczba wynikowa.

## Stan w tym projekcie

Seed jest **`u64`** od wejścia do wyjścia.

| Warstwa | Typ | Uwagi |
| --- | --- | --- |
| UI (`app.rs`) | `u64` + draft jako `String` w polu tekstowym | Regenerate parsuje nieujemną liczbę całkowitą |
| `WorldConfig::seed` | `u64` | Domyślnie `6` |
| Profil kształtu | `seed % 6` | Radial … Continents |
| Pochodne strumienie | `prng` / `unit_noise` z solami | np. wilgotność, detal, wybrzeże — nie osobny globalny RNG |

Domyślny seed aplikacji: **`6`** → profil **Ellipse**.

### Profile kształtu (`seed % 6`)

| Reszta | Profil |
| --- | --- |
| 0 | Radial |
| 1 | Ellipse |
| 2 | SquareBump |
| 3 | Irregular |
| 4 | Archipelago |
| 5 | Continents |

### Parametry (`TerrainGenParams`)

Suwaki UI (0–1) trafiają do `TerrainGenParams` i przez `apply_to` skalują `WorldConfig` (orogeny, moisture, detail, roughness, land size, coast). Nie zmieniają seeda — tylko intensywność efektów.

### Determinizm

- Ten sam `(seed, params)` → ta sama siatka terenu, te same rzeki, osady i ekologia.
- Po zmianie generatora: baseline regresji to seed `6` (Ellipse); pozostałe `seed % 6` muszą mapować na te same profile.

### Uruchomienie / testy

```bash
cd src-tauri
cargo run          # UI
cargo test --lib   # testy world/
```

## Historia (krótko)

Wcześniej seed bywał stringiem hashowanym do liczby, a UI szło przez Tauri + React (`invoke`). To zostało usunięte: aplikacja jest czystym Rustem (egui), seed jest `u64`. Stary plan migracji tekst→liczba (`docs/seed_change.md`) jest **zrealizowany / nieaktualny**.

## Powiązane pliki

- `src-tauri/src/world/config.rs` — `WorldConfig`, `TerrainGenParams`, `ShapeProfile`
- `src-tauri/src/world/prng.rs` — pochodne z `u64`
- `src-tauri/src/app.rs` — pole seed / Regenerate / Losuj
- `README.md` — skrót dla użytkownika
