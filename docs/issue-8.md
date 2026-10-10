# Issue #8 — cykle dnia nocy

Źródło: https://github.com/shivetay/tauri_game_POC/issues/8

## Stan po tej zmianie (etapy 1–3)

### Etap 1 — wskaźnik cyklu dnia/nocy (wcześniej)
- Pod zegarem: oś wschód–południe–zachód ze słońcem (06–18) i księżycem (18–06).
- Pozycja markera z `second_of_day`; mapa bez tintu.

### Etap 2 — kalendarz (ta zmiana)
- Rok gry: **12 miesięcy × 30 dni = 360 dni**.
- `GameTime::day` pozostaje bezwzględnym indeksem dnia (1-based).
- Z dnia wyliczane: `year`, `month`, `day_of_month`, `day_of_year`.
- Etykieta zegara: `R{rok} · DD.MM · {pora} · HH:MM:SS`.

### Etap 3 — pory roku (ta zmiana)
- Cztery pory: Wiosna, Lato, Jesień, Zima — po 3 miesiące (90 dni).
- Rok zaczyna się wiosną (miesiące 1–3).
- Zmiany: dzień 1 / 91 / 181 / 271 roku (oraz powrót do wiosny na dniu 361 = R2 · 01.01).

### Etap 4 — poza zakresem tej zmiany
- Wygląd mapy zależny od pory roku **nie** jest wdrożony.

## Pliki
- `src-tauri/src/game_time.rs` — kalendarz, `Season`, testy granic miesięcy/lat/sezonów
- Panel zegara w `app.rs` — bez osobnej zmiany; korzysta z `format_label`

## Weryfikacja
- `cargo test --lib` w `src-tauri` (m.in. `calendar_*`, `season_change_moments`, `format_start_morning`)
