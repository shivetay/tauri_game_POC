# Aktualny stan prac

**Stan na: 19.10.2026**

## Podsumowanie

Projekt działa jako desktopowy prototyp mapy proceduralnego świata z podstawową pętlą podróży postaci. Generator i podgląd świata są rozwinięte; systemy właściwej rozgrywki RPG są jeszcze w większości planem.

## Zaimplementowane

- Deterministyczne generowanie świata z seeda i parametrów; sześć profili kształtu oraz trzy poziomy widoku: świat, region i chunk.
- Teren, wysokości, biomy, sieć rzek oraz generowane osady z populacją, dzielnicami, drogami i mostami.
- Warstwy ekologii dla regionów i chunków oraz flora i fauna prezentowane w UI.
- Postacie niezależne: cechy, skłonności, potrzeby, home/cel zaopatrzenia, wiedza o sąsiednich osadach; symulacja tła wszystkich NPC — lokalny ruch offroad, podróż między osadami po grafie dróg, pytanie NPC→NPC o drogę; statusy w liście Lokalizacja — bez markerów na mapie. Szczegóły: [npc-cechy.md](./npc-cechy.md).
- Po spawnie: panel Lokalizacja (NPC | Budynki), budynki z `(x,y)` i dynamicznym zapasem Sell (także mutacje z podróży między osadami). Szczegóły: [budynki-stan.md](./budynki-stan.md).
- Wybór miejsca startu i spawn postaci na mapie.
- Podróż piesza po lądzie, preferowanie grafu dróg, przejście między poziomami LOD, odkrywanie chunków i znanych osad.
- Zegar gry z pauzą, prędkościami ×1/×10/×60 oraz ręcznym skokiem: +1 h / dzień / tydzień / miesiąc / rok; czas podróży aktualizuje pozycję gracza.
- Wskaźnik cyklu dnia/nocy pod zegarem (po spawnie lub w symulacji świata): oś wschód–południe–zachód ze słońcem (06–18) i księżycem (18–06); mapa nie jest tintowana.
- Przycisk **Symulacja (bez gracza)**: uruchamia zegar, NPC i dyplomację bez spawnu / fog / markera; osobny stop. Spawn gracza wyłącza ten tryb i resetuje czas.
- Kalendarz gry: rok 360-dniowy (12×30), etykieta zegara z rokiem, dniem i miesiącem; cztery pory roku (Wiosna/Lato/Jesień/Zima, po 3 miesiące). Wygląd mapy nadal niezależny od pory roku.
- Aplikacja desktopowa oparta o Bevy i egui.
- Państwa (`Realm`): unikalne nazwy; profil (władca, ustrój, kultura, religia, ambicja, sława, liczebność, siła wojskowa, cele tylko do odczytu). Statyczny handel: eksport/import + szlaki. Dyplomacja v0 (sesja): napięcie −100…100; `RelationKind` w dossier. Wojna: casus nie tylko podbój; okupacja tylko przy `ExpandBorders`; ograniczona wojna kończy się szybciej bez pełnego zajęcia; trzecie państwo może embargo **lub** wypowiedzieć wojnę już walczącym. Ugoda v0: aneksja tego, co okupowane → `owners` + polyline; osady bez zmiany przynależności. Granice → etykieta; klik → dossier.
- Prawa państwowe (etap A + UI): `Realm.laws` generowane z religii/kultury/ustroju/ambicji oraz pojemności administracyjnej z budynków; gwarantowany `Hall` w stolicy przy świeżej generacji budynków; dossier państwa pokazuje listę praw. Bez filtrów handlu — [prawa.md](./prawa.md).
- Nazwy osad na mapie nad markerem: świat — miasta i miasteczka; region — też wsie; chunk — wszystkie; bez nakładania; pod mgłą tylko odkryte.

## W trakcie

- Dalsza praca nad spójnością mostów / przebiegu dróg (osobny wątek generatora; nie blokuje podróży NPC po grafie osad).
- Plan aneksji osad po wojnie (E1–E5): dokument [aneksja-osad.md](./aneksja-osad.md) — kod jeszcze nie rozpoczęty; kolejny slice: E1+E2.

## Jeszcze niezaimplementowane systemy rozgrywki

- Ekwipunek, zbieranie zasobów i potrzeby survivalowe.
- Handel, dynamiczna gospodarka, praca, zlecenia i questy.
- Zapis i wczytywanie stanu sesji.
- Handel z graczem, markery NPC na mapie.
- Dialog gracza z NPC (informator / handel) i zlecenia; zapis relacji w sesji.
- Trwała migracja populacji, harmonogramy NPC, frakcje oraz wydarzenia świata.
- Aktywna polityka państw: **przepisanie osad po ugodzie**, opór/bunt/interwencja/secesja ([aneksja-osad.md](./aneksja-osad.md)); ekspansja poza wojną; reputacja; cele runtime; prawa — skutek na handel + panel osady ([prawa.md](./prawa.md) B–C).
- Rozgrywkowe delty terenu i infrastruktura gracza.
- Wpływ pory roku na wygląd mapy (etap 4 z issue #8).

## Zasady aktualizacji

Każdy agent po zadaniu, które zmienia stan projektu, aktualizuje tę stronę: odświeża datę, przenosi ukończone elementy do „Zaimplementowane”, poprawia sekcje „W trakcie” i „Jeszcze niezaimplementowane” oraz usuwa informacje, które przestały być prawdziwe. Opis ma odzwierciedlać działający kod, nie sam plan ani intencję.
