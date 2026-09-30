# Plan postaci i zależności systemów świata

Plan rozwija założenia z zakładki „Postać” oraz łączy postać z biomami, drogami, osadami, państwami i NPC. To propozycja kolejności implementacji, nie opis funkcji już dostępnych w grze.

## Postać gracza

### Tożsamość i start

- Gracz może wybrać rasę, klasę/profesję początkową i krótką historię; wybory opisują perspektywę i możliwości postaci, ale nie powinny zastępować pętli „zaczynasz jako nikt”.
- Domyślny start: bez tytułu, majątku i ekwipunku, w wybranym bezpiecznym miejscu. Opcjonalny tryb startu może dać doświadczenie lub wyposażenie, lecz powinien być osobnym scenariuszem.
- Historia wybiera kilka konkretnych tagów (np. dawny zawód, dług, więź z osadą), które odblokowują dialog, zlecenie lub reakcję NPC. Nie powinna dawać ukrytych, nieczytelnych kar.
- Minimalny stan postaci: pozycja i odkrycia, rasa, profesja/talenty, ekwipunek, monety, potrzeby, rany/obrażenia, reputacje i historia.

### Przetrwanie i rozwój

- Zamiast abstrakcyjnej puli życia używać czytelnych obrażeń i stanów (np. krwawienie, złamanie, wyczerpanie, choroba); konkretne skutki muszą być widoczne przed ryzykowną decyzją.
- Potrzeby na start ograniczyć do głodu i zmęczenia; zimno, choroby i inne stany dodawać dopiero, gdy da się im przeciwdziałać.
- Rozwój przez działania i profesje: zbieractwo, handel, rzemiosło, przewóz, służba/ochrona. Umiejętność ma dawać mały, zrozumiały efekt, nie blokować podstawowej gry.
- Śmierć kończy lub zmienia linię postaci zgodnie ze scenariuszem; późniejszy system rodu może umożliwić kontynuację świata przez dziedzica.

### Wpływ na świat

- **Osobisty:** zdobywa wiedzę i odkrycia, zmienia relacje oraz reputację, bierze zlecenia i handluje.
- **Lokalny:** zużywa zasoby, pomaga mieszkańcom, kupuje własność, buduje lub naprawia infrastrukturę; skutki zapisywane jako delty sesji.
- **Regionalny:** wpływa na popyt, ceny, migracje, bezpieczeństwo dróg i wydarzenia przez handel, zlecenia i decyzje.
- **Polityczny:** przy wysokiej reputacji/majątku może zmieniać lokalne prawo lub obejmować urząd; wymaga już działających osad i frakcji.
- Każda zmiana powinna wskazywać sprawcę, miejsce, czas i skutek. Bazowy teren z seeda pozostaje niemutowalny; rozgrywka dopisuje warstwę zmian.

## Systemy świata

### Biomy

- Określają warunki podróży, dostępne zasoby, ryzyko środowiskowe i potencjalne źródła utrzymania.
- Wykorzystują już wygenerowany teren, wysokość, wilgotność, rzeki i ekologię. Biomy nie powinny bezpośrednio zawierać zmiennego stanu sesji.
- Zbieranie, pogoda i lokalne zmiany nakładają się jako dane sesji na biom bazowy.
- Zależności: **teren + wysokość + wilgotność + hydrologia → biom → zasoby/ryzyko → aktywności i gospodarka**.

### Drogi

- Są grafem podróży łączącym osady, dzielnice, przeprawy i miejsca warte odkrycia.
- Typ i stan drogi wpływają na czas/koszt podróży, handel, karawany i bezpieczeństwo.
- Gracz może początkowo korzystać z istniejących dróg; budowa, naprawy, blokady i opłaty są zmianami sesji.
- Zależności: **osady + przełęcze/przeprawy + teren/rzeki → sieć dróg → podróż + handel + transport + wydarzenia**.

### Miasta i osady

- Osada ma pozycję, typ, populację, potrzeby, zapasy oraz dostęp do biomów i tras.
- Typ i wielkość osady wyznaczają podstawowe usługi oraz dostępne role/zlecenia.
- Dzielnice uzasadniają miejsca, NPC, usługi i aktywności; nie każdy obiekt występuje w każdej osadzie.
- Etap pierwszy używa osad statycznych z generacji. Później zmieniają się zapasy i populacja; dopiero zaawansowana symulacja może zmieniać typ/osadnictwo.
- Zależności: **biom + woda + teren + zasoby + drogi → osada → dzielnice/usługi/rynek → aktywności gracza**.

### Państwa i frakcje

- Na początek frakcja może odpowiadać jednej osadzie lub małej grupie osad; unikamy przedwczesnego systemu granic globalnych.
- Frakcja ma cele, strukturę władzy, terytorium, prawa, podatki i relacje z innymi frakcjami.
- Prawa powinny wpływać na widoczne działania: ceny, opłaty, dostęp do dzielnic, rekrutację, bezpieczeństwo lub legalność towarów.
- Zmiana władzy/terytorium wymaga historii zdarzeń i musi być zapisana w stanie sesji.
- Zależności: **osady + populacja + drogi + zasoby → frakcje/terytoria → prawa i relacje → podatki, dostęp, konflikty i zadania**.

### NPC

- NPC są osadzeni w konkretnej osadzie/dzielnicy, mają rolę, potrzeby, relacje i zakres wiedzy.
- Pierwszy etap: generowane deterministycznie imię/rola oraz statyczny kontakt do informacji, handlu lub zlecenia.
- Kolejne etapy: reputacja/dialogi, harmonogram dnia, przemieszczanie po drogach, relacje i reakcje na wydarzenia.
- Nie symulować wszystkich mieszkańców szczegółowo: aktywna okolica może mieć NPC jednostkowych, reszta świata zaś agregaty populacji.
- Zależności: **osada + dzielnice + populacja → role i NPC → informacje/usługi/zlecenia/dialogi → relacje i skutki świata**.

## Mapa zależności

```text
Seed + parametry
  └─> teren / wysokość / wilgotność / rzeki
        └─> biomy + ekologia + zasoby + ryzyka
              └─> miejsca osad + populacja + dzielnice
                    ├─> NPC i usługi
                    ├─> sieć dróg <─ teren + przeprawy
                    │     └─> podróż, handel, karawany, bezpieczeństwo
                    └─> frakcje / terytoria / prawa
                          └─> podatki, dostęp, reputacja, konflikty

Postać + zapis sesji
  ├─> podróż i odkrycia ───────────────> informacja o świecie
  ├─> potrzeby + ekwipunek + obrażenia -> przeżycie
  ├─> NPC + usługi + rynek ────────────> handel i zlecenia
  └─> decyzje gracza ──────────────────> reputacja / gospodarka / delty
                                            └─> wydarzenia i historia świata
```

## Kolejność wdrażania

1. **Fundament świata:** ustabilizować deterministyczne próbki terenu, biomów, rzek, osad i dróg; dodać wersjonowany zapis sesji oddzielony od seeda.
2. **Postać i podróż:** ujednolicić stan postaci, spawn, poruszanie, odkrycia, czas i koszt podróży; zachować dotychczasową deterministyczność świata.
3. **Biomy jako rozgrywka:** mapować biom/ekologię na niewielką liczbę zasobów i ryzyk; dodać zbieranie, ekwipunek oraz podstawowe potrzeby.
4. **Osada, NPC i rynek:** udostępnić informatora, prosty handel i lokalne zlecenie; wykorzystać istniejące osady, dzielnice i NPC.
5. **Drogi i kariery:** podpiąć dostawy i przewóz do sieci dróg, rozwinąć profesje, koszty oraz reputację.
6. **Frakcje i reakcje świata:** wprowadzić lokalne prawa, podatki, relacje i wydarzenia; dopiero wtedy rozszerzać symulację regionów.
7. **Długoterminowy wpływ:** własność, budowa, delty, zmiany populacji i dziedziczenie; testować skutki w zapisie i historii.

## Reguły architektoniczne

- Bazowy świat jest funkcją `(seed, params)` i nie zmienia się po rozpoczęciu gry.
- Stan mutowalny (postać, zapasy, reputacja, NPC aktywni, wydarzenia, delty) należy do sesji i zapisu.
- Generowanie wszystkich bytów korzysta z deterministycznych strumieni seeda; losowość sesji ma oddzielne kanały i jest zapisywana.
- System nie może deklarować wpływu postaci, jeśli konsekwencja nie jest widoczna i zapisana.
- Dla każdej funkcji planować ścieżkę: **akcja gracza → zmieniony stan → widoczny feedback → zapis → odtworzenie po wczytaniu**.
