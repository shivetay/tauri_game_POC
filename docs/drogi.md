# Drogi — zależności i plan wdrożenia

## Rola

Drogi są grafem łączącym osady, dzielnice i przeprawy. Z mapowego połączenia mają wynikać podróż, przewóz towarów, ruch NPC i bezpieczeństwo.

## Od czego zależą drogi

- Od położeń osad i miejsc, które mają być połączone.
- Od terenu, wysokości, rzek i dostępnych przepraw.
- Od rodzaju połączenia oraz znaczenia połączonych osad.

**Łańcuch:** `teren + rzeki/przeprawy + osady → graf dróg → czas i koszt podróży → handel, zlecenia, ruch NPC i wydarzenia`.

## Co drogi udostępniają innym systemom

- Trasę dla gracza i wyznaczanie czasu podróży.
- Korytarze handlu i przewozu między osadami.
- Trasy karawan i podróży NPC.
- Miejsca ryzyka (np. napad), opłaty, blokady i cele wydarzeń.
- Infrastrukturę, którą gracz może później budować lub naprawiać.

## Wpływ na postać i świat

- Droga skraca podróż i może zmniejszać jej koszt, ale jej stan lub zagrożenia wpływają na ryzyko.
- Dostawy i praca przewoźnika wymagają połączenia między źródłem a celem.
- Bezpieczeństwo dróg wpływa na dostępność towarów, ceny i podróże NPC.
- Budowa, naprawa, blokada i opłaty zmieniają stan sesji; nie przebudowują po cichu bazowej, deterministycznej sieci.

## Kolejność wdrażania

1. Zachować deterministyczną generację dróg i poprawne połączenia przez rzeki/mosty.
2. Udostępnić graf jako wspólną podstawę pathfindingu postaci i przewozu.
3. Powiązać typ i stan drogi z czasem podróży oraz prostym kosztem.
4. Użyć istniejących tras w zleceniach, handlu i ruchu NPC.
5. Dodać stan bezpieczeństwa, naprawy i blokady jako delty zapisanej sesji.
6. Rozszerzać transport rzeczny i strategiczne przeprawy dopiero po działaniu handlu lądowego.

## Minimalny zakres

Na początek wystarczy przejście po istniejących drogach z czasem zależnym od typu trasy oraz dostawa pomiędzy dwiema połączonymi osadami.

## Kryterium ukończenia

Postać i dostawa mogą skorzystać z tego samego grafu trasy, a prędkość/czas podróży są przewidywalne. Mosty i przeprawy działają spójnie, a lokalne zmiany drogi można zapisać i odtworzyć.
