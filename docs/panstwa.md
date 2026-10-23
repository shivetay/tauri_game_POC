# Państwa i frakcje — zależności i plan wdrożenia

## Rola

Państwo/frakcja organizuje grupę osad, terytorium i reguły, które zmieniają warunki życia. System powinien najpierw wspierać małą, czytelną skalę lokalną.

## Od czego zależą państwa i frakcje

- Od osad, populacji, zasobów, dróg i relacji pomiędzy osadami.
- Od celów i ustroju frakcji, które określają zachowanie oraz decyzje.
- Od granic/terytoriów, które przypisują osady i szlaki do władzy.

**Łańcuch:** `osady + populacja + zasoby + drogi → frakcje i terytoria → prawa/relacje → podatki, dostęp, handel, konflikty i wydarzenia`.

## Co udostępnia pozostałym systemom

- Prawa, podatki, cła i reguły legalności towarów.
- Reputację, dostęp do dzielnic/usług i warunki zleceń.
- Cele, konflikty, sojusze i wydarzenia regionalne.
- Kontekst polityczny dla informacji przekazywanych przez NPC.

## Wpływ na postać i świat

- Początkowo decyzje gracza wpływają na reputację lokalnej osady, ceny i dostępność zleceń.
- Później reputacja, majątek i wykonane zadania mogą otworzyć udział w prawach, urzędach i dyplomacji.
- Polityka może wpływać na stawki, rekrutację, dostęp do dróg i bezpieczeństwo.
- Każda zmiana prawa lub granicy ma widoczny skutek i jest zapisywana w historii sesji.

## Kolejność wdrażania

1. Zacząć od lokalnej frakcji przypisanej do jednej osady; bez globalnych granic i dyplomacji.
2. Dodać prosty profil frakcji oraz reputację postaci.
3. Dodać 1–2 prawa, które modyfikują cenę, opłatę lub dostęp do zlecenia.
4. Połączyć osady w małe terytoria przez istniejące drogi i dodać relacje między frakcjami.
5. Wprowadzić wydarzenia, cła i konflikty z czytelnymi konsekwencjami.
6. Dopiero później rozwijać ekspansję państw, zmiany ustroju, dyplomację i wojny.

## Minimalny zakres

Jedna osada ma lokalną frakcję i reputację gracza; co najmniej jedna decyzja lub prawo wpływa na konkretną cenę, usługę albo dostępne zlecenie.

## Stan w kodzie (podgląd)

- Generowane państwa (`Realm`) mają terytorium, nazwę i profil: władca, ustrój, kultura, religia, ambicja władcy, sława, liczebność wojsk, siła wojskowa (z sławy i liczebności), cele (tylko odczyt) oraz relacje historyczne z sąsiadami.
- Statyczny handel międzypaństwowy: eksport/import z lokalnej podaży (biom, dzielnice, Sell budynków) i popytu (populacja, Buy budynków); szlaki z partnerami przy sprzyjających relacjach. Bez mutacji zapasów w sesji.
- Klik w nazwę państwa na mapie (przy włączonych granicach) pokazuje dossier w lewym panelu.
- Dyplomacja v0 (warstwa sesji): napięcie między parami sąsiadów ewoluuje co miesiąc gry; dossier pokazuje aktualny rodzaj relacji i napięcie.
- Wojna abstrakcyjna v0 (etap B): przy wysokim napięciu para wchodzi w `AtWar` (casus ≠ tylko podbój — też wiara, handel, rywalizacja). Blokada szlaków A↔B; osoby trzecie: embargo **albo** wypowiedzenie wojny już walczącym sąsiadom (okazja). Okupacja komórek tylko gdy strona ma ambicję `ExpandBorders` — konflikt może skończyć się bez zajęcia ziemi i bez „pełnego podboju”. Na mapie: kreskowanie napięcia i cross-hatch okupacji.
- Ugoda terytorialna: aneksja okupowanych komórek → `owners` + polyline; **E1:** osady na anektowanej ziemi (centroid) przechodzą do zwycięzcy; stolica przegranego korygowana.
- **Prawa:** generacja + dossier; `GoodBan` filtruje eksport; klik osady pokazuje państwo + prawo. Spec: [prawa.md](./prawa.md).
- **Aneksja E1+E2 done;** opór/bunt/interwencja/secesja (E3–E5) — plan [aneksja-osad.md](./aneksja-osad.md).
- Brak jeszcze: reputacji gracza, karawan, E3–E5.

## Kryterium ukończenia

Gracz może zrozumieć, kto ustala lokalne reguły, jak jego działania zmieniają reputację i jaki wymierny skutek ma ta reputacja w osadzie.
