# Budynki — stan wdrożenia

Dokument opisuje to, co jest w kodzie. Pełna lista propozycji: [budynki.md](./budynki.md), profesje: [profesje-budynki.md](./profesje-budynki.md).

## Co jest wdrożone

- Deterministyczna generacja budynków z seeda (`src-tauri/src/world/building.rs`).
- Osada z dzielnicami: 2–4 budynki na dzielnicę z puli dzielnicy.
- Osada bez dzielnic: 3–5 budynków z puli podstawowej typu osady.
- Statyczne potrzeby budynku: `Buy` / `Sell` + `Good` (np. kram kupuje skóry, sprzedaje towary).
- Zapas sprzedaży (`stock`): start ze seeda; w osadzie gracza **dynamiczny** — kupno NPC zmniejsza zapas, praca/zbiór w domu go zwiększa. UI: zapas obok nazwy budynku.
- Budynki mają pozycję świata `(x, y)` (centroid dzielnicy / jitter w osadzie) — cel nawigacji NPC w sesji.
- NPC: home, cel zaopatrzenia ≠ home; w osadzie gracza ruch offroad + statusy aktywności (lista Lokalizacja, bez markerów mapy).
- Po spawnie, panel **Lokalizacja**: zakładki **NPC | Budynki**.
- Budynki i NPC reuse przy zmianie LOD; regeneracja przy zmianie seeda/parametrów. Generacja: buildings → npcs.

## Czego świadomie nie robi ten etap

- Ruchu NPC na mapie do budynku-celu.
- Zużycia / odnawiania zapasu w czasie gry.
- Handlu z graczem i dialogu.
- Rysowania budynków na mapie.
- Pełnej listy typów z `budynki.md` (pule świadomie okrojone).

## Dalsze kroki

1. Stub transakcji (zmniejszenie stock przy „zakupie” NPC) w warstwie sesji.
2. Fizyczny lub animowany ruch do `supply_target`.
3. Rozszerzenie pul typów i powiązanie z profesjami.
4. Warstwa sesji (budowa / zniszczenie) bez mutacji bazowego seeda.
