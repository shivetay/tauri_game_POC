# Budynki — stan wdrożenia

Dokument opisuje to, co jest w kodzie. Pełna lista propozycji: [budynki.md](./budynki.md), profesje: [profesje-budynki.md](./profesje-budynki.md). NPC i podróż między osadami: [npc-cechy.md](./npc-cechy.md).

## Co jest wdrożone

- Deterministyczna generacja budynków z seeda (`src-tauri/src/world/building.rs`).
- Osada z dzielnicami: 2–4 budynki na dzielnicę z puli dzielnicy.
- Osada bez dzielnic: 3–5 budynków z puli podstawowej typu osady.
- Statyczne potrzeby budynku: `Buy` / `Sell` + `Good` (np. kram kupuje skóry, sprzedaje towary).
- Zapas sprzedaży (`stock`): start ze seeda; **dynamiczny globalnie** w sesji — kupno NPC (lokalne i w innej osadzie) zmniejsza zapas, praca/zbiór w domu go zwiększa. UI Lokalizacja pokazuje zapas tylko w osadzie gracza.
- Budynki mają pozycję świata `(x, y)` (centroid dzielnicy / jitter w osadzie) — cel nawigacji NPC w sesji.
- NPC: home, cel zaopatrzenia ≠ home; przy braku lokalnego stocku wyjazd do znanej osady z towarem (graf dróg) albo pytanie innego NPC o drogę.
- Po spawnie, panel **Lokalizacja**: zakładki **NPC | Budynki**.
- Budynki i NPC reuse przy zmianie LOD; regeneracja przy zmianie seeda/parametrów. Generacja: buildings → npcs (z drogami).

## Czego świadomie nie robi ten etap

- Markerów NPC / budynków na mapie.
- Handlu z graczem i dialogu.
- Pełnej listy typów z `budynki.md` (pule świadomie okrojone).
- Budowy / zniszczenia budynków w sesji.

## Dalsze kroki

1. Handel z graczem.
2. Rozszerzenie pul typów i powiązanie z profesjami.
3. Warstwa sesji (budowa / zniszczenie) bez mutacji bazowego seeda.
