# Aktualny stan prac

**Stan na: 14.10.2026**

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
- Zegar gry z pauzą, prędkościami ×1/×10/×60 oraz ręcznym przesunięciem o godzinę; czas podróży aktualizuje pozycję gracza.
- Wskaźnik cyklu dnia/nocy pod zegarem (po spawnie): oś wschód–południe–zachód ze słońcem (06–18) i księżycem (18–06); czas gry rusza dopiero po spawnie; mapa nie jest tintowana.
- Kalendarz gry: rok 360-dniowy (12×30), etykieta zegara z rokiem, dniem i miesiącem; cztery pory roku (Wiosna/Lato/Jesień/Zima, po 3 miesiące). Wygląd mapy nadal niezależny od pory roku.
- Aplikacja desktopowa oparta o Bevy i egui.

## W trakcie

- Dalsza praca nad spójnością mostów / przebiegu dróg (osobny wątek generatora; nie blokuje podróży NPC po grafie osad).

## Jeszcze niezaimplementowane systemy rozgrywki

- Ekwipunek, zbieranie zasobów i potrzeby survivalowe.
- Handel, dynamiczna gospodarka, praca, zlecenia i questy.
- Zapis i wczytywanie stanu sesji.
- Handel z graczem, markery NPC na mapie.
- Dialog gracza z NPC (informator / handel) i zlecenia; zapis relacji w sesji.
- Trwała migracja populacji, harmonogramy NPC, frakcje oraz wydarzenia świata.
- Rozgrywkowe delty terenu i infrastruktura gracza.
- Wpływ pory roku na wygląd mapy (etap 4 z issue #8).

## Zasady aktualizacji

Każdy agent po zadaniu, które zmienia stan projektu, aktualizuje tę stronę: odświeża datę, przenosi ukończone elementy do „Zaimplementowane”, poprawia sekcje „W trakcie” i „Jeszcze niezaimplementowane” oraz usuwa informacje, które przestały być prawdziwe. Opis ma odzwierciedlać działający kod, nie sam plan ani intencję.
