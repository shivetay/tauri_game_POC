use crate::world::elevation::WATER_M;
use crate::world::noise::clamp01;
use crate::world::prng::unit_noise;

/// Strength of bank fertility boost applied in the sampler.
pub const RIVER_MOISTURE_BOOST: f32 = 0.48;

/// Shallow carved channel elevation (m) — below sea level → Water biome.
pub const RIVER_CHANNEL_ELEV_M: f32 = -35.0;

const CHANNEL_SRC: u64 = 701;
const CHANNEL_PICK: u64 = 702;
const CHANNEL_TRIB: u64 = 703;
const CHANNEL_MEANDER: u64 = 704;
const CHANNEL_DELTA: u64 = 705;

const GRID_STEP: f32 = 4.0;
/// Uniform cartographic channel half-width (mains, tributaries, delta arms).
const CHANNEL_HALF_W: f32 = 0.95;
const BANK_MULT: f32 = 3.2;
const MAIN_COUNT: usize = 9;
const TRIB_COUNT: usize = 7;
const SRC_MIN_SEP: f32 = 28.0;
const MAX_FLOW_STEPS: usize = 2_000;
const BIN_SIZE: f32 = 16.0;
const INERTIA: f32 = 0.32;
const MEANDER_AMP: f32 = 1.35;
const DELTA_ARM_LEN: f32 = 10.0;

pub struct RiverSample {
    /// 0 = dry land, 1 = in the river channel.
    pub channel: f32,
    /// 0 = far, 1 = on the bank / near channel (fertility falloff).
    pub proximity: f32,
}

#[derive(Clone)]
struct Polyline {
    points: Vec<(f32, f32)>,
    half_w: f32,
}

#[derive(Clone)]
struct Lake {
    x: f32,
    y: f32,
    radius: f32,
}

/// Deterministic hydrology network: sources → downhill flow → mouth / lake / join.
pub struct RiverNetwork {
    polylines: Vec<Polyline>,
    lakes: Vec<Lake>,
    /// Spatial bins: flat `(by * bins_x + bx) → segment indices`.
    bins: Vec<Vec<usize>>,
    bins_x: u32,
    bins_y: u32,
    world_w: f32,
    world_h: f32,
    source_count: usize,
}

impl RiverNetwork {
    pub fn empty(world_w: f32, world_h: f32) -> Self {
        Self {
            polylines: Vec::new(),
            lakes: Vec::new(),
            bins: Vec::new(),
            bins_x: 0,
            bins_y: 0,
            world_w,
            world_h,
            source_count: 0,
        }
    }

    /// Build from a base elevation oracle (no river carve).
    pub fn build(
        seed: u64,
        world_w: u32,
        world_h: u32,
        elev_m: impl Fn(f64, f64) -> f32,
    ) -> Self {
        let ww = world_w as f32;
        let wh = world_h as f32;
        let cols = ((ww / GRID_STEP).floor() as usize).max(8);
        let rows = ((wh / GRID_STEP).floor() as usize).max(8);

        let mut elev = vec![0.0f32; cols * rows];
        for gy in 0..rows {
            for gx in 0..cols {
                let x = (gx as f32 + 0.5) * GRID_STEP;
                let y = (gy as f32 + 0.5) * GRID_STEP;
                elev[gy * cols + gx] = elev_m(x as f64, y as f64);
            }
        }

        let mut occupied = vec![false; cols * rows];
        let mut polylines: Vec<Polyline> = Vec::new();
        let mut lakes: Vec<Lake> = Vec::new();

        let mains = pick_sources(seed, CHANNEL_SRC, &elev, cols, rows, MAIN_COUNT, true);
        let mut source_count = 0usize;
        for (si, (gx, gy)) in mains.iter().enumerate() {
            if trace_flow(
                seed,
                si as u64,
                *gx,
                *gy,
                &elev,
                cols,
                rows,
                &mut occupied,
                &mut polylines,
                &mut lakes,
            ) {
                source_count += 1;
            }
        }

        let tribs = pick_sources(seed, CHANNEL_TRIB, &elev, cols, rows, TRIB_COUNT, false);
        for (ti, (gx, gy)) in tribs.iter().enumerate() {
            let idx = gy * cols + gx;
            if occupied[idx] {
                continue;
            }
            if elev[idx] < WATER_M {
                continue;
            }
            // Prefer mid elevations for tributaries.
            if elev[idx] < 400.0 || elev[idx] > 4_500.0 {
                continue;
            }
            if trace_flow(
                seed,
                10_000 + ti as u64,
                *gx,
                *gy,
                &elev,
                cols,
                rows,
                &mut occupied,
                &mut polylines,
                &mut lakes,
            ) {
                source_count += 1;
            }
        }

        let mut net = Self {
            polylines,
            lakes,
            bins: Vec::new(),
            bins_x: 0,
            bins_y: 0,
            world_w: ww,
            world_h: wh,
            source_count,
        };
        net.build_bins();
        net
    }

    pub fn source_count(&self) -> usize {
        self.source_count
    }

    pub fn lake_count(&self) -> usize {
        self.lakes.len()
    }

    pub fn polyline_count(&self) -> usize {
        self.polylines.len()
    }

    fn build_bins(&mut self) {
        self.bins_x = ((self.world_w / BIN_SIZE).ceil() as u32).max(1);
        self.bins_y = ((self.world_h / BIN_SIZE).ceil() as u32).max(1);
        let n = (self.bins_x * self.bins_y) as usize;
        self.bins = vec![Vec::new(); n];
        for (si, line) in self.polylines.iter().enumerate() {
            for w in line.points.windows(2) {
                let (ax, ay) = w[0];
                let (bx, by) = w[1];
                let min_x = ax.min(bx) - line.half_w - 2.0;
                let max_x = ax.max(bx) + line.half_w + 2.0;
                let min_y = ay.min(by) - line.half_w - 2.0;
                let max_y = ay.max(by) + line.half_w + 2.0;
                let x0 = ((min_x / BIN_SIZE).floor() as i32).max(0) as u32;
                let x1 = ((max_x / BIN_SIZE).floor() as i32)
                    .clamp(0, self.bins_x as i32 - 1) as u32;
                let y0 = ((min_y / BIN_SIZE).floor() as i32).max(0) as u32;
                let y1 = ((max_y / BIN_SIZE).floor() as i32)
                    .clamp(0, self.bins_y as i32 - 1) as u32;
                for by in y0..=y1 {
                    for bx in x0..=x1 {
                        let bi = (by * self.bins_x + bx) as usize;
                        if !self.bins[bi].contains(&si) {
                            self.bins[bi].push(si);
                        }
                    }
                }
            }
        }
    }

    pub fn sample(&self, world_x: f64, world_y: f64) -> RiverSample {
        let px = world_x as f32;
        let py = world_y as f32;
        if self.polylines.is_empty() && self.lakes.is_empty() {
            return RiverSample {
                channel: 0.0,
                proximity: 0.0,
            };
        }

        let mut best_chan = 0.0f32;
        let mut best_prox = 0.0f32;

        for lake in &self.lakes {
            let dx = px - lake.x;
            let dy = py - lake.y;
            let d = (dx * dx + dy * dy).sqrt();
            let bank = lake.radius * 1.85;
            if d <= lake.radius {
                best_chan = 1.0;
                best_prox = 1.0;
            } else if d < bank {
                let t = 1.0 - (d - lake.radius) / (bank - lake.radius).max(0.01);
                best_prox = best_prox.max(t);
            }
        }

        if !self.bins.is_empty() {
            let bx = ((px / BIN_SIZE).floor() as i32).clamp(0, self.bins_x as i32 - 1) as u32;
            let by = ((py / BIN_SIZE).floor() as i32).clamp(0, self.bins_y as i32 - 1) as u32;
            for oy in -1i32..=1 {
                for ox in -1i32..=1 {
                    let nx = bx as i32 + ox;
                    let ny = by as i32 + oy;
                    if nx < 0 || ny < 0 || nx >= self.bins_x as i32 || ny >= self.bins_y as i32 {
                        continue;
                    }
                    let bi = (ny as u32 * self.bins_x + nx as u32) as usize;
                    for &si in &self.bins[bi] {
                        let line = &self.polylines[si];
                        let d = dist_to_polyline(px, py, &line.points);
                        let half = line.half_w;
                        let bank = half * BANK_MULT;
                        if d <= half {
                            let t = 1.0 - (d / half.max(0.01)) * 0.15;
                            best_chan = best_chan.max(t);
                            best_prox = best_prox.max(1.0);
                        } else if d < bank {
                            let t = 1.0 - (d - half) / (bank - half).max(0.01);
                            best_prox = best_prox.max(t);
                        }
                    }
                }
            }
        }

        RiverSample {
            channel: clamp01(best_chan as f64) as f32,
            proximity: clamp01(best_prox as f64) as f32,
        }
    }
}

fn cell_xy(gx: usize, gy: usize) -> (f32, f32) {
    ((gx as f32 + 0.5) * GRID_STEP, (gy as f32 + 0.5) * GRID_STEP)
}

fn pick_sources(
    seed: u64,
    channel: u64,
    elev: &[f32],
    cols: usize,
    rows: usize,
    want: usize,
    highland: bool,
) -> Vec<(usize, usize)> {
    let mut scored: Vec<(f32, usize, usize)> = Vec::new();
    for gy in 2..rows.saturating_sub(2) {
        for gx in 2..cols.saturating_sub(2) {
            let e = elev[gy * cols + gx];
            if e < WATER_M {
                continue;
            }
            if highland {
                if e < 1_200.0 {
                    continue;
                }
            } else if e < 500.0 {
                continue;
            }
            let idx = (gy * cols + gx) as u64;
            let jitter = unit_noise(seed, channel, idx) as f32;
            let score = e * 0.001 + jitter;
            scored.push((score, gx, gy));
        }
    }
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    let mut out: Vec<(usize, usize)> = Vec::new();
    let sep2 = SRC_MIN_SEP * SRC_MIN_SEP;
    for &(_, gx, gy) in &scored {
        if out.len() >= want {
            break;
        }
        let (x, y) = cell_xy(gx, gy);
        let ok = out.iter().all(|&(ox, oy)| {
            let (px, py) = cell_xy(ox, oy);
            let dx = x - px;
            let dy = y - py;
            dx * dx + dy * dy >= sep2
        });
        if ok {
            // Extra deterministic gate so we do not over-fill every highland peak.
            let gate = unit_noise(seed, CHANNEL_PICK, (gy * cols + gx) as u64);
            if gate < 0.72 {
                out.push((gx, gy));
            }
        }
    }
    out
}

const NEIGH: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

fn trace_flow(
    seed: u64,
    path_id: u64,
    start_gx: usize,
    start_gy: usize,
    elev: &[f32],
    cols: usize,
    rows: usize,
    occupied: &mut [bool],
    polylines: &mut Vec<Polyline>,
    lakes: &mut Vec<Lake>,
) -> bool {
    let start_i = start_gy * cols + start_gx;
    if elev[start_i] < WATER_M || occupied[start_i] {
        return false;
    }

    let mut points: Vec<(f32, f32)> = Vec::new();
    let mut gx = start_gx;
    let mut gy = start_gy;
    let mut prev_dir: Option<(i32, i32)> = None;
    let mut joined = false;
    let mut reached_ocean = false;
    let mut made_lake = false;
    let mut mouth: Option<(f32, f32, f32, f32)> = None;

    for step in 0..MAX_FLOW_STEPS {
        let idx = gy * cols + gx;
        let e = elev[idx];
        if e < WATER_M {
            reached_ocean = true;
            let (mx, my) = cell_xy(gx, gy);
            points.push((mx, my));
            if points.len() >= 2 {
                let (ax, ay) = points[points.len() - 2];
                mouth = Some((ax, ay, mx - ax, my - ay));
            }
            break;
        }

        // Join existing river (not our own freshly marked path start).
        if occupied[idx] && !points.is_empty() {
            points.push(cell_xy(gx, gy));
            joined = true;
            break;
        }

        occupied[idx] = true;
        points.push(cell_xy(gx, gy));

        // Choose downhill neighbor with soft inertia.
        let mut best: Option<(f32, usize, usize, i32, i32)> = None;
        for &(dx, dy) in &NEIGH {
            let nx = gx as i32 + dx;
            let ny = gy as i32 + dy;
            if nx < 0 || ny < 0 || nx >= cols as i32 || ny >= rows as i32 {
                continue;
            }
            let ngx = nx as usize;
            let ngy = ny as usize;
            let ne = elev[ngy * cols + ngx];
            let mut cost = ne;
            if let Some((pdx, pdy)) = prev_dir {
                let same = dx == pdx && dy == pdy;
                let soft = (dx - pdx).abs() + (dy - pdy).abs() <= 1;
                if same {
                    cost -= INERTIA * 80.0;
                } else if soft {
                    cost -= INERTIA * 35.0;
                }
            }
            // Soft lateral preference so courses bend before smoothing.
            let nudge = unit_noise(seed, path_id, (step as u64) * 17 + (ngy * cols + ngx) as u64)
                as f32
                * 12.0;
            cost += nudge;
            let better = match best {
                None => true,
                Some((bc, ..)) => cost < bc,
            };
            if better {
                best = Some((cost, ngx, ngy, dx, dy));
            }
        }

        let Some((_, ngx, ngy, dx, dy)) = best else {
            break;
        };

        let ne = elev[ngy * cols + ngx];
        if ne >= e - 0.5 {
            // Local sink / flat — form a lake, then try pour point.
            let (lx, ly) = cell_xy(gx, gy);
            let radius = (2.2 + (points.len() as f32) * 0.012).clamp(2.2, 6.0);
            lakes.push(Lake {
                x: lx,
                y: ly,
                radius,
            });
            made_lake = true;
            mark_lake_occupied(gx, gy, radius, cols, rows, occupied);

            if let Some((px, py, pdx, pdy)) =
                find_pour_point(gx, gy, elev, cols, rows, occupied, radius)
            {
                prev_dir = Some((pdx, pdy));
                gx = px;
                gy = py;
                continue;
            }
            break;
        }

        prev_dir = Some((dx, dy));
        gx = ngx;
        gy = ngy;
    }

    if points.len() < 4 {
        return false;
    }
    // Accept paths that reach ocean, join, or end in a lake.
    if !(reached_ocean || joined || made_lake) {
        // Still keep long downhill runs that hit the map edge / flat end.
        if points.len() < 12 {
            return false;
        }
    }

    let points = naturalize_polyline(seed, path_id, points);
    polylines.push(Polyline {
        points,
        half_w: CHANNEL_HALF_W,
    });

    if let Some((ox, oy, mdx, mdy)) = mouth {
        push_delta_arms(seed, path_id, ox, oy, mdx, mdy, elev, cols, rows, polylines);
    }

    true
}

/// Chaikin corner-cutting + light meander — soft atlas curves, not grid zigzags.
fn naturalize_polyline(seed: u64, path_id: u64, raw: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    if raw.len() < 3 {
        return raw;
    }
    let mut pts = chaikin(&raw, 2);
    // Perpendicular meander (skip endpoints so mouths/sources stay put).
    let n = pts.len();
    for i in 1..n.saturating_sub(1) {
        let (x1, y1) = pts[i];
        let (x0, y0) = pts[i - 1];
        let (x2, y2) = pts[i + 1];
        let tx = x2 - x0;
        let ty = y2 - y0;
        let len = (tx * tx + ty * ty).sqrt().max(1e-3);
        let nx = -ty / len;
        let ny = tx / len;
        let t = i as f64 / (n as f64);
        // Envelope: stronger mid-course, weaker near ends.
        let env = (t * std::f64::consts::PI).sin() as f32;
        let wave = (unit_noise(seed, CHANNEL_MEANDER, path_id.wrapping_add(i as u64)) as f32 - 0.5)
            * 2.0;
        let amp = MEANDER_AMP * env * wave;
        pts[i] = (x1 + nx * amp, y1 + ny * amp);
    }
    chaikin(&pts, 1)
}

fn chaikin(points: &[(f32, f32)], rounds: usize) -> Vec<(f32, f32)> {
    let mut cur = points.to_vec();
    for _ in 0..rounds {
        if cur.len() < 2 {
            break;
        }
        let mut next = Vec::with_capacity(cur.len() * 2);
        next.push(cur[0]);
        for w in cur.windows(2) {
            let (ax, ay) = w[0];
            let (bx, by) = w[1];
            next.push((0.75 * ax + 0.25 * bx, 0.75 * ay + 0.25 * by));
            next.push((0.25 * ax + 0.75 * bx, 0.25 * ay + 0.75 * by));
        }
        next.push(*cur.last().unwrap());
        cur = next;
    }
    cur
}

/// Fan short arms into the sea at a river mouth (delta).
fn push_delta_arms(
    seed: u64,
    path_id: u64,
    origin_x: f32,
    origin_y: f32,
    dir_x: f32,
    dir_y: f32,
    elev: &[f32],
    cols: usize,
    rows: usize,
    polylines: &mut Vec<Polyline>,
) {
    let len = (dir_x * dir_x + dir_y * dir_y).sqrt().max(1e-3);
    let fx = dir_x / len;
    let fy = dir_y / len;
    let px = -fy;
    let py = fx;

    let roll = unit_noise(seed, CHANNEL_DELTA, path_id);
    // 2 or 3 arms; skip tiny streams.
    let arms = if roll < 0.35 { 2 } else { 3 };
    let spreads: &[f32] = if arms == 2 {
        &[-0.55, 0.55]
    } else {
        &[-0.75, 0.0, 0.75]
    };

    for (ai, &spread) in spreads.iter().enumerate() {
        // Skip the center arm when 3-way — main polyline already enters the sea.
        if arms == 3 && ai == 1 {
            continue;
        }
        let mut arm = Vec::with_capacity(6);
        arm.push((origin_x, origin_y));
        for s in 1..=4 {
            let t = s as f32 / 4.0;
            let along = DELTA_ARM_LEN * t;
            let side = spread * DELTA_ARM_LEN * 0.55 * t;
            let wobble = (unit_noise(
                seed,
                CHANNEL_DELTA,
                path_id.wrapping_add(100 + ai as u64 * 10 + s as u64),
            ) as f32
                - 0.5)
                * 1.1
                * t;
            let x = origin_x + fx * along + px * (side + wobble);
            let y = origin_y + fy * along + py * (side + wobble);
            arm.push((x, y));
        }
        // Keep only arms that reach water or leave the land mass.
        let tip = *arm.last().unwrap();
        let tip_ocean = point_is_ocean(tip.0, tip.1, elev, cols, rows);
        if !tip_ocean && roll > 0.85 {
            continue;
        }
        let arm = naturalize_polyline(seed, path_id.wrapping_add(50 + ai as u64), arm);
        polylines.push(Polyline {
            points: arm,
            half_w: CHANNEL_HALF_W,
        });
    }
}

fn point_is_ocean(x: f32, y: f32, elev: &[f32], cols: usize, rows: usize) -> bool {
    let gx = ((x / GRID_STEP).floor() as i32).clamp(0, cols as i32 - 1) as usize;
    let gy = ((y / GRID_STEP).floor() as i32).clamp(0, rows as i32 - 1) as usize;
    elev[gy * cols + gx] < WATER_M
}

fn mark_lake_occupied(
    gx: usize,
    gy: usize,
    radius: f32,
    cols: usize,
    rows: usize,
    occupied: &mut [bool],
) {
    let r_cells = ((radius / GRID_STEP).ceil() as i32).max(1);
    for dy in -r_cells..=r_cells {
        for dx in -r_cells..=r_cells {
            let nx = gx as i32 + dx;
            let ny = gy as i32 + dy;
            if nx < 0 || ny < 0 || nx >= cols as i32 || ny >= rows as i32 {
                continue;
            }
            let (cx, cy) = cell_xy(nx as usize, ny as usize);
            let (ox, oy) = cell_xy(gx, gy);
            let ddx = cx - ox;
            let ddy = cy - oy;
            if ddx * ddx + ddy * ddy <= radius * radius {
                occupied[ny as usize * cols + nx as usize] = true;
            }
        }
    }
}

fn find_pour_point(
    gx: usize,
    gy: usize,
    elev: &[f32],
    cols: usize,
    rows: usize,
    occupied: &[bool],
    radius: f32,
) -> Option<(usize, usize, i32, i32)> {
    let r_cells = ((radius / GRID_STEP).ceil() as i32 + 1).max(2);
    let mut best: Option<(f32, usize, usize, i32, i32)> = None;
    for dy in -r_cells..=r_cells {
        for dx in -r_cells..=r_cells {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = gx as i32 + dx;
            let ny = gy as i32 + dy;
            if nx < 0 || ny < 0 || nx >= cols as i32 || ny >= rows as i32 {
                continue;
            }
            let ngx = nx as usize;
            let ngy = ny as usize;
            let (cx, cy) = cell_xy(ngx, ngy);
            let (ox, oy) = cell_xy(gx, gy);
            let ddx = cx - ox;
            let ddy = cy - oy;
            if ddx * ddx + ddy * ddy <= radius * radius {
                continue;
            }
            let e = elev[ngy * cols + ngx];
            if occupied[ngy * cols + ngx] && e >= WATER_M {
                // Prefer joining an existing channel outside the lake.
                return Some((ngx, ngy, dx.signum(), dy.signum()));
            }
            let better = match best {
                None => true,
                Some((be, ..)) => e < be,
            };
            if better {
                best = Some((e, ngx, ngy, dx.signum(), dy.signum()));
            }
        }
    }
    best.map(|(_, x, y, dx, dy)| (x, y, dx, dy))
}

fn dist_to_polyline(px: f32, py: f32, points: &[(f32, f32)]) -> f32 {
    let mut best = f32::INFINITY;
    for w in points.windows(2) {
        best = best.min(dist_point_seg(px, py, w[0].0, w[0].1, w[1].0, w[1].1));
    }
    if points.len() == 1 {
        let dx = px - points[0].0;
        let dy = py - points[0].1;
        return (dx * dx + dy * dy).sqrt();
    }
    best
}

fn dist_point_seg(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let apx = px - ax;
    let apy = py - ay;
    let ab2 = abx * abx + aby * aby;
    let t = if ab2 < 1e-8 {
        0.0
    } else {
        ((apx * abx + apy * aby) / ab2).clamp(0.0, 1.0)
    };
    let qx = ax + abx * t;
    let qy = ay + aby * t;
    let dx = px - qx;
    let dy = py - qy;
    (dx * dx + dy * dy).sqrt()
}

/// True when water should block roads (ocean/lake), false for fordable river channels.
pub fn blocks_road(elevation_m: f32, river_channel: f32) -> bool {
    if elevation_m >= 0.0 {
        return false;
    }
    // Shallow carved river — allow fords. Lakes are deeper discs carved the same
    // elev, but wider; treat strong channel away from thin rivers as blocking via
    // elevation alone when clearly below channel floor.
    if river_channel > 0.45 && elevation_m >= RIVER_CHANNEL_ELEV_M - 1.0 {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::config::WorldConfig;
    use crate::world::sampler::TerrainSampler;

    #[test]
    fn river_sample_is_deterministic() {
        let sampler = TerrainSampler::new(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let a = sampler.river_at(180.0, 220.0);
        let b = sampler.river_at(180.0, 220.0);
        assert!((a.channel - b.channel).abs() < 1e-6);
        assert!((a.proximity - b.proximity).abs() < 1e-6);
    }

    #[test]
    fn network_has_sources_and_paths() {
        let sampler = TerrainSampler::new(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        assert!(
            sampler.river_network().source_count() >= 1,
            "expected at least one river source"
        );
        assert!(
            sampler.river_network().polyline_count() >= 1,
            "expected at least one river path"
        );
    }

    #[test]
    fn channel_implies_proximity() {
        let sampler = TerrainSampler::new(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let mut found = false;
        for yi in 0..256 {
            for xi in 0..256 {
                let x = f64::from(xi) * 2.0;
                let y = f64::from(yi) * 2.0;
                let s = sampler.river_at(x, y);
                if s.channel > 0.55 {
                    assert!(
                        s.proximity >= s.channel - 0.05,
                        "bank proximity should cover the channel"
                    );
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "expected at least one channel sample for seed 6");
    }

    #[test]
    fn two_samplers_same_seed_match_network() {
        let a = TerrainSampler::new(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        let b = TerrainSampler::new(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        assert_eq!(a.river_network().source_count(), b.river_network().source_count());
        assert_eq!(
            a.river_network().polyline_count(),
            b.river_network().polyline_count()
        );
        assert_eq!(a.river_network().lake_count(), b.river_network().lake_count());
    }

    #[test]
    fn channel_widths_are_uniform() {
        let sampler = TerrainSampler::new(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        assert!(sampler.river_network().polyline_count() >= 1);
        assert!(
            sampler
                .river_network()
                .polylines
                .iter()
                .all(|p| (p.half_w - CHANNEL_HALF_W).abs() < 1e-6),
            "all rivers must share uniform cartographic width"
        );
    }

    #[test]
    fn ocean_mouths_can_form_delta_arms() {
        let sampler = TerrainSampler::new(WorldConfig {
            seed: 6,
            ..WorldConfig::default()
        });
        // Deltas add extra polylines beyond source traces.
        assert!(
            sampler.river_network().polyline_count() > sampler.river_network().source_count(),
            "expected delta (or extra) arms beyond stem sources"
        );
    }

    #[test]
    fn shallow_river_does_not_block_roads() {
        assert!(!blocks_road(RIVER_CHANNEL_ELEV_M, 0.9));
        assert!(blocks_road(-500.0, 0.0));
        assert!(!blocks_road(10.0, 0.0));
    }
}
