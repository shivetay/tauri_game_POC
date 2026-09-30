# Generowanie mostów — diagnoza obecnego stanu

## Wymagane zachowanie

- Most może łączyć dwa brzegi rzeki.
- Dwa mosty nie mogą przecinać się nad rzeką.
- Kolejna droga może korzystać z istniejącego mostu, ale nie może tworzyć z nim kolizji.
- Nie każda droga musi przekraczać rzekę. Może kończyć się na brzegu, zamiast wymuszać most.
- Reguły powinny działać również w gęstych układach dróg wokół miast.

## Jak działa obecny generator

Drogi między osadami powstają w `build_roads`. Dla każdego połączenia generator próbuje znaleźć trasę lądową, a w niektórych przypadkach trasę z przeprawą przez rzekę. Most jest zapisany jako punkt `Bridge` umieszczony pomiędzy dwoma punktami na lądzie; para tych punktów wyznacza jego odcinek brzeg–brzeg.

Podczas generowania kolejne drogi rejestrują swoje mosty. Następnie generator buduje rejestr z mostów całej sieci i dwukrotnie próbuje ponownie przetworzyć polilinie tak, aby uwzględniały inne przeprawy. Szczegóły znajdują się w [`settlement.rs`](../src-tauri/src/world/settlement.rs).

## Wnioski z analizy

### 1. Rejestr może pominąć kolidujący most, ale pozostawić go na drodze

`BridgeRegistry::register` nie rejestruje odcinka, jeśli `resolve_span` uzna go za istniejący, pobliski lub przecinający się z innym mostem. Rejestr nie zwraca jednak informacji, że odcinek został odrzucony, i nie usuwa ani nie zmienia punktów mostu w polilinii drogi.

W rezultacie sama droga może nadal zawierać most, którego rejestr już nie reprezentuje. Kolejne sprawdzenia kolizji bazują na rejestrze, więc nie widzą takiego odcinka. To bezpośrednia droga do pozostawienia w wyniku przecinających się mostów.

### 2. „Naprawa” po generacji nie gwarantuje zastąpienia wadliwej drogi

W dwóch przebiegach korekty generator zastępuje trasę tylko wtedy, gdy nowa polilinia ma co najmniej dwa punkty. Gdy korekta nie może zbudować poprawnej trasy, stara polilinia zostaje bez zmian. Odrzucona korekta nie powoduje więc ani odrzucenia oryginalnej drogi, ani błędu walidacji całej mapy.

Ponadto rejestr użyty przy korekcie może już nie obejmować mostów pominiętych wcześniej przy rejestracji. Kolejny przebieg nie gwarantuje zatem, że wykryje i naprawi wszystkie kolizje.

### 3. Dopasowanie do istniejącego mostu jest zbyt szerokie

`resolve_span` dopasowuje most na podstawie odległości środków do **32 jednostek świata**. To kryterium nie sprawdza, czy kandydat przecina tę samą rzekę, trafia w te same brzegi ani czy jego geometria pasuje do istniejącej przeprawy. Kandydat może więc zostać skierowany do odległego lub niewłaściwego mostu.

Także test przecięcia ma wyjątek dla odcinków, których końce leżą blisko dowolnego końca drugiego odcinka. Nie wymaga, by bliskie końce były tym samym portalem brzegowym. Przy gęstych trasach miejskich taki wyjątek może uznać za dopuszczalne odcinki, które w rzeczywistości przecinają się nad wodą.

### 4. Końcowy etap może wytworzyć most bez pełnej polityki generowania

`finalize_bridge_polyline` potrafi wstawić most pomiędzy kolejne punkty lądowe, jeżeli odcinek przebiega nad samą rzeką i nie wykryje konfliktu w rejestrze. Ten etap nie stosuje ponownie progu dopuszczenia mostu zależnego od klasy drogi, używanego podczas wyszukiwania trasy. Oznacza to, że decyzja o przeprawie może zależeć od tego, który etap przetwarza dany odcinek.

### 5. Brzeg nie jest obsługiwany jako prawidłowy koniec trasy

Obecny model połączeń wyznacza drogi pomiędzy osadami, a nie niezależne odcinki zakończone na brzegu. Gdy trasa dochodzi do rzeki, ale nie może legalnie zbudować lub wykorzystać mostu, generator nie ma jawnego wariantu „zakończ drogę na brzegu”. Może zamiast tego nie znaleźć drogi albo odrzucić polilinię. Wymagane zachowanie wymaga traktowania brzegu jako dopuszczalnego końca drogi, a nie jako nieudanego przekroczenia.

### 6. Problem nasila się przy miastach

Miasta są łączone szlakami głównymi, a przy każdym mieście może zbiegać się kilka odcinków sieci. Większe zagęszczenie dróg oznacza więcej kandydatów na przeprawy, więcej prób współdzielenia mostu oraz większe prawdopodobieństwo, że luźne dopasowanie lub pominięcie w rejestrze ukryje kolizję. To wyjaśnia, dlaczego błąd jest szczególnie widoczny w miastach, choć mechanizm nie jest ograniczony wyłącznie do nich.

## Pokrycie testami

Test `seed_6_bridges_are_shore_to_shore` sprawdza poprawność przepraw i brak przecinających się mostów dla jednego skonfigurowanego świata. Uruchomienie tego testu zakończyło się powodzeniem. Nie rozstrzyga to jednak problemu zgłaszanego dla innych seedów lub gęstych układów miejskich. Testy nie weryfikują ogólnej niezmienniczej reguły dla każdego wygenerowanego mostu ani zachowania drogi kończącej się nad rzeką.

## Podsumowanie

Ochrona przed kolizjami jest realizowana przez pomocniczy rejestr, ale rejestr nie jest wiarygodnym źródłem prawdy o mostach faktycznie zapisanych w poliliniach. Most może pozostać na drodze po odrzuceniu go przez rejestr, a nieudana korekta nie usuwa pierwotnego przebiegu. Szerokie dopasowanie do istniejącej przeprawy i wyjątek oparty wyłącznie na bliskości końców dodatkowo osłabiają wykrywanie kolizji.

Osobnym brakiem jest brak legalnego wariantu trasy zakończonej na brzegu. W efekcie generator traktuje brak mostu jako przeszkodę w połączeniu osad, zamiast móc zakończyć drogę przed rzeką.
