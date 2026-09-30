# Checklista parytetu migracji Bevy

Ręczne scenariusze dla funkcji obecnych przed migracją. Nie zawiera pozycji z `plans.md`.

Baseline: seed `6` (Ellipse). Dodatkowo co najmniej jeden seed dla każdego profilu: `0` Radial, `2` SquareBump, `3` Irregular, `4` Archipelago, `5` Continents (lub równoważne `seed % 6`).

## Generator i parametry

- [ ] Seed `u64`, domyślnie `6`; wpisywanie, walidacja, Losuj seed, opis seeda, Regenerate
- [ ] Profile `seed % 6`: Radial, Ellipse, SquareBump, Irregular, Archipelago, Continents
- [ ] Suwaki `TerrainGenParams` (wartości i znaczenie bez zmian)
- [ ] Determinizm terenu/rzek/osad/ekologii dla tego samego seeda i parametrów
- [ ] Stan ładowania; niedostępne operacje zablokowane podczas generowania

## Mapa i LOD

- [ ] LOD: świat → region → obszar/chunk; klik poza osadą schodzi niżej; przycisk powrotu wraca wyżej
- [ ] Cache widoków LOD; osady/NPC reuse przy zmianie samego LOD
- [ ] Mapowanie kursora na współrzędne świata przy skalowaniu / resize
- [ ] Teren, biomy, cień wysokości, AA wybrzeża, siatka regionów, skala mapy
- [ ] Drogi, mosty, markery/footprinty osad, dzielnice; style per LOD
- [ ] Nakładka siedlisk (region), ekologia (obszar), legendy biomów/osad/ekologii
- [ ] Fog of war: zasięg widzenia, odkryte obszary, marker gracza

## Wybór miejsca i informacje

- [ ] Klik osady → info, bez zmiany LOD
- [ ] Klik dzielnicy → info (jeśli dostępne)
- [ ] Klik obszaru → biom + podsumowanie gatunków
- [ ] Lista miejsc (warunki, pozycje, wybór celu)
- [ ] Teksty/etykiety/zakres danych bez zmiany znaczenia

## Gracz, ruch i czas

- [ ] Spawn na lądzie: wybór, potwierdzenie, anulowanie
- [ ] Ruch pieszy do celu, fokus na graczu, ruch kierunkowy, cel z listy
- [ ] Przechodniość, prędkość off-road/on-road, snap do dróg, trasa
- [ ] Aktualizacja chunka po granicy; visited places / known chunks
- [ ] Zegar: start, pauza/wznowienie, mnożniki, +1 h; wpływ na ruch
- [ ] Etykiety ruchu i statusów gracza

## Stan i diagnostyka

- [ ] FPS i CPU% (jeśli platforma udostępnia)
- [ ] Info generowania, status mapy, błędy formularza
- [ ] `cargo test --lib` przechodzi; determinizm seed `6` = Ellipse
