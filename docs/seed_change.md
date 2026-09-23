# Seed — status zmiany (String → u64)

**Status: zrobione.** Ten plik zostawiony jako znacznik historii.

## Co było planowane

Przyjmowanie seeda jak w Minecraft: jeśli tekst parsuje się jako liczba, użyj go bezpośrednio; w przeciwnym razie hash stringa.

## Co jest w kodzie teraz

- Seed to `u64` w `WorldConfig` i w UI.
- Pole tekstowe w egui trzyma draft; **Regenerate** wymaga nieujemnej liczby całkowitej (parse → `u64`).
- Nie ma już hashowania stringa seeda ani mostu Tauri/React.

Aktualny opis działania: [`seed.md`](./seed.md).
