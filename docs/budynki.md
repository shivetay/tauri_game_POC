# Budynki — lista według dzielnic i plan wdrożenia

## Status

To propozycja zawartości gry, nie lista obiektów już generowanych przez kod. Obecny generator osad tworzy ich typ, położenie, drogi i — zależnie od osady — dzielnice; nie generuje jeszcze budynków. Małe osady mogą mieć pustą listę dzielnic.

## Budynki w dzielnicach

| Dzielnica | Proponowane budynki | Uwarunkowania |
| --- | --- | --- |
| Centrum / rynek (`Center`) | Ratusz lub dom starszyzny; gospoda; studnia publiczna; tablica ogłoszeń i waga miejska | Rdzeń usług osady. W małych osadach rolę ratusza może pełnić wspólna izba. |
| Handlowa (`Market`) | Kramy; hala targowa; skład kupiecki; kantor lub dom kupca; waga targowa | Liczba kramów rośnie wraz z wielkością osady i ruchem na drogach. |
| Rzemieślnicza (`Craft`) | Kuźnia; warsztat ciesielski; garncarnia z piecem; garbarnia lub farbiarnia; wspólny warsztat | Dobór warsztatów zależy od dostępnych surowców i ograniczeń, np. dymu lub dostępu do wody. |
| Portowa (`Port`) | Nabrzeże i przystań; szopa łodziowa; targ rybny; warsztat szkutniczy i powroźniczy; komora celna | Tylko przy żeglownej rzece lub odpowiednim wybrzeżu; zakres zależy od skali transportu wodnego. |
| Świątynna (`Temple`) | Świątynia lub kaplica; dom duchownych; przytułek; dom pielgrzyma; cmentarz | Wielkość i liczba obiektów zależą od wielkości osady i lokalnych zwyczajów. |
| Zamożna (`Noble`) | Dwór lub rezydencja; ogród reprezentacyjny; stajnia; dom straży; sala przyjęć | Występuje w większych osadach lub siedzibach lokalnej władzy. |
| Wojskowa (`Military`) | Koszary; zbrojownia; plac ćwiczeń; wieża strażnicza; magazyn zaopatrzenia | Zależy od znaczenia obronnego osady; mur lub palisada nie wymaga istnienia tej dzielnicy. |
| Mieszkaniowa (`Residential`) | Domy rodzinne; kamienice lub długie domy; studnia lub pompa; łaźnia publiczna | Typ zabudowy i gęstość zależą od populacji oraz dostępu do wody. |
| Leśna / park (`Forest`) | Leśniczówka; pawilon parkowy; szkółka drzew; zielnik; posterunek strażnika | Leśniczówka i szkółka wymagają dostępu do lasu; parkowe obiekty mogą zastąpić je w mieście. |
| Obrzeża (`Outskirts`) | Zagrody i stodoły; młyn; piec węglarski; kamieniołom lub obóz wydobywczy; szopa podróżna | Gospodarka zależy od biomu, zasobów, wody i połączeń drogowych; obiekty wydobywcze są warunkowe. |

Nazwy i zestawy są propozycjami. Nie każda osada ani każda dzielnica musi zawierać wszystkie wymienione budynki.

## Osada bez dzielnic

Brak dzielnic nie może oznaczać braku zabudowy. Budynki należy wtedy rozmieszczać bezpośrednio w zwykłym obszarze osady — bez przypisywania ich do nieistniejącej dzielnicy i bez tworzenia dzielnic wyłącznie po to, by umieścić budynki.

Minimalny zestaw dla takiej osady:

- skupisko domów lub pojedyncze zagrody, dobrane do populacji i typu osady;
- wspólna studnia albo inne lokalne ujęcie wody, jeśli pozwala na to geografia;
- wspólna izba lub miejsce spotkań;
- skład lub stodoła na lokalne zapasy;
- co najmniej jeden warsztat, kram albo gospoda jako punkt pracy lub wymiany.

W najmniejszych osadach funkcje mogą łączyć się w jednym obiekcie, np. izba starszyzny może być jednocześnie gospodą i miejscem ogłoszeń. Liczba budynków powinna rosnąć wraz z populacją, a ich dobór pozostać możliwy także bez dzielnic.

## Zależności

`typ i populacja osady + teren/biom + woda/zasoby + drogi → możliwe budynki → usługi, praca i handel dla NPC oraz gracza`.

- Jeśli osada ma dzielnice, dzielnica wskazuje pulę budynków; warunki lokalne filtrują tę pulę.
- Jeśli osada nie ma dzielnic, ten sam dobór korzysta z puli podstawowej i lokalnych warunków, a budynki trafiają do wspólnego obszaru osady.
- Wielkość osady ogranicza liczbę i skalę budynków; mała populacja może łączyć kilka funkcji w jednym budynku.
- Biom i zasoby uzasadniają produkcję, np. warsztat drewna przy lesie lub obiekt wydobywczy przy złożu.
- Drogi, rzeka i wybrzeże wpływają na dostępność targu, składu, przystani i usług podróżnych.
- Budynki powinny udostępniać role NPC, towary, usługi i aktywności; skutki trwałych zmian należą do zapisywanej warstwy sesji.

## Kolejność wdrażania

1. Zdefiniować dane budynku i pule typów, oddzielając budynki podstawowe od dzielnicowych.
2. Rozmieszczać podstawową zabudowę w każdej osadzie, także przy pustej liście dzielnic.
3. Dla obecnych dzielnic dobierać budynki z ich puli, uwzględniając populację i lokalne warunki.
4. Powiązać budynki z usługami, rolami NPC, handlem i pracą.
5. Zapisywać budowę, zniszczenie i ulepszenia jako zmiany sesji, bez naruszania deterministycznego bazowego układu świata.

## Kryterium ukończenia

Każda osada ma widoczną, sensowną zabudowę niezależnie od tego, czy wygenerowano w niej dzielnice. Dzielnice różnicują funkcje i budynki, a teren, populacja i dostęp do zasobów ograniczają ich dobór.
