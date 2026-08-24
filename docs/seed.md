# Seed w generatorze map

## Cel

Ten dokument opisuje:

- jak dziala seed,
- jak podobny mechanizm dziala np. w Minecraft,
- jak seed dziala obecnie w tym projekcie,
- jak mozna zmienic aplikacje tak, aby przyjmowala liczby zamiast tekstu,
- jaki kod nalezaloby zmienic.

Ten plik jest **opisem planowanej zmiany**.
Poza `docs/seed.md` nie zostaly wprowadzone zadne zmiany w kodzie.

## Co to jest seed

Seed to wartosc startowa dla generatora pseudolosowego.
Generator nie tworzy "prawdziwej losowosci", tylko liczy wynik deterministycznie.

Jesli:

- seed jest ten sam,
- algorytm jest ten sam,
- parametry sa te same,

to wynik tez bedzie ten sam.

Dlatego seed pozwala odtworzyc identyczny swiat wiele razy.

Mozna o nim myslec jak o numerze przepisu:

- seed `5` daje jeden konkretny swiat,
- seed `123456` daje inny konkretny swiat,
- seed `5` wpisany ponownie zawsze odtworzy dokladnie ten sam swiat.

Seed sam w sobie nie opisuje swiata po ludzku.
To tylko wartosc wejsciowa, z ktorej generator wyprowadza dalsze liczby.

## Jak seed dziala np. w Minecraft

Minecraft korzysta z tej samej ogolnej idei:

1. Swiat dostaje seed glowny.
2. Na podstawie tego seeda inicjalizowane sa wewnetrzne generatory.
3. Generator laczy seed z polozeniem w swiecie, zeby policzyc teren, biomy, jaskinie i struktury.
4. Ten sam seed w tej samej wersji generatora daje ten sam swiat.

Wazna obserwacja:

- seed nie "znaczy" nic po ludzku,
- seed jest tylko liczba startowa,
- cale zroznicowanie swiata wynika z algorytmu.

W wielu grach seed moze byc wpisany jako tekst.
Wtedy tekst jest najpierw zamieniany na liczbe przez hash, a dopiero potem trafia do generatora.

Przyklad:

- `"abc"` nie jest magicznym slowem,
- generator nie rozumie znaczenia tego tekstu,
- najpierw powstaje liczba z hasha,
- dopiero potem ta liczba steruje swiatem.

Dlatego w Minecraft slowo `island` nie musi dawac wyspy.
Liczy sie tylko liczba, ktora powstanie z tego tekstu.

W praktyce sa tam dwa poziomy:

- **seed glowny** - identyfikuje swiat,
- **pochodne seeda** - rozne systemy uzywaja tego samego seeda inaczej, np. dla biomow, struktur albo chunkow.

To bardzo podobny wzorzec do tego projektu.

## Jak dziala seed w tym projekcie teraz

Obecnie projekt przyjmuje seed jako tekst, np. `eons-world-1`.

To widac w kilku miejscach:

```ts
const [seed, setSeed] = useState("eons-world-1");
const [draftSeed, setDraftSeed] = useState("eons-world-1");
```

```ts
export function generateGlobal(seed: string) {
  return invoke<TerrainGrid>("generate_global", { seed });
}
```

```rust
pub struct WorldConfig {
    pub seed: String,
}
```

W Rust seed jest hashowany:

```rust
pub fn hash_seed(seed: &str) -> u32 {
  let mut state: u32 = 0;
  for byte in seed.bytes() {
    state = state.wrapping_mul(31).wrapping_add(byte as u32);
  }
  state
}
```

To oznacza, ze:

- frontend trzyma tekst,
- Tauri przekazuje tekst,
- backend uzywa tego tekstu do wyprowadzenia liczb,
- profil swiata i szumy wynikaja z hasha tego tekstu.

Obecny wybor profilu wyglada tak:

```rust
let h = hash_seed(&self.seed);
ShapeProfile::from_index((h % 5) as u32)
```

Czyli:

`hash(seed) % 5`

Dlatego slowo `archipelago` nie musi dawac archipelagu.
Znaczenie tekstu nie ma znaczenia, liczy sie tylko hash.

Dodatkowo projekt wyprowadza z tekstu kilka pochodnych:

```rust
let elev_seed = hash_seed(&config.seed);
let moist_seed = hash_seed(&format!("{}:moisture", config.seed));
let detail_seed = hash_seed(&format!("{}:detail", config.seed));
let coast_seed = hash_seed(&format!("{}:coast", config.seed));
```

Dzieki temu:

- wysokosc, wilgotnosc, detal i wybrzeze sa rozne,
- ale dla tego samego tekstu zawsze identyczne.

## Dlaczego seed tekstowy jest mniej wygodny

Seed tekstowy nie jest bledny, ale ma kilka wad:

- trudniej przewidziec efekt,
- jedna litera moze calkiem zmienic profil,
- uzytkownik i tak finalnie dostaje liczbe ukryta za hashem,
- dokumentacja jest mniej intuicyjna.

Przyklad:

- `archipelago` nie musi dawac archipelagu,
- bo znaczenie slowa nie ma znaczenia,
- liczy sie tylko hash tekstu.

## Proponowana zmiana

Chcemy, aby aplikacja przyjmowala **liczbowy seed** zamiast tekstowego.

Najprostszy wariant:

- w UI wpisujemy liczbe,
- po stronie TypeScript seed ma typ `number`,
- po stronie Rust seed ma typ `u64`,
- deterministyczne pochodne nadal sa liczone z tego samego seeda.

## Co nalezaloby zmienic

### 1. Frontend - stan aplikacji

Obecnie:

```ts
const [seed, setSeed] = useState("eons-world-1");
const [draftSeed, setDraftSeed] = useState("eons-world-1");
```

Proponowana zmiana:

```ts
const [seed, setSeed] = useState(5);
const [draftSeed, setDraftSeed] = useState("5");
const [seedError, setSeedError] = useState<string | null>(null);
```

Dlaczego tak:

- `seed` jest juz liczba uzywana przez generator,
- `draftSeed` moze zostac stringiem tylko dla wygody inputa,
- `seedError` pozwala pokazac prosty blad walidacji.

### 2. Frontend - input

Obecnie:

```tsx
<input
  value={seed}
  onChange={(e) => onSeedChange(e.target.value)}
  placeholder="eons-world-1"
/>
```

Proponowana zmiana:

```tsx
<input
  type="number"
  min="0"
  step="1"
  value={seed}
  onChange={(e) => onSeedChange(e.target.value)}
  placeholder="5"
/>
```

To nie zmienia jeszcze typu danych w aplikacji, ale prowadzi uzytkownika do wpisywania liczb.

### 3. Frontend - walidacja przed regenerate

Obecnie:

```ts
function applySeed() {
  setSeed(draftSeed);
  setSelectedRegion(null);
}
```

Proponowana zmiana:

```ts
function applySeed() {
  const parsedSeed = Number(draftSeed);
  if (!Number.isSafeInteger(parsedSeed) || parsedSeed < 0) {
    setSeedError("Seed musi byc nieujemna liczba calkowita.");
    return;
  }

  setSeed(parsedSeed);
  setSeedError(null);
  setSelectedRegion(null);
}
```

To jest minimalna walidacja:

- tylko liczba calkowita,
- bez liczb ujemnych,
- w bezpiecznym zakresie JavaScript.

### 4. Frontend - API Tauri

Obecnie:

```ts
export function generateGlobal(seed: string) {
  return invoke<TerrainGrid>("generate_global", { seed });
}

export function generateRegion(seed: string, rx: number, ry: number) {
  return invoke<TerrainGrid>("generate_region", { seed, rx, ry });
}
```

Proponowana zmiana:

```ts
export function generateGlobal(seed: number) {
  return invoke<TerrainGrid>("generate_global", { seed });
}

export function generateRegion(seed: number, rx: number, ry: number) {
  return invoke<TerrainGrid>("generate_region", { seed, rx, ry });
}
```

### 5. Hooki frontendowe

Obecnie:

```ts
export function useWorldMap(seed: string) {
```

```ts
export function useRegionMap(seed: string, region: RegionId | null) {
```

Proponowana zmiana:

```ts
export function useWorldMap(seed: number) {
```

```ts
export function useRegionMap(seed: number, region: RegionId | null) {
```

### 6. Komendy Tauri

Obecnie:

```rust
fn generate_global(state: State<'_, Mutex<WorldState>>, seed: String) -> TerrainGrid
```

```rust
fn generate_region(
    state: State<'_, Mutex<WorldState>>,
    seed: String,
    rx: u32,
    ry: u32,
) -> TerrainGrid
```

Proponowana zmiana:

```rust
fn generate_global(state: State<'_, Mutex<WorldState>>, seed: u64) -> TerrainGrid
```

```rust
fn generate_region(
    state: State<'_, Mutex<WorldState>>,
    seed: u64,
    rx: u32,
    ry: u32,
) -> TerrainGrid
```

### 7. Konfiguracja generatora

Obecnie:

```rust
pub struct WorldConfig {
    pub seed: String,
}
```

Proponowana zmiana:

```rust
pub struct WorldConfig {
    pub seed: u64,
}
```

Obecny domyslny seed:

```rust
seed: "eons-world-1".into(),
```

Proponowany domyslny seed:

```rust
seed: 5,
```

`5` jest wygodne, bo przy prostym mapowaniu `seed % 5` daje profil `Radial`.

### 8. Wybor profilu ksztaltu

Obecnie:

```rust
let h = hash_seed(&self.seed);
ShapeProfile::from_index((h % 5) as u32)
```

Proponowana zmiana:

```rust
ShapeProfile::from_index((self.seed % 5) as u32)
```

To ma jedna duza zalete:

- profil staje sie przewidywalny,
- nie trzeba zgadywac wyniku hasha tekstu.

Przyklad:

- `5` -> `Radial`
- `6` -> `Ellipse`
- `7` -> `SquareBump`
- `8` -> `Irregular`
- `9` -> `Archipelago`

### 9. Pochodne seeda w Rust

Obecnie wiele rzeczy jest liczonych z tekstu:

```rust
let elev_seed = hash_seed(&config.seed);
let moist_seed = hash_seed(&format!("{}:moisture", config.seed));
let detail_seed = hash_seed(&format!("{}:detail", config.seed));
let coast_seed = hash_seed(&format!("{}:coast", config.seed));
```

Proponowana zmiana:

```rust
pub fn hash_seed(seed: u64) -> u32 {
  hash_bytes(seed.to_le_bytes())
}

pub fn derive_seed(seed: u64, salt: &str) -> u32 {
  hash_bytes(seed.to_le_bytes().into_iter().chain(salt.bytes()))
}
```

oraz:

```rust
let elev_seed = hash_seed(config.seed);
let moist_seed = derive_seed(config.seed, "moisture");
let detail_seed = derive_seed(config.seed, "detail");
let coast_seed = derive_seed(config.seed, "coast");
```

To zachowuje determinizm, ale usuwa zaleznosc od tekstowego inputa.

### 10. Archipelago i inne pochodne

Obecnie:

```rust
let n = 3 + (hash_seed(&format!("{}:arch", config.seed)) % 4) as usize;
let h64 = hash_seed_u64(&format!("{}:island:{i}", config.seed));
```

Proponowana zmiana:

```rust
let n = 3 + (derive_seed(config.seed, "arch") % 4) as usize;
let h64 = derive_seed_u64(config.seed, &format!("island:{i}"));
```

## Co dokladnie byloby zmienione

Jesli wdrazac te zmiane, dotkniete bylyby te pliki:

- `src/App.tsx`
- `src/components/Control/SeedControl.tsx`
- `src/api/world.ts`
- `src/hooks/useWorldMap.ts`
- `src/hooks/useRegionMap.ts`
- `src-tauri/src/lib.rs`
- `src-tauri/src/world/config.rs`
- `src-tauri/src/world/prng.rs`
- `src-tauri/src/world/sampler.rs`
- `src-tauri/src/world/shape.rs`

Poza kodem warto pozniej zaktualizowac tez `README.md`, bo obecnie opisuje tekstowe seedy.

## Co zostalo zmienione teraz

Na ten moment zmieniony zostal tylko ten plik:

- `docs/seed.md`

Nie bylo zmian w logice aplikacji.
To jest tylko dokumentacja i propozycja implementacji.

## Konsekwencje planowanej zmiany

Jesli taka zmiana zostanie kiedys wdrozona, trzeba pamietac o skutkach:

- stare tekstowe seedy przestana byc kompatybilne,
- trzeba bedzie ustalic nowy domyslny seed,
- dokumentacja `README.md` tez bedzie wymagac aktualizacji,
- frontend powinien pilnowac bezpiecznego zakresu `number`.

## Podsumowanie

Seed w generatorze to deterministyczna wartosc startowa.
Minecraft dziala na tej samej zasadzie: ten sam seed i ten sam algorytm daja ten sam swiat.

W tym projekcie obecny seed jest tekstowy, ale mozna go uproscic do liczbowego.
Najczystsze rozwiazanie to przejscie na `number` w TypeScript i `u64` w Rust, a wszystkie pochodne losowosci wyprowadzac juz z liczby.

## Kod do zmiany (gotowy do wklejenia)

Ponizej jest tylko kod, ktory trzeba podmienic, zeby seed liczbowy dzialal.
Reszta plikow zostaje bez zmian.

### `src/App.tsx`

```tsx
function App() {
	const [seed, setSeed] = useState(5);
	const [draftSeed, setDraftSeed] = useState("5");
	const [selectedRegion, setSelectedRegion] = useState<RegionId | null>(null);
	const [seedError, setSeedError] = useState<string | null>(null);

	const { grid, loading, error } = useWorldMap(seed);
	const regionMap = useRegionMap(seed, selectedRegion);

	function applySeed() {
		const parsedSeed = Number(draftSeed);
		if (!Number.isSafeInteger(parsedSeed) || parsedSeed < 0) {
			setSeedError("Seed musi byc nieujemna liczba calkowita.");
			return;
		}

		setSeed(parsedSeed);
		setSeedError(null);
		setSelectedRegion(null);
	}

	return (
		<main className="map-screen">
			<div className="map-area">
				{selectedRegion ? (
					<RegionMapView
						region={selectedRegion}
						grid={regionMap.grid}
						onBack={() => setSelectedRegion(null)}
					/>
				) : (
					<GlobalMapView grid={grid} onRegionSelect={setSelectedRegion} />
				)}
			</div>

			<aside className="side-panel">
				<SeedControl
					seed={draftSeed}
					onSeedChange={setDraftSeed}
					onRegenerate={applySeed}
					loading={loading || regionMap.loading}
				/>
				{seedError && <p className="map-error">{seedError}</p>}
				{error && <p className="map-error">{error}</p>}
				{regionMap.error && <p className="map-error">{regionMap.error}</p>}
				{(loading || regionMap.loading) && (
					<p className="map-loading">Generowanie mapy…</p>
				)}
			</aside>
		</main>
	);
}
```

### `src/components/Control/SeedControl.tsx`

```tsx
<input
	type="number"
	min="0"
	step="1"
	value={seed}
	onChange={(e) => onSeedChange(e.target.value)}
	placeholder="5"
/>
```

### `src/api/world.ts`

```ts
export function generateGlobal(seed: number) {
	assertTauri();
	return invoke<TerrainGrid>("generate_global", { seed });
}

export function generateRegion(seed: number, rx: number, ry: number) {
	assertTauri();
	return invoke<TerrainGrid>("generate_region", { seed, rx, ry });
}
```

### `src/hooks/useWorldMap.ts`

```ts
export function useWorldMap(seed: number) {
```

### `src/hooks/useRegionMap.ts`

```ts
export function useRegionMap(seed: number, region: RegionId | null) {
```

### `src-tauri/src/lib.rs`

```rust
#[tauri::command]
fn generate_global(state: State<'_, Mutex<WorldState>>, seed: u64) -> TerrainGrid {
	let mut guard = state.lock().unwrap();
	guard.config.seed = seed;
	let grid = generate_global_grid(guard.config.clone());
	guard.global_cache = Some(grid.clone());
	grid
}

#[tauri::command]
fn generate_region(
	state: State<'_, Mutex<WorldState>>,
	seed: u64,
	rx: u32,
	ry: u32,
) -> TerrainGrid {
	let guard = state.lock().unwrap();
	let mut config = guard.config.clone();
	config.seed = seed;
	generate_region_grid(config, RegionId { rx, ry })
}
```

### `src-tauri/src/world/config.rs`

Usun import:

```rust
use crate::world::prng::hash_seed;
```

Podmien pole i domyslna wartosc:

```rust
pub struct WorldConfig {
    pub seed: u64,
    // ... reszta bez zmian
}

impl Default for WorldConfig {
  fn default() -> Self {
    Self {
      seed: 5,
      // ... reszta bez zmian
    }
  }
}
```

Podmien metody:

```rust
impl WorldConfig {
  pub fn with_seed(mut self, seed: u64) -> Self {
      self.seed = seed;
      self
  }

  pub fn resolved_shape_profile(&self) -> ShapeProfile {
      self.shape_profile.unwrap_or_else(|| {
          ShapeProfile::from_index((self.seed % 5) as u32)
      })
  }
}
```

### `src-tauri/src/world/prng.rs`

Caly plik:

```rust
fn hash_bytes(bytes: impl IntoIterator<Item = u8>) -> u32 {
  let mut state: u32 = 0;
  for byte in bytes {
    state = state.wrapping_mul(31).wrapping_add(byte as u32);
  }
  state
}

pub fn hash_seed(seed: u64) -> u32 {
  hash_bytes(seed.to_le_bytes())
}

pub fn derive_seed(seed: u64, salt: &str) -> u32 {
  hash_bytes(seed.to_le_bytes().into_iter().chain(salt.bytes()))
}

pub fn derive_seed_u64(seed: u64, salt: &str) -> u64 {
  let a = derive_seed(seed, salt) as u64;
  let b = derive_seed(seed, &format!("{salt}:b")) as u64;
  (a << 32) | b
}
```

### `src-tauri/src/world/sampler.rs`

```rust
use crate::world::prng::{derive_seed, hash_seed};

impl TerrainSampler {
    pub fn new(config: WorldConfig) -> Self {
        let elev_seed = hash_seed(config.seed);
        let moist_seed = derive_seed(config.seed, "moisture");
        let detail_seed = derive_seed(config.seed, "detail");
        let coast_seed = derive_seed(config.seed, "coast");

        Self {
            config,
            elev_noise: SuperSimplex::new(elev_seed),
            moist_noise: SuperSimplex::new(moist_seed),
            detail_noise: SuperSimplex::new(detail_seed),
            coast_noise: SuperSimplex::new(coast_seed),
        }
    }
}
```

### `src-tauri/src/world/shape.rs`

```rust
use crate::world::prng::{derive_seed, derive_seed_u64, hash_seed};
```

W `Ellipse`:

```rust
let seed_h = hash_seed(config.seed) as f64;
```

W `Archipelago`:

```rust
let n = 3 + (derive_seed(config.seed, "arch") % 4) as usize;
let mut max_shape = 0.0_f64;
for i in 0..n {
    let h64 = derive_seed_u64(config.seed, &format!("island:{i}"));
    let ix = (h64 % config.world_width as u64) as f64;
    let iy = ((h64 >> 32) % config.world_height as u64) as f64;
    let dx = (world_x - ix) / (config.region_size as f64 * 0.8);
    let dy = (world_y - iy) / (config.region_size as f64 * 0.8);
    let d = (dx * dx + dy * dy).sqrt();
    max_shape = max_shape.max((1.0 - d).clamp(0.0, 1.0));
}
max_shape
```
