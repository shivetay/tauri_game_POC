# NPC — zależności i plan wdrożenia

## Rola

NPC zamieniają dane świata w informacje i interakcje. Są mieszkańcami konkretnych osad/dzielnic, a nie niezależnymi od geografii punktami dialogowymi.

## Od czego zależą NPC

- Od osady, populacji, typu i dzielnicy.
- Od potrzeb i usług osady oraz lokalnej frakcji.
- Od dróg, czasu i wydarzeń, jeśli NPC mają się przemieszczać lub reagować.
- Od seeda przy generowaniu tożsamości bazowej; zmienne relacje należą do sesji.

**Łańcuch:** `osada + populacja + dzielnice → liczba/role NPC → usługi, wiedza i zadania → interakcje gracza → relacje i skutki świata`.

## Co NPC udostępniają innym systemom

- Informacje o odkrytych miejscach, drogach i wydarzeniach.
- Handel, usługi, pracę i zlecenia.
- Dialogi i relacje powiązane z reputacją, rasą, profesją i historią postaci.
- Widoczne reakcje na zmiany w osadzie, frakcji i lokalnej gospodarce.

## Wpływ na postać i świat

- NPC przekazują wiedzę, która może odsłaniać cele i miejsca do eksploracji.
- Rozmowa, handel, praca i decyzje zmieniają relacje lub reputację.
- Zlecenia NPC łączą potrzeby osady z podróżą i zasobami biomów.
- Harmonogramy i symulacja ruchu mogą wpływać na dostępność usług, ale nie są potrzebne do pierwszej pętli.

## Stan wdrożenia (kod)

Szczegóły cech i ruchu: [npc-cechy.md](./npc-cechy.md). Budynki i zapasy: [budynki-stan.md](./budynki-stan.md).

| Obszar | Stan |
| --- | --- |
| Imię, rola, osada/dzielnica (seed) | wdrożone |
| Cechy, `LifeGoal`, wygląd, skills, `inclination` | wdrożone |
| Home / cel zaopatrzenia, potrzeby, stock | wdrożone |
| Wiedza o sąsiednich osadach (`known_settlement_indices`) | wdrożone |
| Symulacja tła: lokalny ruch + wyjazd między osadami po drogach | wdrożone (bez markerów) |
| NPC→NPC: pytanie o drogę (`ShareLocalInfo`) | wdrożone (tło) |
| Dialog / handel / zlecenia z graczem | brak |
| Markery NPC na mapie, trwała migracja | brak |
| Zapis relacji w sesji / save | brak |

## Kolejność wdrażania

1. Zachować deterministyczne imiona/role powiązane z osadą i dzielnicą. *(zrobione)*
1a. Cechy liczbowe + API skłonności. *(zrobione)*
1b. Potrzeby, budynki, lokalny cykl aktywności. *(zrobione)*
1c. Wiedza o osadach + podróż między miastami w tle + NPC→NPC. *(zrobione)*
2. Nadać rolom proste, działające funkcje wobec gracza: informator, sprzedawca, zleceniodawca.
3. Dodać dialog/zlecenie zależne od stanu osady, reputacji i tagów historii postaci.
4. Zapisywać relacje oraz zakończenie rozmów/zleceń w stanie sesji.
5. Markery / widoczny ruch na drodze; opcjonalnie harmonogramy w aktywnej okolicy.
6. Dla bardzo odległego świata rozważyć agregaty zamiast pełnego ticka każdej osoby (obecnie tick tła jest abstrakcyjny: ETA po grafie, bez polilinii).

## Minimalny zakres

W każdej osadzie gracz może spotkać co najmniej jedną osobę z użyteczną funkcją: otrzymać informację, handlować albo przyjąć zlecenie.

## Kryterium ukończenia

NPC ma spójną osadę/rolę, interakcja czyta stan gry i daje widoczny skutek, a wynik jest zachowany w zapisie.
