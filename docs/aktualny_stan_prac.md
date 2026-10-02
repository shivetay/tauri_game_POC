# Aktualny stan prac

**Stan na: 01.10.2026**

## Podsumowanie

Projekt działa jako desktopowy prototyp mapy proceduralnego świata z podstawową pętlą podróży postaci. Generator i podgląd świata są rozwinięte; systemy właściwej rozgrywki RPG są jeszcze w większości planem.

## Zaimplementowane

- Deterministyczne generowanie świata z seeda i parametrów; sześć profili kształtu oraz trzy poziomy widoku: świat, region i chunk.
- Teren, wysokości, biomy, sieć rzek oraz generowane osady z populacją, dzielnicami, drogami i mostami.
- Warstwy ekologii dla regionów i chunków oraz flora i fauna prezentowane w UI.
- Postacie niezależne generowane deterministycznie dla osad i dzielnic; podgląd imion i ról w panelu osady.
- Wybór miejsca startu i spawn postaci na mapie.
- Podróż piesza po lądzie, preferowanie grafu dróg, przejście między poziomami LOD, odkrywanie chunków i znanych osad.
- Zegar gry z pauzą, prędkościami ×1/×10/×60 oraz ręcznym przesunięciem o godzinę; czas podróży aktualizuje pozycję gracza.
- Wskaźnik cyklu dnia/nocy pod zegarem (po spawnie): oś wschód–południe–zachód ze słońcem (06–18) i księżycem (18–06); czas gry rusza dopiero po spawnie; mapa nie jest tintowana.
- Aplikacja desktopowa oparta o Bevy i egui.

## W trakcie

- Lokalne, niezacommitowane zmiany w `src-tauri/src/world/settlement.rs` dotyczą spójności przebiegu dróg i rejestru mostów.

## Jeszcze niezaimplementowane systemy rozgrywki

- Ekwipunek, zbieranie zasobów i potrzeby survivalowe.
- Handel, dynamiczna gospodarka, praca, zlecenia i questy.
- Zapis i wczytywanie stanu sesji.
- Symulacja zmian populacji, aktywności i harmonogramów NPC, frakcji oraz wydarzeń świata.
- Rozgrywkowe delty terenu i infrastruktura gracza.
- Kalendarz, pory roku oraz ich wpływ na wygląd mapy.

## Zasady aktualizacji

Każdy agent po zadaniu, które zmienia stan projektu, aktualizuje tę stronę: odświeża datę, przenosi ukończone elementy do „Zaimplementowane”, poprawia sekcje „W trakcie” i „Jeszcze niezaimplementowane” oraz usuwa informacje, które przestały być prawdziwe. Opis ma odzwierciedlać działający kod, nie sam plan ani intencję.
