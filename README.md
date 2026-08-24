# map_tests

Podgląd proceduralnej mapy świata (Tauri + React + Rust). Teren jest deterministyczny: **ten sam seed zawsze daje ten sam świat**.

```bash
pnpm install
pnpm tauri dev
```

Wpisz seed w polu **Seed** i kliknij **Regenerate**. Klik na mapie otwiera region.

## Jak dostać konkretny kształt świata

Kształt lądu **nie zależy od znaczenia słowa** w seedzie. Generator liczy hash stringa i wybiera jeden z pięciu profili:

`hash(seed) % 5`

Dlatego `wyspa` to archipelag, a `archipelago` (ang.) to jedna okrągła wyspa. Poniższe seedy są sprawdzone pod dany profil.

| Chcesz | Seed | Profil |
| --- | --- | --- |
| Pojedyncza okrągła wyspa | `eons-world-1`, `island`, `pangea` | Radial |
| Wydłużona wyspa | `eons-world-2` (szeroka), `madagascar` (wyższa) | Ellipse |
| Ląd przy krawędziach, woda w centrum | `eons-world-3`, `earth`, `staly-lad` | SquareBump |
| Jedna wyspa z zatokami i półwyspami | `eons-world-4`, `iceland`, `coast` | Irregular |
| Dużo wysp (archipelag) | `archipelag`, `duzo-wysp` (~5 wysp), `mapa-1` (~6 wysp) | Archipelago |

Domyślny seed aplikacji to `eons-world-1` (okrągła wyspa).

## Co jeszcze zmienia seed

Poza profilem ten sam string steruje szumem wysokości, wilgotnością (osobny hash `seed:moisture`) i detalami wybrzeża.

- **Ellipse** — proporcje wyspy (szersza vs wyższa) też wychodzą z seeda.
- **Archipelago** — liczba wysp to 3–6, pozycje centrów są losowane z seeda (deterministycznie).
- **Irregular** — linia brzegowa jest „rozejechana” szumem, więc zatoki różnią się między seedami tego samego profilu.

Zmiana jednej litery może zmienić zarówno rzeźbę terenu, jak i cały profil kształtu.

## Profile (krótko)

- **Radial** — okrągła wyspa na środku mapy, ocean dookoła.
- **Ellipse** — jedna wyspa, spłaszczona w poziomie lub pionie.
- **SquareBump** — ląd przy brzegach kwadratu, otwarte morze w centrum.
- **Irregular** — jedna wyspa, ale z zatokami, cyplami i nierównym wybrzeżem.
- **Archipelago** — kilka osobnych wysp (unia małych falloffów).
