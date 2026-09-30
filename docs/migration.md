# Migracja aplikacji do Bevy — parytet 1:1

## Cel

Przenieść obecną aplikację desktopową Rust + `eframe`/`egui` do aplikacji uruchamianej przez Bevy, zachowując wszystkie aktualne funkcje i ich zachowanie. Interfejs pozostaje w Rust: egui działa przez `bevy_egui`; Bevy odpowiada za cykl życia aplikacji, render mapy i obsługę aktualizacji.

To jest migracja techniczna, nie rozwój nowych funkcji gry. Nie dodawać wojsk, karawan, granic miast ani państw, pogody, pór roku, nowych systemów ekonomii, ekranów, akcji gracza, danych ani reguł. Przyszłe pomysły opisane gdzie indziej pozostają poza zakresem tego zadania.

Dozwolona jest zmiana skórki i układu wizualnego zgodnie z kierunkiem średniowieczno-fantasy, o ile nie usuwa ani nie dodaje żadnej dostępnej operacji lub informacji. „1:1” dotyczy funkcji, danych, reguł i reakcji aplikacji, nie pikselowej kopii tymczasowego GUI.

## Założenia architektoniczne

- `src-tauri/src/world/` pozostaje jedynym źródłem prawdy dla generowanego terenu. Nie przenosić generatora na ECS ani nie zmieniać algorytmów generacji.
- Ten sam seed i te same `TerrainGenParams` muszą nadal dawać identyczny świat.
- Użyć Bevy z `bevy_egui`, dobranych w zgodnych wersjach. Przed dodaniem zależności agent wdrażający ma potwierdzić zgodność wersji, toolchainu i backendu okna na macOS. Nie aktualizować niezwiązanych zależności.
- Nie przepisywać logiki świata na Bevy ECS. ECS stosować tylko tam, gdzie jest potrzebny do integracji aplikacji i prezentacji. Zachować istniejące typy i API, o ile nie blokują migracji.
- Początkowo zachować istniejące składanie obrazu mapy. Przekazać jego wynik do tekstury Bevy; nie przepisywać renderera na nowy system warstw bez konieczności technicznej potwierdzonej testem.
- Zachować generowanie poza głównym wątkiem, identyfikatory żądań, ignorowanie nieaktualnych wyników oraz ponowne używanie osad/NPC przy zmianie samego LOD.
- Panele i sterowanie mogą zmienić wygląd, ale nadal korzystają z obecnego egui. Nie dodawać alternatywnego GUI ani osobnego frontendowego procesu.

## Zakres funkcjonalny do zachowania

### Generator i parametry świata

- Seed jako `u64`, domyślnie `6`; wpisywanie i walidacja, losowanie seeda, opis seeda i regeneracja.
- Sześć profili z `seed % 6`: Radial, Ellipse, SquareBump, Irregular, Archipelago i Continents.
- Wszystkie obecne suwaki `TerrainGenParams` oraz ich wartości i znaczenie.
- Determinizm terenu, rzek, osad i ekologii dla identycznego seeda i parametrów.
- Stan ładowania i blokowanie operacji, które są obecnie niedostępne podczas generowania.

### Mapa i poziomy szczegółowości

- LOD: mapa świata → region → obszar/chunk; kliknięcie poza osadą schodzi poziom niżej, przycisk powrotu wraca wyżej.
- Cache widoków LOD oraz zachowanie osad i NPC przy zmianie samego LOD.
- Poprawne mapowanie pozycji kursora na współrzędne świata przy skalowaniu mapy.
- Teren, biomy, cieniowanie wysokości, wygładzanie wybrzeża, siatka regionów i skala mapy.
- Drogi, mosty, markery/footprinty osad, dzielnice oraz style rysowania właściwe dla aktualnego LOD.
- Nakładka potencjału siedlisk regionu, nakładka ekologii obszaru, legenda biomów/osad/ekologii.
- Fog of war: aktywny zasięg widzenia, odkryte obszary pozostające widoczne i marker gracza.

### Wybór miejsca i informacje

- Kliknięcie osady pokazuje jej informacje bez zmiany LOD.
- Kliknięcie dzielnicy pokazuje jej informacje, jeśli obecnie jest dostępne.
- Kliknięcie obszaru pokazuje biom i bieżące podsumowanie gatunków.
- Lista miejsc widoczna w obecnych warunkach, jej pozycje i wybór celu.
- Zachować teksty, etykiety i zawartość informacji, chyba że zmiana samej prezentacji jest wymagana przez docelową skórkę. Nie zmieniać znaczenia ani zakresu danych.

### Gracz, ruch i czas

- Wybór punktu spawnu na lądzie, potwierdzenie i anulowanie spawnu.
- Ruch pieszy do klikniętego celu, fokus na graczu, ruch kierunkowy i wybór miejsca docelowego z listy.
- Istniejące reguły przechodniości terenu, prędkość poza drogą i na drodze, snap do grafu dróg oraz wyznaczanie trasy.
- Aktualizacja widocznego chunka po przekroczeniu granicy oraz zapamiętywanie odwiedzonych miejsc i odkrytych chunków.
- Zegar: start, pauza/wznowienie, aktualne mnożniki prędkości, pominięcie godziny oraz wpływ czasu gry na ruch.
- Obecne etykiety ruchu i statusów gracza.

### Stan aplikacji i diagnostyka

- FPS i procent użycia CPU, jeśli bieżąca platforma udostępnia obecny pomiar.
- Informacje o generowaniu, status mapy i zachowanie błędów formularza.
- Zachować działanie istniejących testów jednostkowych i deterministycznych.

## Etapy wdrożenia

Każdy etap kończy się przejściem bramki weryfikacyjnej. Jeśli bramka nie przejdzie, poprawić ten etap przed rozpoczęciem kolejnego. Nie łączyć migracji z refaktoryzacją niezwiązaną z parytetem.

### 0. Ustalenie bazowego zachowania

**Praca:** użyć obecnego kodu i README jako specyfikacji. Przygotować checklistę ręcznych scenariuszy dla funkcji wymienionych powyżej. Odnotować wynik dla seeda `6`, co najmniej po jednym seedzie dla pozostałych pięciu profili oraz niezmienionych i zmienionych parametrów.

**Bramka:** lista wszystkich obecnie dostępnych operacji i widocznych danych jest jawna. Nie dopisywać przyszłych funkcji z `docs/plans.md` do zakresu migracji.

### 1. Spike technologiczny Bevy + egui

**Pliki:** `src-tauri/Cargo.toml`, tymczasowy kod startowy lub izolowany binarny spike.

**Praca:** dobrać zgodne, jawnie przypięte wersje Bevy i `bevy_egui`. Uruchomić okno na macOS, wyświetlić prostą teksturę w scenie 2D i jeden egui panel, sprawdzić kliknięcie i resize. Ustalić minimalne wymagane crate features. Nie dodawać jeszcze funkcji gry.

**Bramka:** `cargo check` i uruchomienie aplikacji przechodzą; egui i mapowa tekstura mogą współistnieć w jednym oknie. Jeśli nie, rozwiązać problem wersji/backendu przed przenoszeniem logiki.

### 2. Odseparowanie stanu i przepływu pracy od starego launchera

**Pliki:** przede wszystkim `src-tauri/src/app.rs`; w razie potrzeby małe, uzasadnione moduły dla stanu/sesji. Bez zmiany `world/`.

**Praca:** wydzielić tylko te elementy, których Bevy nie może bezpośrednio użyć z `MapApp`: stan sesji, wyniki/żądania generowania, obsługę LOD, ruch i wejścia. Zachować semantykę istniejących funkcji; nie zmieniać przy tej okazji nazw UI, kolorów, algorytmów ani reguł ruchu.

**Bramka:** dotychczasowa aplikacja nadal się uruchamia, a `cargo test --lib` przechodzi. Testy/dowody deterministyczności przed i po ekstrakcji są zgodne.

### 3. Szkielet Bevy i przepływ generowania

**Pliki:** `src-tauri/src/main.rs`, `Cargo.toml`, obecny `app.rs` oraz tylko niezbędne nowe moduły aplikacji/pluginów.

**Praca:** uruchomić Bevy jako launcher. Zarejestrować stan aplikacji i system odbierający wyniki workera przez kanał. Zachować kolejność i warunki stosowania wyników, odrzucanie wyników o nieaktualnym `request_id`, cache LOD i ponowne używanie osad/NPC. Zachować brak blokowania głównego wątku.

**Bramka:** wygenerowanie świata nadal nie blokuje UI; stany ładowania i wyniki dla aktualnego żądania odpowiadają starej aplikacji.

### 4. Renderer mapy i współrzędne

**Pliki:** `src-tauri/src/render.rs`, `src-tauri/src/main.rs` oraz moduł prezentacji mapy, jeśli jest potrzebny.

**Praca:** zachować `compose_map_image` oraz istniejące rysowanie osad, dróg, ekologii, gracza i fog of war. Dodać adapter obrazu do tekstury Bevy. Wprowadzić kamerę/widok mapy 2D i odwzorowanie przestrzeń ekranu ↔ przestrzeń mapy. Nie zmieniać kolejności warstw ani mapowania pikseli.

**Bramka:** dla tych samych wejść wynik obrazu i jego warstwy są zgodne z wersją bazową; zmiana rozmiaru okna nie przesuwa kliknięć względem mapy.

### 5. Interakcje LOD, osad i ekologii

**Pliki:** logika interakcji przeniesiona z `app.rs`, `settlements_draw.rs`, `ecology_draw.rs` i moduł UI.

**Praca:** przenieść reguły kliknięć, powrót między LOD, wybór osad/dzielnic, podsumowania biomu/gatunków, listę miejsc, legendy i teksty statusowe. Zachować różnice renderowania świat/region/chunk. Nie dodawać zoomowania, filtrowania ani nowych widoków.

**Bramka:** ręczna checklista kliknięć przechodzi na każdym LOD; kliknięcie osady nie zmienia LOD, a pozostałe kliknięcia zachowują dotychczasowe skutki.

### 6. Gracz, ruch, fog of war i zegar

**Pliki:** `app.rs`, `game_loop.rs`, `game_time.rs`, `render.rs` i moduły mapy/UI, zależnie od podziału po etapach 2–5.

**Praca:** przenieść spawn, anulowanie/potwierdzenie, ruch, trasowanie drogami, prędkości, fokus gracza, zmianę chunka, listę odkryć, widoczność i zegar. Zachować obecne progi i reguły z kodu. Bevy `Time` może dostarczać `dt`, ale skale, pauza oraz semantyka `GameLoop` pozostają bez zmian.

**Bramka:** porównawcze scenariusze ruchu, celu drogowego, ruchu poza drogą, przekroczenia chunku, pauzy, każdej prędkości i pominięcia godziny zgadzają się z wersją bazową.

### 7. GUI, diagnostyka i finalne przełączenie

**Pliki:** `src-tauri/src/app.rs`, `main.rs`, `perf.rs`, `Cargo.toml`, `README.md`; pozostałe wyłącznie wtedy, gdy wynika to z migracji.

**Praca:** przenieść wszystkie obecne kontrolki i panele do egui zintegrowanego z Bevy. Można zastosować uzgodniony motyw fantasy, ale zachować pola, wartości, akcje i informacje. Zachować odczyty FPS/CPU oraz komunikaty błędów/ładowania. Po przejściu testów usunąć `eframe` jako launcher i nieużywany stary kod UI.

**Bramka końcowa:** jedynym uruchamianym programem jest aplikacja Bevy; wszystkie pozycje parytetu działają; brak zależności i kodu nieużywanych po migracji.

## Weryfikacja

Uruchamiać polecenia z `src-tauri/`.

1. Po zmianach logiki świata lub sesji: `cargo test --lib`.
2. Po każdej fazie integracyjnej: `cargo check`.
3. Na końcu: `cargo test --lib` oraz `cargo run`.
4. Sprawdzić deterministyczność: ten sam seed i `TerrainGenParams` dają ten sam grid i te same wygenerowane dane; seed `6` nadal jest profilem Ellipse, a wszystkie indeksy `seed % 6` zachowują swój profil.
5. Wykonać pełną checklistę ręczną. Automatyczne testy jednostkowe nie zastępują sprawdzenia wejścia myszy, mapowania pozycji, renderu tekstury i interakcji okna.
6. Sprawdzić logi: brak błędów, panik, nieużywanych importów/dead code powstałych w migracji i blokowania UI przez generowanie.

Nie dodawać testów nowych mechanik, bo nie należą do zakresu. Testy migracyjne mają dokumentować i zabezpieczać istniejące zachowanie.

## Główne ryzyka i ograniczenia

- **Niedopasowanie wersji Bevy/`bevy_egui`:** rozwiązać w spike'u, nie po przeniesieniu całego UI.
- **Wymogi `Send`/`Sync` zasobów Bevy:** kanały, uchwyty tekstur i stan aplikacji umieszczać w zasobach tylko zgodnie z ograniczeniami API; nie przenosić ciężkiego generowania do systemu renderującego.
- **Zmiana współrzędnych mapy:** osobno sprawdzić skalowanie, letterboxing, resize i kliknięcia na każdym LOD.
- **Niezamierzona zmiana zachowania przez harmonogram:** zachować warunek, że symulacja nie tyka w trakcie generowania; zachować dotychczasowy `dt`, pauzę i mnożniki czasu.
- **Zbyt szeroki rewrite renderera:** utrzymać składanie obrazu jako bazę parytetu. Optymalizacje lub podział warstw wymagają osobnego uzasadnienia i nie są częścią tej migracji.

## Zasady dla agenta wdrażającego

- Przed pierwszą zmianą porównać tę listę z aktualnym kodem i zgłosić rozbieżności; nie zgadywać brakujących zachowań.
- Pracować etapami i po każdym etapie uruchamiać wskazaną bramkę. Nie zaczynać kolejnego etapu po nieudanej weryfikacji.
- Nie modyfikować algorytmów w `world/`, balansu, etykiet, kolorów ani reguł rozgrywki jako „przy okazji”.
- Nie dodawać funkcji z planów przyszłej gry. Zgłoszone braki względem parytetu naprawiać; pomysły poza zakresem pozostawić bez implementacji.
- Aktualizować `README.md` tylko w zakresie uruchamiania i stosu technologicznego po zakończonej migracji.
- Nie tworzyć commita ani gałęzi bez osobnego polecenia.

## Istniejące punkty odniesienia

- [AGENTS.md](../AGENTS.md) — kontrakt repozytorium, deterministyczność i komendy weryfikacyjne.
- [README.md](../README.md) — aktualne zachowanie użytkowe i opis LOD.
- [docs/plans.md](plans.md) — przyszły kierunek gry; przyszłe funkcje są wyłączone z zakresu tej migracji.
- `src-tauri/src/app.rs` — aktualny przepływ UI, LOD, ruch i zegar.
- `src-tauri/src/render.rs`, `settlements_draw.rs`, `ecology_draw.rs` — obecne warstwy obrazu.
- `src-tauri/src/world/` — deterministyczny generator świata, którego migracja nie zmienia.
