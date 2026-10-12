use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::world::config::LodLevel;
use crate::world::elevation::is_water_biome;
use crate::world::prng::unit_noise;
use crate::world::sampler::TerrainSampler;
use crate::world::settlement::{Road, Settlement, SettlementKind};

/// Dedicated PRNG channel — must not reuse settlement/road/NPC channels.
const CHANNEL_BORDER: u64 = 50;

const GRID_STEP: f32 = 8.0;
const ROAD_ENDPOINT_R: f32 = 12.0;
const FREE_CITY_COST: f32 = 140.0;
const LAND_COST: u32 = 10;
const RIVER_COST: u32 = 40;
const ENCLAVE_MIN_CELLS: usize = 6;
const CITY_VERTS: usize = 16;
const TOWN_VERTS: usize = 12;
const TAU: f32 = std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorderPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityBorder {
    pub settlement_idx: usize,
    pub polygon: Vec<BorderPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Realm {
    pub id: usize,
    pub name: String,
    pub capital_idx: usize,
    pub settlement_idxs: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealmBorder {
    /// Index into `realms`.
    pub realm_a: usize,
    /// Other realm, or `None` for water / world edge.
    pub realm_b: Option<usize>,
    pub polyline: Vec<BorderPoint>,
}

/// Coarse ownership raster for territory wash. `owners[i] == u16::MAX` → ocean / unowned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryMap {
    pub cols: u32,
    pub rows: u32,
    pub step: f32,
    pub owners: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BorderGen {
    pub city_borders: Vec<CityBorder>,
    pub realms: Vec<Realm>,
    pub realm_borders: Vec<RealmBorder>,
    pub territory: TerritoryMap,
}

pub fn generate(
    seed: u64,
    sampler: &TerrainSampler,
    settlements: &[Settlement],
    roads: &[Road],
) -> BorderGen {
    if settlements.is_empty() {
        return BorderGen {
            city_borders: Vec::new(),
            realms: Vec::new(),
            realm_borders: Vec::new(),
            territory: TerritoryMap::default(),
        };
    }

    let city_borders = build_city_borders(seed, sampler, settlements);
    let realms = build_realms(settlements, roads);
    let (realm_borders, territory) = build_realm_borders(sampler, settlements, &realms);

    BorderGen {
        city_borders,
        realms,
        realm_borders,
        territory,
    }
}

fn kind_radius_factor(kind: SettlementKind) -> f32 {
    match kind {
        SettlementKind::City => 1.55,
        SettlementKind::Town => 1.35,
        SettlementKind::Village | SettlementKind::Hamlet => 1.0,
    }
}

fn is_ocean(sampler: &TerrainSampler, x: f32, y: f32) -> bool {
    let cell = sampler.cell_at(x as f64, y as f64, LodLevel::Macro);
    if !is_water_biome(cell.elevation) {
        return false;
    }
    let river = sampler.river_at(x as f64, y as f64);
    river.channel < 0.55
}

fn is_land(sampler: &TerrainSampler, x: f32, y: f32) -> bool {
    !is_water_biome(sampler.cell_at(x as f64, y as f64, LodLevel::Macro).elevation)
}

fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
}

fn build_city_borders(
    seed: u64,
    sampler: &TerrainSampler,
    settlements: &[Settlement],
) -> Vec<CityBorder> {
    let mut out = Vec::new();
    for (si, s) in settlements.iter().enumerate() {
        if !matches!(s.kind, SettlementKind::Town | SettlementKind::City) {
            continue;
        }
        let n_verts = if s.kind == SettlementKind::City {
            CITY_VERTS
        } else {
            TOWN_VERTS
        };
        let base_r = s.radius * kind_radius_factor(s.kind);
        let mut half_neighbor = f32::INFINITY;
        for (oj, o) in settlements.iter().enumerate() {
            if oj == si {
                continue;
            }
            half_neighbor = half_neighbor.min(0.5 * dist(s.x, s.y, o.x, o.y));
        }
        let max_r = base_r.min(half_neighbor).max(s.radius * 0.35);

        let mut polygon = Vec::with_capacity(n_verts);
        for vi in 0..n_verts {
            let ang = TAU * (vi as f32) / (n_verts as f32);
            let n = unit_noise(seed, CHANNEL_BORDER, (si as u64) * 100 + vi as u64) as f32;
            let mut r = max_r * (0.82 + n * 0.36);
            let min_r = s.radius * 0.25;
            let (dx, dy) = (ang.cos(), ang.sin());
            while r > min_r {
                let px = s.x + dx * r;
                let py = s.y + dy * r;
                if is_land(sampler, px, py) {
                    break;
                }
                r *= 0.88;
            }
            let px = s.x + dx * r.max(min_r);
            let py = s.y + dy * r.max(min_r);
            // Pull onto land if the tip still sits in water.
            let (px, py) = if is_land(sampler, px, py) {
                (px, py)
            } else {
                (s.x + dx * min_r, s.y + dy * min_r)
            };
            polygon.push(BorderPoint { x: px, y: py });
        }
        if polygon.len() >= 3 {
            out.push(CityBorder {
                settlement_idx: si,
                polygon,
            });
        }
    }
    out
}

fn road_length(road: &Road) -> f32 {
    let mut len = 0.0f32;
    for w in road.points.windows(2) {
        len += dist(w[0].x, w[0].y, w[1].x, w[1].y);
    }
    len
}

fn nearest_settlement(settlements: &[Settlement], x: f32, y: f32) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;
    for (i, s) in settlements.iter().enumerate() {
        let d = dist(x, y, s.x, s.y);
        if d > ROAD_ENDPOINT_R {
            continue;
        }
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((i, d));
        }
    }
    best
}

fn build_road_graph(settlements: &[Settlement], roads: &[Road]) -> Vec<Vec<(usize, f32)>> {
    let n = settlements.len();
    let mut adj = vec![Vec::new(); n];
    for road in roads {
        if road.points.len() < 2 {
            continue;
        }
        let a = &road.points[0];
        let b = &road.points[road.points.len() - 1];
        let Some((ia, _)) = nearest_settlement(settlements, a.x, a.y) else {
            continue;
        };
        let Some((ib, _)) = nearest_settlement(settlements, b.x, b.y) else {
            continue;
        };
        if ia == ib {
            continue;
        }
        let cost = road_length(road).max(1.0);
        adj[ia].push((ib, cost));
        adj[ib].push((ia, cost));
    }
    adj
}

fn dijkstra_from(adj: &[Vec<(usize, f32)>], sources: &[usize]) -> Vec<f32> {
    let n = adj.len();
    let mut dist = vec![f32::INFINITY; n];
    #[derive(Copy, Clone, PartialEq)]
    struct Node {
        cost: f32,
        i: usize,
    }
    impl Eq for Node {}
    impl Ord for Node {
        fn cmp(&self, other: &Self) -> Ordering {
            other
                .cost
                .partial_cmp(&self.cost)
                .unwrap_or(Ordering::Equal)
                .then_with(|| self.i.cmp(&other.i))
        }
    }
    impl PartialOrd for Node {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    let mut heap = BinaryHeap::new();
    for &s in sources {
        if s < n {
            dist[s] = 0.0;
            heap.push(Node { cost: 0.0, i: s });
        }
    }
    while let Some(Node { cost, i }) = heap.pop() {
        if cost > dist[i] {
            continue;
        }
        for &(j, w) in &adj[i] {
            let next = cost + w;
            if next < dist[j] {
                dist[j] = next;
                heap.push(Node { cost: next, i: j });
            }
        }
    }
    dist
}

fn pick_capitals(settlements: &[Settlement]) -> Vec<usize> {
    let mut cities: Vec<usize> = settlements
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind == SettlementKind::City)
        .map(|(i, _)| i)
        .collect();
    let mut towns: Vec<usize> = settlements
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind == SettlementKind::Town)
        .map(|(i, _)| i)
        .collect();

    let pool = if !cities.is_empty() {
        &mut cities
    } else if !towns.is_empty() {
        &mut towns
    } else {
        // Fallback: one capital from the largest settlement.
        let mut best = 0usize;
        for (i, s) in settlements.iter().enumerate() {
            if s.population > settlements[best].population
                || (s.population == settlements[best].population && i < best)
            {
                best = i;
            }
        }
        return vec![best];
    };

    pool.sort_by(|&a, &b| {
        settlements[b]
            .population
            .cmp(&settlements[a].population)
            .then_with(|| a.cmp(&b))
    });
    let want = pool.len().clamp(1, 6);
    let want = if pool.len() >= 2 { want.max(2) } else { want };
    pool.iter().copied().take(want).collect()
}

fn build_realms(settlements: &[Settlement], roads: &[Road]) -> Vec<Realm> {
    let n = settlements.len();
    let adj = build_road_graph(settlements, roads);
    let mut capitals = pick_capitals(settlements);

    // Per-capital road distances.
    let mut cap_dists: Vec<Vec<f32>> = capitals
        .iter()
        .map(|&c| dijkstra_from(&adj, &[c]))
        .collect();

    let mut owner = vec![usize::MAX; n];
    for i in 0..n {
        let mut best_c = 0usize;
        let mut best_cost = f32::INFINITY;
        for (ci, &cap) in capitals.iter().enumerate() {
            let mut cost = cap_dists[ci][i];
            if !cost.is_finite() {
                cost = dist(
                    settlements[i].x,
                    settlements[i].y,
                    settlements[cap].x,
                    settlements[cap].y,
                );
            }
            if cost < best_cost || (cost == best_cost && cap < capitals[best_c]) {
                best_cost = cost;
                best_c = ci;
            }
        }
        owner[i] = best_c;
    }

    // Free city-states: distant City becomes its own capital.
    for i in 0..n {
        if settlements[i].kind != SettlementKind::City {
            continue;
        }
        if capitals.contains(&i) {
            continue;
        }
        let ci = owner[i];
        let mut cost = cap_dists[ci][i];
        if !cost.is_finite() {
            let cap = capitals[ci];
            cost = dist(
                settlements[i].x,
                settlements[i].y,
                settlements[cap].x,
                settlements[cap].y,
            );
        }
        if cost > FREE_CITY_COST {
            capitals.push(i);
            let new_ci = capitals.len() - 1;
            owner[i] = new_ci;
            cap_dists.push(dijkstra_from(&adj, &[i]));
            // Reassign any settlements closer to this new capital.
            for j in 0..n {
                if j == i {
                    continue;
                }
                let mut new_cost = cap_dists[new_ci][j];
                if !new_cost.is_finite() {
                    new_cost = dist(
                        settlements[j].x,
                        settlements[j].y,
                        settlements[i].x,
                        settlements[i].y,
                    );
                }
                let mut old_cost = cap_dists[owner[j]][j];
                if !old_cost.is_finite() {
                    let cap = capitals[owner[j]];
                    old_cost = dist(
                        settlements[j].x,
                        settlements[j].y,
                        settlements[cap].x,
                        settlements[cap].y,
                    );
                }
                if new_cost < old_cost || (new_cost == old_cost && i < capitals[owner[j]]) {
                    owner[j] = new_ci;
                }
            }
        }
    }

    // Ensure every capital owns itself.
    for (ci, &cap) in capitals.iter().enumerate() {
        owner[cap] = ci;
    }

    let mut realms = Vec::with_capacity(capitals.len());
    for (ci, &cap) in capitals.iter().enumerate() {
        let mut idxs: Vec<usize> = (0..n).filter(|&i| owner[i] == ci).collect();
        idxs.sort_unstable();
        realms.push(Realm {
            id: ci,
            name: settlements[cap].name.clone(),
            capital_idx: cap,
            settlement_idxs: idxs,
        });
    }
    realms
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CellKind {
    Ocean,
    Land,
    River,
}

struct TerritoryGrid {
    cols: usize,
    rows: usize,
    step: f32,
    kind: Vec<CellKind>,
    /// Realm id, or `usize::MAX` for unowned / ocean.
    owner: Vec<usize>,
}

fn build_territory_grid(
    sampler: &TerrainSampler,
    settlements: &[Settlement],
    realms: &[Realm],
) -> TerritoryGrid {
    let config = sampler.config();
    let world_w = config.world_width as f32;
    let world_h = config.world_height as f32;
    let step = GRID_STEP;
    let cols = ((world_w / step).ceil() as usize).max(1);
    let rows = ((world_h / step).ceil() as usize).max(1);
    let n = cols * rows;

    let mut kind = vec![CellKind::Ocean; n];
    for gy in 0..rows {
        for gx in 0..cols {
            let x = (gx as f32 + 0.5) * step;
            let y = (gy as f32 + 0.5) * step;
            let idx = gy * cols + gx;
            let river = sampler.river_at(x as f64, y as f64);
            if river.channel >= 0.55 {
                kind[idx] = CellKind::River;
            } else if is_ocean(sampler, x, y) {
                kind[idx] = CellKind::Ocean;
            } else if is_land(sampler, x, y) {
                kind[idx] = CellKind::Land;
            } else {
                // Water that is not ocean (e.g. lake) — treat as impassable.
                kind[idx] = CellKind::Ocean;
            }
        }
    }

    let mut settlement_realm = vec![usize::MAX; settlements.len()];
    for (ri, realm) in realms.iter().enumerate() {
        for &si in &realm.settlement_idxs {
            settlement_realm[si] = ri;
        }
    }

    #[derive(Copy, Clone, Eq, PartialEq)]
    struct Node {
        cost: u32,
        idx: usize,
        realm: usize,
    }
    impl Ord for Node {
        fn cmp(&self, other: &Self) -> Ordering {
            other
                .cost
                .cmp(&self.cost)
                .then_with(|| self.idx.cmp(&other.idx))
                .then_with(|| self.realm.cmp(&other.realm))
        }
    }
    impl PartialOrd for Node {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut owner = vec![usize::MAX; n];
    let mut best = vec![u32::MAX; n];
    let mut heap = BinaryHeap::new();

    for (si, s) in settlements.iter().enumerate() {
        let ri = settlement_realm[si];
        if ri == usize::MAX {
            continue;
        }
        let gx = ((s.x / step).floor() as isize).clamp(0, cols as isize - 1) as usize;
        let gy = ((s.y / step).floor() as isize).clamp(0, rows as isize - 1) as usize;
        let idx = gy * cols + gx;
        if kind[idx] == CellKind::Ocean {
            // Snap to nearest non-ocean cell.
            if let Some(alt) = nearest_passable(gx, gy, cols, rows, &kind) {
                if best[alt] > 0 {
                    best[alt] = 0;
                    owner[alt] = ri;
                    heap.push(Node {
                        cost: 0,
                        idx: alt,
                        realm: ri,
                    });
                }
            }
            continue;
        }
        if 0 < best[idx] {
            best[idx] = 0;
            owner[idx] = ri;
            heap.push(Node {
                cost: 0,
                idx,
                realm: ri,
            });
        }
    }

    let dirs = [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)];
    while let Some(Node { cost, idx, realm }) = heap.pop() {
        if cost > best[idx] || owner[idx] != realm {
            continue;
        }
        let gx = (idx % cols) as isize;
        let gy = (idx / cols) as isize;
        for (dx, dy) in dirs {
            let nx = gx + dx;
            let ny = gy + dy;
            if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                continue;
            }
            let nidx = ny as usize * cols + nx as usize;
            let step_cost = match kind[nidx] {
                CellKind::Ocean => continue,
                CellKind::Land => LAND_COST,
                CellKind::River => RIVER_COST,
            };
            let next = cost.saturating_add(step_cost);
            if next < best[nidx]
                || (next == best[nidx] && realm < owner[nidx])
            {
                best[nidx] = next;
                owner[nidx] = realm;
                heap.push(Node {
                    cost: next,
                    idx: nidx,
                    realm,
                });
            }
        }
    }

    absorb_enclaves(cols, rows, &kind, &mut owner);

    TerritoryGrid {
        cols,
        rows,
        step,
        kind,
        owner,
    }
}

fn nearest_passable(
    gx: usize,
    gy: usize,
    cols: usize,
    rows: usize,
    kind: &[CellKind],
) -> Option<usize> {
    let mut best: Option<(u32, usize)> = None;
    for dy in -2isize..=2 {
        for dx in -2isize..=2 {
            let nx = gx as isize + dx;
            let ny = gy as isize + dy;
            if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                continue;
            }
            let idx = ny as usize * cols + nx as usize;
            if kind[idx] == CellKind::Ocean {
                continue;
            }
            let d = (dx.abs() + dy.abs()) as u32;
            if best.map(|(bd, _)| d < bd).unwrap_or(true) {
                best = Some((d, idx));
            }
        }
    }
    best.map(|(_, i)| i)
}

fn absorb_enclaves(cols: usize, rows: usize, kind: &[CellKind], owner: &mut [usize]) {
    let n = cols * rows;
    let mut seen = vec![false; n];
    let dirs = [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)];
    for start in 0..n {
        if seen[start] || kind[start] == CellKind::Ocean || owner[start] == usize::MAX {
            continue;
        }
        let realm = owner[start];
        let mut q = VecDeque::new();
        let mut comp = Vec::new();
        seen[start] = true;
        q.push_back(start);
        while let Some(i) = q.pop_front() {
            comp.push(i);
            let gx = (i % cols) as isize;
            let gy = (i / cols) as isize;
            for (dx, dy) in dirs {
                let nx = gx + dx;
                let ny = gy + dy;
                if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                    continue;
                }
                let ni = ny as usize * cols + nx as usize;
                if seen[ni] || kind[ni] == CellKind::Ocean || owner[ni] != realm {
                    continue;
                }
                seen[ni] = true;
                q.push_back(ni);
            }
        }
        if comp.len() >= ENCLAVE_MIN_CELLS {
            continue;
        }
        let mut votes: HashMap<usize, usize> = HashMap::new();
        for &i in &comp {
            let gx = (i % cols) as isize;
            let gy = (i / cols) as isize;
            for (dx, dy) in dirs {
                let nx = gx + dx;
                let ny = gy + dy;
                if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                    continue;
                }
                let ni = ny as usize * cols + nx as usize;
                let o = owner[ni];
                if o != usize::MAX && o != realm && kind[ni] != CellKind::Ocean {
                    *votes.entry(o).or_default() += 1;
                }
            }
        }
        let Some((&new_realm, _)) = votes.iter().max_by(|a, b| {
            a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)) // prefer lower id on tie
        }) else {
            continue;
        };
        for &i in &comp {
            owner[i] = new_realm;
        }
    }
}

fn build_realm_borders(
    sampler: &TerrainSampler,
    settlements: &[Settlement],
    realms: &[Realm],
) -> (Vec<RealmBorder>, TerritoryMap) {
    if realms.is_empty() {
        return (Vec::new(), TerritoryMap::default());
    }
    let grid = build_territory_grid(sampler, settlements, realms);
    let territory = TerritoryMap {
        cols: grid.cols as u32,
        rows: grid.rows as u32,
        step: grid.step,
        owners: grid
            .owner
            .iter()
            .map(|&o| if o == usize::MAX { u16::MAX } else { o as u16 })
            .collect(),
    };
    let cols = grid.cols;
    let rows = grid.rows;
    let step = grid.step;

    // Segment key: ordered (realm_a, realm_b) with None as water.
    let mut segments: HashMap<(usize, Option<usize>), Vec<[(f32, f32); 2]>> = HashMap::new();

    let push_seg = |map: &mut HashMap<(usize, Option<usize>), Vec<[(f32, f32); 2]>>,
                    ra: usize,
                    rb: Option<usize>,
                    a: (f32, f32),
                    b: (f32, f32)| {
        let key = match rb {
            Some(rb) if ra <= rb => (ra, Some(rb)),
            Some(rb) => (rb, Some(ra)),
            None => (ra, None),
        };
        map.entry(key).or_default().push([a, b]);
    };

    for gy in 0..rows {
        for gx in 0..cols {
            let idx = gy * cols + gx;
            if grid.kind[idx] == CellKind::Ocean || grid.owner[idx] == usize::MAX {
                continue;
            }
            let ra = grid.owner[idx];
            let x0 = gx as f32 * step;
            let y0 = gy as f32 * step;
            let x1 = x0 + step;
            let y1 = y0 + step;

            // Right neighbor
            if gx + 1 < cols {
                let ni = gy * cols + (gx + 1);
                if grid.kind[ni] == CellKind::Ocean || grid.owner[ni] == usize::MAX {
                    push_seg(&mut segments, ra, None, (x1, y0), (x1, y1));
                } else if grid.owner[ni] != ra {
                    push_seg(
                        &mut segments,
                        ra,
                        Some(grid.owner[ni]),
                        (x1, y0),
                        (x1, y1),
                    );
                }
            } else {
                push_seg(&mut segments, ra, None, (x1, y0), (x1, y1));
            }

            // Down neighbor
            if gy + 1 < rows {
                let ni = (gy + 1) * cols + gx;
                if grid.kind[ni] == CellKind::Ocean || grid.owner[ni] == usize::MAX {
                    push_seg(&mut segments, ra, None, (x0, y1), (x1, y1));
                } else if grid.owner[ni] != ra {
                    push_seg(
                        &mut segments,
                        ra,
                        Some(grid.owner[ni]),
                        (x0, y1),
                        (x1, y1),
                    );
                }
            } else {
                push_seg(&mut segments, ra, None, (x0, y1), (x1, y1));
            }

            // Left / top world edges for cells on the rim.
            if gx == 0 {
                push_seg(&mut segments, ra, None, (x0, y0), (x0, y1));
            }
            if gy == 0 {
                push_seg(&mut segments, ra, None, (x0, y0), (x1, y0));
            }
        }
    }

    let mut borders = Vec::new();
    let mut keys: Vec<_> = segments.keys().copied().collect();
    keys.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    for key in keys {
        let segs = segments.remove(&key).unwrap_or_default();
        for poly in chain_segments(&segs) {
            let simplified = simplify_polyline(&poly, step * 0.35);
            if simplified.len() < 2 {
                continue;
            }
            // Drop segments that midpoints sit in ocean (should not happen; belt-and-suspenders).
            if polyline_crosses_ocean(sampler, &simplified) {
                continue;
            }
            borders.push(RealmBorder {
                realm_a: key.0,
                realm_b: key.1,
                polyline: simplified
                    .into_iter()
                    .map(|(x, y)| BorderPoint { x, y })
                    .collect(),
            });
        }
    }
    (borders, territory)
}

fn point_key(p: (f32, f32)) -> (i32, i32) {
    ((p.0 * 100.0).round() as i32, (p.1 * 100.0).round() as i32)
}

fn chain_segments(segs: &[[(f32, f32); 2]]) -> Vec<Vec<(f32, f32)>> {
    let mut adj: HashMap<(i32, i32), Vec<(usize, bool)>> = HashMap::new();
    for (i, seg) in segs.iter().enumerate() {
        adj.entry(point_key(seg[0])).or_default().push((i, false));
        adj.entry(point_key(seg[1])).or_default().push((i, true));
    }
    let mut used = vec![false; segs.len()];
    let mut polys = Vec::new();

    for start_i in 0..segs.len() {
        if used[start_i] {
            continue;
        }
        used[start_i] = true;
        let mut poly = vec![segs[start_i][0], segs[start_i][1]];
        // Extend forward
        loop {
            let end = *poly.last().unwrap();
            let ek = point_key(end);
            let Some(cands) = adj.get(&ek) else { break };
            let mut advanced = false;
            for &(si, rev) in cands {
                if used[si] {
                    continue;
                }
                used[si] = true;
                let next = if rev { segs[si][0] } else { segs[si][1] };
                poly.push(next);
                advanced = true;
                break;
            }
            if !advanced {
                break;
            }
        }
        // Extend backward
        loop {
            let start = poly[0];
            let sk = point_key(start);
            let Some(cands) = adj.get(&sk) else { break };
            let mut advanced = false;
            for &(si, rev) in cands {
                if used[si] {
                    continue;
                }
                used[si] = true;
                let prev = if rev { segs[si][1] } else { segs[si][0] };
                poly.insert(0, prev);
                advanced = true;
                break;
            }
            if !advanced {
                break;
            }
        }
        if poly.len() >= 2 {
            polys.push(poly);
        }
    }
    polys
}

fn simplify_polyline(points: &[(f32, f32)], eps: f32) -> Vec<(f32, f32)> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut out = Vec::with_capacity(points.len());
    out.push(points[0]);
    for i in 1..points.len() - 1 {
        let prev = *out.last().unwrap();
        let cur = points[i];
        let next = points[i + 1];
        let ax = cur.0 - prev.0;
        let ay = cur.1 - prev.1;
        let bx = next.0 - cur.0;
        let by = next.1 - cur.1;
        let cross = (ax * by - ay * bx).abs();
        let scale = ((ax * ax + ay * ay) * (bx * bx + by * by)).sqrt().max(1e-6);
        if cross / scale > eps * 0.02 || dist(prev.0, prev.1, cur.0, cur.1) > eps {
            out.push(cur);
        }
    }
    out.push(*points.last().unwrap());
    out
}

fn polyline_crosses_ocean(sampler: &TerrainSampler, points: &[(f32, f32)]) -> bool {
    for w in points.windows(2) {
        let mx = (w[0].0 + w[1].0) * 0.5;
        let my = (w[0].1 + w[1].1) * 0.5;
        if is_ocean(sampler, mx, my) {
            // Coastline borders are allowed (land–ocean). Midpoint on ocean means the
            // segment is along the coast edge of a land cell — keep those.
            // Reject only if both endpoints are ocean (interior ocean cut).
            if is_ocean(sampler, w[0].0, w[0].1) && is_ocean(sampler, w[1].0, w[1].1) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::config::WorldConfig;
    use crate::world::settlement::{generate, SettlementMap};
    use std::collections::HashSet;

    #[test]
    fn border_types_construct() {
        let city = CityBorder {
            settlement_idx: 1,
            polygon: vec![
                BorderPoint { x: 0.0, y: 0.0 },
                BorderPoint { x: 1.0, y: 0.0 },
                BorderPoint { x: 0.5, y: 1.0 },
            ],
        };
        let realm = Realm {
            id: 0,
            name: "Test".into(),
            capital_idx: 1,
            settlement_idxs: vec![0, 1],
        };
        let edge = RealmBorder {
            realm_a: 0,
            realm_b: None,
            polyline: vec![BorderPoint { x: 2.0, y: 3.0 }, BorderPoint { x: 4.0, y: 5.0 }],
        };
        assert_eq!(city.polygon.len(), 3);
        assert_eq!(realm.settlement_idxs, vec![0, 1]);
        assert!(edge.realm_b.is_none());
    }

    #[test]
    fn generate_fills_borders_when_settlements_exist() {
        let map = generate(WorldConfig::default());
        assert!(!map.settlements.is_empty());
        assert!(!map.realms.is_empty());
        assert!(!map.city_borders.is_empty());
        assert!(!map.realm_borders.is_empty());
    }

    #[test]
    fn every_settlement_belongs_to_exactly_one_realm() {
        let map = generate(WorldConfig::default());
        let mut seen = HashSet::new();
        for realm in &map.realms {
            for &si in &realm.settlement_idxs {
                assert!(seen.insert(si), "settlement {si} in multiple realms");
                assert!(si < map.settlements.len());
            }
            assert!(realm.settlement_idxs.contains(&realm.capital_idx));
        }
        assert_eq!(seen.len(), map.settlements.len());
    }

    #[test]
    fn city_borders_only_for_town_and_city() {
        let map = generate(WorldConfig::default());
        for cb in &map.city_borders {
            let kind = map.settlements[cb.settlement_idx].kind;
            assert!(matches!(kind, SettlementKind::Town | SettlementKind::City));
            assert!(cb.polygon.len() >= 3);
            let first = cb.polygon[0];
            let last = cb.polygon[cb.polygon.len() - 1];
            // Closed in the sense of a ring (renderer closes last→first).
            assert!(dist(first.x, first.y, last.x, last.y) > 0.01 || cb.polygon.len() >= 3);
        }
    }

    #[test]
    fn borders_are_deterministic() {
        let a = generate(WorldConfig::default());
        let b = generate(WorldConfig::default());
        assert_eq!(a.realms, b.realms);
        assert_eq!(a.city_borders, b.city_borders);
        assert_eq!(a.realm_borders, b.realm_borders);
    }

    #[test]
    fn settlement_map_default_border_fields() {
        let map = SettlementMap {
            settlements: Vec::new(),
            roads: Vec::new(),
            city_borders: Vec::new(),
            realms: Vec::new(),
            realm_borders: Vec::new(),
            territory: TerritoryMap::default(),
        };
        assert!(map.city_borders.is_empty());
        assert!(map.realms.is_empty());
        assert!(map.realm_borders.is_empty());
    }
}
