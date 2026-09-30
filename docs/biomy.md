# Biomy — zależności i plan wdrożenia

## Rola

Biom opisuje lokalne warunki świata. Dla gracza powinien przekładać się na czytelne zasoby, ryzyka, możliwości pracy i koszt podróży, a nie być wyłącznie kolorem mapy.

## Od czego zależy biom

- Deterministycznego seeda i parametrów świata.
- Ukształtowania terenu, wysokości i wilgotności.
- Rzek, jezior, wybrzeża oraz lokalnej ekologii.

**Łańcuch:** `seed + parametry → teren/wysokość/wilgotność + woda → biom → zasoby/ryzyka → aktywności gracza i osad`.

## Co biom udostępnia innym systemom

- Zasoby do zbierania i podstawę lokalnej produkcji.
- Modyfikatory podróży oraz zagrożenia środowiskowe.
- Warunki osadnictwa i specjalizację osad.
- Kontekst wydarzeń, pogody i ekologii.

## Wpływ na postać i świat

- Postać może zbierać surowce, polować lub podejmować pracę zależną od biomu.
- Podróż przez trudny biom kosztuje więcej czasu lub wymaga przygotowania.
- Zasoby biomu wpływają na zapasy i ceny osad, a przez to na handel i zlecenia.
- Zbieranie, wycinka i inne zmiany są deltami sesji; nie zmieniają bazowego biomu wygenerowanego z seeda.

## Kolejność wdrażania

1. Utrzymać biom bazowy jako wynik generatora i deterministycznych próbek świata.
2. Zdefiniować małą tabelę `biom → zasoby, ryzyka, koszt ruchu`.
3. Udostępnić tę tabelę zbieraniu, potrzebom postaci i lokalnej produkcji osad.
4. Powiązać dostępność surowców z cenami, pracą i zleceniami.
5. Dodać pogodę/sezony i regenerację zasobów dopiero z zapisywanym stanem sesji.
6. Testować, że ten sam seed i parametry nadal dają te same biomy; efekty gracza odtwarza zapis sesji.

## Minimalny zakres

Na początek każdy biom ma prostą listę zasobów oraz najwyżej jeden istotny modyfikator ryzyka albo podróży. System nie powinien wymagać pełnej symulacji ekologii.

## Kryterium ukończenia

Gracz może rozpoznać biom, zrozumieć jego podstawowy zasób i ryzyko oraz zobaczyć, jak wpływają one na zbieranie, podróż albo osadę. Po ponownym wygenerowaniu świata biom bazowy pozostaje zgodny z seedem.
