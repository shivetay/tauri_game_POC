# Seed — obsługa wartości liczbowych (Minecraft-style)

## Problem

Aktualnie seed jest zawsze traktowany jako `String` i hashowany do `u64` przez `hash_seed_u64`.
Wpisanie `128765487` jako tekstu daje inny seed niż gdyby użytkownik oczekiwał wartości liczbowej.

## Rozwiązanie

Wzorujemy się na podejściu Minecrafta:

> Jeśli wpisana wartość parsuje się jako `i64`, jest używana bezpośrednio (bez hashowania).
> Jeśli nie — string jest hashowany jak dotychczas.

Dzięki temu `"128765487"` i `128765487` dają **ten sam seed**.

## Zmiana w `prng.rs`

Dodana funkcja `parse_seed`:

```rust
/// Minecraft-style: if the string is a valid integer, use it directly.
/// Otherwise hash the string.
pub fn parse_seed(seed: &str) -> u64 {
    if let Ok(n) = seed.trim().parse::<i64>() {
        n as u64
    } else {
        hash_seed_u64(seed)
    }
}
```

## Kolejne kroki

- [ ] Zastąpić bezpośrednie wywołania `hash_seed(&config.seed)` przez `parse_seed` w:
  - `sampler.rs` — `elev_seed`, `moist_seed`, `detail_seed`, `coast_seed`
  - `shape.rs` — `seed_h`, `arch`, `island:{i}`
  - `config.rs` — `resolved_shape_profile`
- [ ] Upewnić się, że istniejące seedy tekstowe (np. `"eons-world-1"`) dają te same wyniki co przed zmianą (backwards compatible — `parse_seed` dla nieparsowalnych stringów wywołuje `hash_seed_u64`, więc jest OK).
