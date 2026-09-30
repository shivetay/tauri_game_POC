# Dzielnice — aktualna lista i zależności

## Aktualne typy w kodzie

Poniższe etykiety są obecnie używane w legendzie i panelu osady. Generator dobiera dzielnice zależnie od osady i jej warunków; nie każda osada ma wszystkie typy.

| Typ w kodzie | Nazwa widoczna w UI | Proponowana rola |
| --- | --- | --- |
| `Center` | Centrum / rynek | Centralny punkt osady, ulice i wspólne usługi. |
| `Market` | Handlowa | Sprzedaż, zakupy i wymiana towarów. |
| `Craft` | Rzemieślnicza | Warsztaty, przetwarzanie surowców i praca rzemieślnicza. |
| `Port` | Portowa | Przystań, transport i handel wodny; tylko przy odpowiedniej geografii. |
| `Temple` | Świątynna | Miejsca kultu i aktywności religijne. |
| `Noble` | Zamożna | Rezydencje i usługi dla zamożnych mieszkańców. |
| `Military` | Wojskowa | Garnizon, szkolenie i obrona osady. |
| `Residential` | Mieszkaniowa | Domy i codzienne życie mieszkańców. |
| `Forest` | Leśna / park | Zieleń, las lub park; obecność zależy od układu i otoczenia. |
| `Outskirts` | Obrzeża | Zabudowa peryferyjna, gospodarstwa i możliwe miejsce rozbudowy. |

## Zależności

`teren + biom + woda + wielkość osady → układ i dostępne dzielnice → miejsca/NPC/usługi → aktywności gracza`.

- Typ i populacja osady ograniczają liczbę oraz dobór dzielnic.
- Biom, wybrzeże/rzeka i zasoby powinny uzasadniać wyspecjalizowane dzielnice, np. portową.
- Dzielnica określa, jakie role NPC, usługi i zlecenia można tam znaleźć.
- Drogi i ulice łączą dzielnice z bramami, centrum, rynkiem oraz trasami poza osadę.
- Frakcja i lokalne prawa mogą modyfikować dostęp lub opłaty, ale nie powinny zmieniać bazowego układu wygenerowanego z seeda.

## Wpływ postaci

- Postać odkrywa dzielnice i pozyskuje w nich informacje, usługi, pracę lub towary.
- Reputacja może otwierać lub ograniczać dostęp do wybranych dzielnic i NPC.
- Później gracz może kupować działki, budować lub naprawiać obiekty; zmiany są zapisywanymi deltami sesji.
- Nieobecność typu w konkretnej osadzie jest prawidłowa, jeśli nie uzasadniają go warunki lub wielkość.

## Kolejność wdrażania

1. Utrzymać aktualne typy/etykiety i dobór zależny od osady.
2. Nadać usługę każdej obecnej dzielnicy przez przypisane role NPC.
3. Podłączyć usługi i zlecenia do rynku, zasobów biomu i sieci dróg.
4. Dodać reputację i prawa frakcji jako warunki dostępu.
5. Dodać własność i zabudowę jako delty sesji, bez modyfikowania bazowego layoutu.

## Kryterium ukończenia

Gracz może rozpoznać dzielnicę, skorzystać z właściwej dla niej usługi lub spotkać jej role NPC, a dostępność dzielnicy ma sens względem typu osady i geografii.
