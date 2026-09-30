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

## Kolejność wdrażania

1. Zachować deterministyczne imiona/role powiązane z osadą i dzielnicą.
2. Nadać rolom proste, działające funkcje: informator, sprzedawca, zleceniodawca.
3. Dodać dialog/zlecenie zależne od stanu osady, reputacji i tagów historii postaci.
4. Zapisywać relacje oraz zakończenie rozmów/zleceń w stanie sesji.
5. Dodać harmonogramy i przemieszczanie tylko dla aktywnej okolicy.
6. Dla reszty świata utrzymać agregaty populacji zamiast szczegółowej symulacji każdej osoby.

## Minimalny zakres

W każdej osadzie gracz może spotkać co najmniej jedną osobę z użyteczną funkcją: otrzymać informację, handlować albo przyjąć zlecenie.

## Kryterium ukończenia

NPC ma spójną osadę/rolę, interakcja czyta stan gry i daje widoczny skutek, a wynik jest zachowany w zapisie.
