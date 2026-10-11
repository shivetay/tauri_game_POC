use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use crate::colors::BIOMES;
use crate::game_loop::GameLoop;
use crate::perf::PerfStats;
use crate::settlements_draw::{
    settlement_info, RoadDetail, SettlementDrawStyle, SettlementHit, WorldBounds,
};
use crate::world::config::{TerrainGenParams, WorldConfig};
use crate::world::ecology::{
    generate_chunk_ecology_sampled, generate_region_ecology_sampled, ChunkEcologyMap,
    RegionEcologyMap,
};
use crate::world::elevation::{is_water_biome, HIGH_M};
use crate::world::grid::{
    generate_global_grid_sampled, generate_region_grid_sampled, generate_view_grid_sampled,
};
use crate::world::sampler::TerrainSampler;
use crate::world::npc::{generate as generate_npcs, NpcMap};
use crate::world::settlement::{
    generate_sampled as generate_settlements_sampled, Road, RoadKind, Settlement, SettlementKind,
    SettlementMap,
};
use crate::world::types::{ChunkId, RegionId, TerrainGrid, TileType};

/// On-road speed: 5 km/h with 1 world unit = 1 km.
const WALK_ROAD_WU_PER_GAME_HOUR: f32 = 5.0;
/// Off-road speed (no visible road underfoot).
const WALK_OFFROAD_WU_PER_GAME_HOUR: f32 = 2.0;
/// Distance below which the player is considered "already at" a destination.
const AT_PLACE_EPS: f32 = 0.35;
/// Vision / fog radius in world units (km).
pub(crate) const VISION_RADIUS: f32 = 3.0;
/// How close to a road polyline counts as on the road ribbon (bridges / walkability).
const ROAD_TRAVEL_EPS: f32 = 1.0;
/// Max distance to snap start/goal onto the road network for routed travel.
const ROAD_SNAP_EPS: f32 = 5.0;
/// Join nearby road points (crossings) into one graph.
const ROAD_NODE_LINK_EPS: f32 = 1.5;
/// |dot(move, segment)| above this ⇒ traveling along the road (not merely crossing).
const ROAD_ALIGN_MIN: f32 = 0.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinal {
    North,
    South,
    East,
    West,
}

impl Cardinal {
    pub const ALL: [Cardinal; 4] = [
        Cardinal::North,
        Cardinal::South,
        Cardinal::East,
        Cardinal::West,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::North => "Północ",
            Self::South => "Południe",
            Self::East => "Wschód",
            Self::West => "Zachód",
        }
    }

    fn delta(self) -> (f32, f32) {
        match self {
            Self::North => (0.0, -1.0),
            Self::South => (0.0, 1.0),
            Self::West => (-1.0, 0.0),
            Self::East => (1.0, 0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodLevel {
    Global,
    Region(RegionId),
    Chunk { region: RegionId, chunk: ChunkId },
}

impl LodLevel {
    pub(crate) fn bounds(self, config: &WorldConfig) -> WorldBounds {
        match self {
            Self::Global => WorldBounds {
                x0: 0.0,
                y0: 0.0,
                span: config.world_width as f32,
            },
            Self::Region(region) => WorldBounds {
                x0: region.rx as f32 * config.region_size as f32,
                y0: region.ry as f32 * config.region_size as f32,
                span: config.region_size as f32,
            },
            Self::Chunk { region, chunk } => WorldBounds {
                x0: region.rx as f32 * config.region_size as f32
                    + chunk.cx as f32 * config.chunk_size as f32,
                y0: region.ry as f32 * config.region_size as f32
                    + chunk.cy as f32 * config.chunk_size as f32,
                span: config.chunk_size as f32,
            },
        }
    }

    pub(crate) fn idle_label(self, has_player_spawn: bool) -> String {
        match self {
            Self::Global => "Mapa świata — kliknij region · kliknij miasto po info".to_string(),
            Self::Region(r) => {
                format!("Region ({}, {}) — kliknij obszar · kliknij miasto po info", r.rx, r.ry)
            }
            Self::Chunk { chunk, .. } => {
                if has_player_spawn {
                    format!(
                        "Obszar ({}, {}) — kliknij ląd, by iść · biom/miasto",
                        chunk.cx, chunk.cy
                    )
                } else {
                    format!(
                        "Obszar ({}, {}) — kliknij ląd lub osadę (Spawn) · biom/gatunki",
                        chunk.cx, chunk.cy
                    )
                }
            }
        }
    }

    pub(crate) fn back_label(self) -> Option<String> {
        match self {
            Self::Global => None,
            Self::Region(_) => Some("← Mapa świata".to_string()),
            Self::Chunk { region, .. } => Some(format!("← Region ({}, {})", region.rx, region.ry)),
        }
    }
}

struct GenRequest {
    id: u64,
    seed: u64,
    params: TerrainGenParams,
    lod: LodLevel,
    /// Reuse settlements when only LOD changed (same seed/params).
    settlements: Option<SettlementMap>,
    /// Reuse NPCs with settlements (same seed/params).
    npcs: Option<NpcMap>,
}

struct GenResult {
    id: u64,
    seed: u64,
    lod: LodLevel,
    grid: TerrainGrid,
    settlements: SettlementMap,
    npcs: NpcMap,
    habitat: Option<RegionEcologyMap>,
    life: Option<ChunkEcologyMap>,
}

/// Cached terrain view for one LOD — avoids regenerating on zoom/back.
#[derive(Clone)]
struct LodView {
    grid: TerrainGrid,
    habitat: Option<RegionEcologyMap>,
    life: Option<ChunkEcologyMap>,
}

#[derive(Default)]
struct LodCache {
    global: Option<LodView>,
    region: Option<(RegionId, LodView)>,
    chunk: Option<(RegionId, ChunkId, LodView)>,
}

impl LodCache {
    fn clear(&mut self) {
        *self = Self::default();
    }

    fn store(&mut self, lod: LodLevel, view: LodView) {
        match lod {
            LodLevel::Global => self.global = Some(view),
            LodLevel::Region(region) => self.region = Some((region, view)),
            LodLevel::Chunk { region, chunk } => self.chunk = Some((region, chunk, view)),
        }
    }

    fn get(&self, lod: LodLevel) -> Option<&LodView> {
        match lod {
            LodLevel::Global => self.global.as_ref(),
            LodLevel::Region(region) => self
                .region
                .as_ref()
                .filter(|(r, _)| *r == region)
                .map(|(_, v)| v),
            LodLevel::Chunk { region, chunk } => self
                .chunk
                .as_ref()
                .filter(|(r, c, _)| *r == region && *c == chunk)
                .map(|(_, _, v)| v),
        }
    }
}

pub struct PlaceRow {
    pub(crate) index: usize,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) name: String,
    pub(crate) kind: String,
    kind_rank: u8,
    pub(crate) population: u32,
    pub(crate) walled: bool,
}

pub struct MapSession {
    pub(crate) seed: u64,
    pub(crate) draft_seed: String,
    pub(crate) seed_error: Option<String>,
    pub(crate) params: TerrainGenParams,
    pub(crate) draft_params: TerrainGenParams,
    pub(crate) show_seed_info: bool,
    pub(crate) lod: LodLevel,
    pub(crate) grid: Option<TerrainGrid>,
    pub(crate) settlements: Option<SettlementMap>,
    pub(crate) npcs: Option<NpcMap>,
    pub(crate) habitat: Option<RegionEcologyMap>,
    pub(crate) life: Option<ChunkEcologyMap>,
    pub texture_dirty: bool,
    pub(crate) selected_info: Option<String>,
    pub(crate) selected_hit: Option<SettlementHit>,
    /// Land cell selected on Chunk LOD; enables the Spawn button.
    pub(crate) pending_spawn: Option<(f32, f32)>,
    /// Confirmed player start (fixed); enables gameplay after Spawn.
    pub(crate) player_spawn: Option<(f32, f32)>,
    /// Current marker position.
    pub(crate) player_pos: Option<(f32, f32)>,
    /// Remaining walk waypoints (first = current leg target). Prefer road network.
    pub(crate) move_path: Vec<(f32, f32)>,
    terrain: Option<TerrainSampler>,
    known_places: HashSet<(i32, i32)>,
    known_chunks: HashSet<(RegionId, ChunkId)>,
    pub(crate) loading: bool,
    request_id: u64,
    lod_cache: LodCache,
    pub(crate) game: GameLoop,
    last_frame: Instant,
    pub(crate) perf: PerfStats,
    tx: Sender<GenRequest>,
    rx: Receiver<GenResult>,
}

impl Default for MapSession {
    fn default() -> Self {
        Self::new()
    }
}

impl MapSession {
    pub fn new() -> Self {
        let (tx_req, rx_req) = mpsc::channel::<GenRequest>();
        let (tx_res, rx_res) = mpsc::channel::<GenResult>();

        thread::spawn(move || {
            while let Ok(req) = rx_req.recv() {
                let _ = tx_res.send(generate_job(req));
            }
        });

        let mut app = Self {
            seed: 6,
            draft_seed: "6".to_string(),
            seed_error: None,
            params: TerrainGenParams::default(),
            draft_params: TerrainGenParams::default(),
            show_seed_info: false,
            lod: LodLevel::Global,
            grid: None,
            settlements: None,
            npcs: None,
            habitat: None,
            life: None,
            texture_dirty: false,
            selected_info: None,
            selected_hit: None,
            pending_spawn: None,
            player_spawn: None,
            player_pos: None,
            move_path: Vec::new(),
            terrain: None,
            known_places: HashSet::new(),
            known_chunks: HashSet::new(),
            loading: false,
            request_id: 0,
            lod_cache: LodCache::default(),
            game: GameLoop::new(),
            last_frame: Instant::now(),
            perf: PerfStats::new(),
            tx: tx_req,
            rx: rx_res,
        };
        app.queue_generate(true);
        app
    }

    pub(crate) fn config(&self) -> WorldConfig {
        let mut config = WorldConfig::default();
        config.seed = self.seed;
        self.params.apply_to(&mut config);
        config
    }

    fn current_view(&self) -> Option<LodView> {
        Some(LodView {
            grid: self.grid.clone()?,
            habitat: self.habitat.clone(),
            life: self.life.clone(),
        })
    }

    fn apply_view(&mut self, lod: LodLevel, view: LodView) {
        self.lod = lod;
        self.grid = Some(view.grid);
        self.habitat = view.habitat;
        self.life = view.life;
        self.loading = false;
        self.selected_info = None;
        self.selected_hit = None;
        self.texture_dirty = true;
    }

    /// Zoom / back: reuse cached LOD when possible; otherwise generate.
    pub(crate) fn set_lod(&mut self, lod: LodLevel) {
        if let Some(view) = self.current_view() {
            self.lod_cache.store(self.lod, view);
        }
        if let Some(view) = self.lod_cache.get(lod).cloned() {
            self.apply_view(lod, view);
            return;
        }
        self.lod = lod;
        self.queue_generate(false);
    }

    pub(crate) fn queue_generate(&mut self, refresh_settlements: bool) {
        if refresh_settlements {
            self.lod_cache.clear();
            self.pending_spawn = None;
            self.player_spawn = None;
            self.player_pos = None;
            self.move_path.clear();
            self.terrain = None;
            self.known_places.clear();
            self.known_chunks.clear();
        }
        self.request_id += 1;
        self.loading = true;
        self.selected_info = None;
        self.selected_hit = None;
        let (settlements, npcs) = if refresh_settlements {
            (None, None)
        } else {
            (self.settlements.clone(), self.npcs.clone())
        };
        let _ = self.tx.send(GenRequest {
            id: self.request_id,
            seed: self.seed,
            params: self.params,
            lod: self.lod,
            settlements,
            npcs,
        });
    }

    pub(crate) fn apply_seed_and_params(&mut self) {
        match self.draft_seed.trim().parse::<u64>() {
            Ok(seed) => {
                let seed_changed = seed != self.seed;
                self.seed = seed;
                self.params = self.draft_params;
                self.seed_error = None;
                self.lod = LodLevel::Global;
                if seed_changed {
                    self.game.reset_time();
                }
                self.queue_generate(true);
            }
            Err(_) => {
                self.seed_error = Some("Seed musi być nieujemną liczbą całkowitą.".to_string());
            }
        }
    }

    pub(crate) fn random_seed(&mut self) {
        let seed = rand_u32() as u64;
        self.draft_seed = seed.to_string();
        self.seed = seed;
        self.seed_error = None;
        self.lod = LodLevel::Global;
        self.game.reset_time();
        self.queue_generate(true);
    }

    pub(crate) fn go_back(&mut self) {
        let next = match self.lod {
            LodLevel::Chunk { region, .. } => LodLevel::Region(region),
            LodLevel::Region(_) => LodLevel::Global,
            LodLevel::Global => return,
        };
        self.set_lod(next);
    }

    pub(crate) fn view_settlements(&self) -> Vec<Settlement> {
        let Some(map) = &self.settlements else {
            return Vec::new();
        };
        match self.lod {
            LodLevel::Global => map
                .settlements
                .iter()
                .filter(|s| matches!(s.kind, SettlementKind::City | SettlementKind::Town))
                .cloned()
                .collect(),
            _ => map.settlements.clone(),
        }
    }

    pub(crate) fn roads(&self) -> &[Road] {
        self.settlements
            .as_ref()
            .map(|m| m.roads.as_slice())
            .unwrap_or(&[])
    }

    pub(crate) fn ensure_terrain(&mut self) {
        if self.terrain.is_none() {
            self.terrain = Some(TerrainSampler::new(self.config()));
        }
    }

    pub(crate) fn road_detail(&self) -> RoadDetail {
        match self.lod {
            LodLevel::Global => RoadDetail::Main,
            LodLevel::Region(_) => RoadDetail::Region,
            LodLevel::Chunk { .. } => RoadDetail::Close,
        }
    }

    pub(crate) fn settlement_style(&self) -> SettlementDrawStyle {
        match self.lod {
            LodLevel::Global => SettlementDrawStyle::Marker,
            _ => SettlementDrawStyle::Plan,
        }
    }

    pub(crate) fn in_vision(&self, wx: f32, wy: f32) -> bool {
        let Some(pos) = self.player_pos else {
            return true;
        };
        let dx = wx - pos.0;
        let dy = wy - pos.1;
        dx * dx + dy * dy <= VISION_RADIUS * VISION_RADIUS
    }

    /// Visible on the map: vision circle or an explored chunk.
    pub(crate) fn world_revealed(&self, wx: f32, wy: f32, config: &WorldConfig) -> bool {
        if self.player_spawn.is_none() {
            return true;
        }
        if self.in_vision(wx, wy) {
            return true;
        }
        world_to_chunk(wx, wy, config)
            .map(|(r, c)| self.known_chunks.contains(&(r, c)))
            .unwrap_or(false)
    }

    pub(crate) fn explore_player_chunk(&mut self, config: &WorldConfig) -> bool {
        let Some(pos) = self.player_pos else {
            return false;
        };
        let Some(ch) = world_to_chunk(pos.0, pos.1, config) else {
            return false;
        };
        self.known_chunks.insert(ch)
    }

    pub(crate) fn known_bounds_list(&self, config: &WorldConfig) -> Vec<WorldBounds> {
        self.known_chunks
            .iter()
            .map(|&(region, chunk)| LodLevel::Chunk { region, chunk }.bounds(config))
            .collect()
    }

    pub(crate) fn place_key(x: f32, y: f32) -> (i32, i32) {
        (x.floor() as i32, y.floor() as i32)
    }

    pub(crate) fn is_known_place(&self, x: f32, y: f32) -> bool {
        self.known_places.contains(&Self::place_key(x, y))
    }

    /// Mark settlements the player is currently standing at as visited.
    pub(crate) fn update_visited_places(&mut self) -> bool {
        let Some(pos) = self.player_pos else {
            return false;
        };
        let Some(map) = &self.settlements else {
            return false;
        };
        let mut changed = false;
        for s in &map.settlements {
            let r = s.radius.max(AT_PLACE_EPS);
            let dx = s.x - pos.0;
            let dy = s.y - pos.1;
            if dx * dx + dy * dy <= r * r {
                changed |= self.known_places.insert(Self::place_key(s.x, s.y));
            }
        }
        changed
    }

    pub(crate) fn clamp_to_world(&self, wx: f32, wy: f32, config: &WorldConfig) -> (f32, f32) {
        let max = (config.world_width as f32 - 0.05).max(0.05);
        (wx.clamp(0.05, max), wy.clamp(0.05, max))
    }

    pub(crate) fn confirm_spawn(&mut self) {
        let Some(pos) = self.pending_spawn else {
            return;
        };
        self.player_spawn = Some(pos);
        self.player_pos = Some(pos);
        self.move_path.clear();
        self.pending_spawn = None;
        self.known_places.clear();
        self.known_chunks.clear();
        self.game.reset_time();
        let config = self.config();
        self.explore_player_chunk(&config);
        self.update_visited_places();
        self.selected_info = Some(format!(
            "Start gracza: ({:.0}, {:.0}) — kliknij ląd, by iść",
            pos.0, pos.1
        ));
        self.texture_dirty = true;
    }

    pub(crate) fn cancel_pending_spawn(&mut self) {
        self.pending_spawn = None;
        self.selected_info = None;
        self.texture_dirty = true;
    }

    pub(crate) fn show_place_list(&self) -> bool {
        self.player_spawn.is_none()
            && self.pending_spawn.is_none()
            && !self.loading
            && self.grid.is_some()
            && matches!(self.lod, LodLevel::Region(_) | LodLevel::Chunk { .. })
    }

    pub(crate) fn places_in_view(&self) -> Vec<PlaceRow> {
        let Some(map) = &self.settlements else {
            return Vec::new();
        };
        let bounds = self.lod.bounds(&self.config());
        let mut rows: Vec<PlaceRow> = map
            .settlements
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.x >= bounds.x0
                    && s.y >= bounds.y0
                    && s.x < bounds.x0 + bounds.span
                    && s.y < bounds.y0 + bounds.span
            })
            .map(|(index, s)| PlaceRow {
                index,
                x: s.x,
                y: s.y,
                name: s.name.clone(),
                kind: settlement_info(s.kind).label.to_string(),
                kind_rank: match s.kind {
                    SettlementKind::City => 0,
                    SettlementKind::Town => 1,
                    SettlementKind::Village => 2,
                    SettlementKind::Hamlet => 3,
                },
                population: s.population,
                walled: s.walled,
            })
            .collect();
        rows.sort_by(|a, b| {
            a.kind_rank
                .cmp(&b.kind_rank)
                .then(b.population.cmp(&a.population))
                .then(a.name.cmp(&b.name))
        });
        rows
    }

    pub(crate) fn set_walk_target(&mut self, wx: f32, wy: f32) {
        let config = self.config();
        let target = self.clamp_to_world(wx, wy, &config);
        let Some(pos) = self.player_pos else {
            return;
        };
        if player_at(pos, target) {
            self.selected_info = Some(format!(
                "Już tu jesteś ({:.0}, {:.0})",
                target.0, target.1
            ));
            return;
        }
        self.ensure_terrain();
        let roads = self
            .settlements
            .as_ref()
            .map(|m| m.roads.as_slice())
            .unwrap_or(&[]);
        let (raw_path, via_road) = if let Some(route) = road_route(pos, target, roads) {
            (route, true)
        } else {
            (vec![target], false)
        };
        let path = {
            let sampler = self.terrain.as_ref().expect("terrain after ensure");
            prepare_walk_path(sampler, roads, pos, &raw_path)
        };
        if path.is_empty() {
            self.move_path.clear();
            self.selected_info =
                Some("Przejście zablokowane (woda lub góry).".to_string());
            self.texture_dirty = true;
            return;
        }
        let dest = *path.last().unwrap();
        let full = player_at(dest, target);
        self.move_path = path;
        self.selected_info = Some(if via_road && full {
            format!("Po drodze do ({:.0}, {:.0})", target.0, target.1)
        } else if full {
            format!("W drodze do ({:.0}, {:.0})", target.0, target.1)
        } else {
            format!(
                "Przeszkoda — idę do ({:.0}, {:.0})",
                dest.0, dest.1
            )
        });
        self.texture_dirty = true;
    }

    pub(crate) fn focus_player(&mut self) {
        let Some(pos) = self.player_pos else {
            return;
        };
        let config = self.config();
        let Some((region, chunk)) = world_to_chunk(pos.0, pos.1, &config) else {
            return;
        };
        self.selected_hit = None;
        self.selected_info = Some(format!("Gracz: ({:.1}, {:.1})", pos.0, pos.1));
        self.set_lod(LodLevel::Chunk { region, chunk });
    }

    pub(crate) fn walk_toward_cardinal(&mut self, dir: Cardinal) {
        let Some(pos) = self.player_pos else {
            return;
        };
        let config = self.config();
        if let Some(target) = road_target_in_direction(pos, dir, self.roads()) {
            self.set_walk_target(target.0, target.1);
            return;
        }
        let (dx, dy) = dir.delta();
        let step = VISION_RADIUS * 1.5;
        let target = self.clamp_to_world(pos.0 + dx * step, pos.1 + dy * step, &config);
        self.set_walk_target(target.0, target.1);
    }

    pub(crate) fn go_to_destination(
        &mut self,
        wx: f32,
        wy: f32,
        region: RegionId,
        chunk: ChunkId,
    ) {
        if !self.is_known_place(wx, wy) {
            self.selected_info = Some("Nieznane miejsce — najpierw je odwiedź.".to_string());
            return;
        }
        let config = self.config();
        let player_chunk = self
            .player_pos
            .and_then(|(x, y)| world_to_chunk(x, y, &config));
        if player_chunk != Some((region, chunk)) {
            self.set_lod(LodLevel::Chunk { region, chunk });
        }
        self.set_walk_target(wx, wy);
    }

    /// Move on foot using in-game seconds; fog follows; camera follows chunk changes.
    pub(crate) fn advance_movement(&mut self, game_secs: f64) {
        if game_secs <= 0.0 || !game_secs.is_finite() {
            return;
        }
        let Some(pos) = self.player_pos else {
            return;
        };
        let Some(target) = self.move_path.first().copied() else {
            return;
        };
        let config = self.config();
        self.ensure_terrain();
        let dx = target.0 - pos.0;
        let dy = target.1 - pos.1;
        let along_road = {
            let roads = self
                .settlements
                .as_ref()
                .map(|m| m.roads.as_slice())
                .unwrap_or(&[]);
            traveling_along_road(roads, pos.0, pos.1, dx, dy, ROAD_TRAVEL_EPS)
        };
        let speed_wu = if along_road {
            WALK_ROAD_WU_PER_GAME_HOUR
        } else {
            WALK_OFFROAD_WU_PER_GAME_HOUR
        };
        let speed = speed_wu / 3600.0;
        let step = speed * game_secs as f32;
        let dist = (dx * dx + dy * dy).sqrt();
        let reached = dist <= step || dist < 1e-4;
        let (nx, ny) = if reached {
            target
        } else {
            let t = step / dist;
            (pos.0 + dx * t, pos.1 + dy * t)
        };
        let next = self.clamp_to_world(nx, ny, &config);
        let blocked = {
            let roads = self
                .settlements
                .as_ref()
                .map(|m| m.roads.as_slice())
                .unwrap_or(&[]);
            let sampler = self.terrain.as_ref().expect("terrain after ensure");
            !point_traversable(sampler, roads, next.0, next.1)
        };
        if blocked {
            self.move_path.clear();
            self.selected_info =
                Some("Przejście zablokowane (woda lub góry).".to_string());
            self.texture_dirty = true;
            return;
        }
        let prev_chunk = world_to_chunk(pos.0, pos.1, &config);
        self.player_pos = Some(next);
        if reached {
            self.move_path.remove(0);
            if self.move_path.is_empty() {
                self.selected_info =
                    Some(format!("Na miejscu ({:.0}, {:.0})", target.0, target.1));
            }
        }
        let config = self.config();
        let explored = self.explore_player_chunk(&config);
        let visited = self.update_visited_places();
        if visited {
            self.selected_info = Some("Odwiedzono osadę — dodano do znanych miejsc.".to_string());
        } else if explored && !reached {
            self.selected_info = Some("Odkryto nowy obszar.".to_string());
        }
        self.maybe_follow_player_chunk(prev_chunk);
        self.texture_dirty = true;
    }

    /// When the player crosses into another chunk while in Chunk LOD, follow them.
    pub(crate) fn maybe_follow_player_chunk(
        &mut self,
        prev_chunk: Option<(RegionId, ChunkId)>,
    ) {
        let Some(pos) = self.player_pos else {
            return;
        };
        let LodLevel::Chunk { .. } = self.lod else {
            return;
        };
        let config = self.config();
        let Some(cur) = world_to_chunk(pos.0, pos.1, &config) else {
            return;
        };
        if prev_chunk == Some(cur) {
            return;
        }
        let status = self.selected_info.clone();
        let hit = self.selected_hit;
        self.set_lod(LodLevel::Chunk {
            region: cur.0,
            chunk: cur.1,
        });
        self.selected_info = status;
        self.selected_hit = hit;
    }

    pub(crate) fn poll_results(&mut self) {
        let mut got = None;
        while let Ok(result) = self.rx.try_recv() {
            if result.id == self.request_id {
                got = Some(result);
            }
        }
        if let Some(result) = got {
            let view = LodView {
                grid: result.grid,
                habitat: result.habitat,
                life: result.life,
            };
            self.lod_cache.store(result.lod, view.clone());
            self.grid = Some(view.grid);
            self.settlements = Some(result.settlements);
            self.npcs = Some(result.npcs);
            self.habitat = view.habitat;
            self.life = view.life;
            self.lod = result.lod;
            self.seed = result.seed;
            self.loading = false;
            self.selected_info = None;
            self.selected_hit = None;
            self.texture_dirty = true;
        }
    }

    pub(crate) fn tick_simulation(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;
        // Clock / day cycle runs only after spawn (and not while generating).
        if !self.loading && self.player_spawn.is_some() {
            let game_secs = self.game.tick(dt);
            self.advance_movement(game_secs);
        }
    }

    pub(crate) fn known_travel_dests(&self) -> Vec<TravelDest> {
        known_place_dests(&self.settlements, &self.known_places, &self.config())
    }
}

pub(crate) struct TravelDest {
    pub(crate) label: String,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) region: RegionId,
    pub(crate) chunk: ChunkId,
}

pub(crate) fn known_place_dests(
    map: &Option<SettlementMap>,
    known: &HashSet<(i32, i32)>,
    config: &WorldConfig,
) -> Vec<TravelDest> {
    let Some(map) = map else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for s in &map.settlements {
        let key = (s.x.floor() as i32, s.y.floor() as i32);
        if !known.contains(&key) {
            continue;
        }
        let Some((region, chunk)) = world_to_chunk(s.x, s.y, config) else {
            continue;
        };
        let kind = settlement_info(s.kind).label;
        out.push(TravelDest {
            label: format!("{} · {}", s.name, kind),
            x: s.x,
            y: s.y,
            region,
            chunk,
        });
    }
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out
}

/// Best road point ahead in a cardinal direction (prefers highways).
/// Only points inside the player's vision count as known roads.
fn road_target_in_direction(
    pos: (f32, f32),
    dir: Cardinal,
    roads: &[Road],
) -> Option<(f32, f32)> {
    let (dx, dy) = dir.delta();
    let mut best: Option<(f32, f32, f32)> = None; // score (lower better), x, y
    for road in roads {
        let kind_penalty = match road.kind {
            RoadKind::Highway => 0.0,
            RoadKind::Secondary => 40.0,
            RoadKind::Local => 90.0,
        };
        for p in &road.points {
            let vx = p.x - pos.0;
            let vy = p.y - pos.1;
            let dist = (vx * vx + vy * vy).sqrt();
            if dist < 0.6 || dist > VISION_RADIUS {
                continue;
            }
            let along = vx * dx + vy * dy;
            if along < 0.5 {
                continue;
            }
            let lateral = (vx * dy - vy * dx).abs();
            if lateral > along * 0.9 {
                continue;
            }
            let score = dist + lateral * 1.5 + kind_penalty;
            if best.map(|(s, _, _)| score < s).unwrap_or(true) {
                best = Some((score, p.x, p.y));
            }
        }
    }
    best.map(|(_, x, y)| (x, y))
}

pub(crate) fn player_at(pos: (f32, f32), target: (f32, f32)) -> bool {
    let dx = pos.0 - target.0;
    let dy = pos.1 - target.1;
    dx * dx + dy * dy <= AT_PLACE_EPS * AT_PLACE_EPS
}

fn world_to_chunk(wx: f32, wy: f32, config: &WorldConfig) -> Option<(RegionId, ChunkId)> {
    if wx < 0.0 || wy < 0.0 {
        return None;
    }
    let w = config.world_width as f32;
    if wx >= w || wy >= w {
        return None;
    }
    let rs = config.region_size;
    let cs = config.chunk_size;
    if rs == 0 || cs == 0 {
        return None;
    }
    let ix = wx.floor() as u32;
    let iy = wy.floor() as u32;
    let rx = ix / rs;
    let ry = iy / rs;
    let cx = (ix % rs) / cs;
    let cy = (iy % rs) / cs;
    Some((RegionId { rx, ry }, ChunkId { cx, cy }))
}

fn terrain_walkable(sampler: &TerrainSampler, x: f32, y: f32) -> bool {
    let cell = sampler.cell_at(
        x as f64,
        y as f64,
        crate::world::config::LodLevel::Macro,
    );
    if is_water_biome(cell.elevation) || cell.elevation >= HIGH_M {
        return false;
    }
    !matches!(
        cell.biome,
        TileType::Water | TileType::DeepWater | TileType::Mountain | TileType::Snow
    )
}

fn dist2(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

fn dist_point_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let len2 = abx * abx + aby * aby;
    if len2 < 1e-8 {
        return dist2(px, py, ax, ay).sqrt();
    }
    let t = ((px - ax) * abx + (py - ay) * aby) / len2;
    let t = t.clamp(0.0, 1.0);
    let qx = ax + abx * t;
    let qy = ay + aby * t;
    dist2(px, py, qx, qy).sqrt()
}

fn near_road(roads: &[Road], x: f32, y: f32, eps: f32) -> bool {
    let eps2 = eps * eps;
    for road in roads {
        for p in &road.points {
            if dist2(x, y, p.x, p.y) <= eps2 {
                return true;
            }
        }
        for i in 0..road.points.len().saturating_sub(1) {
            let a = &road.points[i];
            let b = &road.points[i + 1];
            if dist_point_to_segment(x, y, a.x, a.y, b.x, b.y) <= eps {
                return true;
            }
        }
    }
    false
}

/// True only when near a road segment and moving roughly along it (not across it).
fn traveling_along_road(
    roads: &[Road],
    x: f32,
    y: f32,
    move_dx: f32,
    move_dy: f32,
    eps: f32,
) -> bool {
    let move_len = (move_dx * move_dx + move_dy * move_dy).sqrt();
    if move_len < 1e-4 {
        return false;
    }
    let mdx = move_dx / move_len;
    let mdy = move_dy / move_len;
    for road in roads {
        for i in 0..road.points.len().saturating_sub(1) {
            let a = &road.points[i];
            let b = &road.points[i + 1];
            if dist_point_to_segment(x, y, a.x, a.y, b.x, b.y) > eps {
                continue;
            }
            let sdx = b.x - a.x;
            let sdy = b.y - a.y;
            let slen = (sdx * sdx + sdy * sdy).sqrt();
            if slen < 1e-4 {
                continue;
            }
            let alignment = ((sdx / slen) * mdx + (sdy / slen) * mdy).abs();
            if alignment >= ROAD_ALIGN_MIN {
                return true;
            }
        }
    }
    false
}

/// Walkable open terrain, or on an existing road ribbon (bridges included).
fn point_traversable(sampler: &TerrainSampler, roads: &[Road], x: f32, y: f32) -> bool {
    terrain_walkable(sampler, x, y) || near_road(roads, x, y, ROAD_TRAVEL_EPS)
}

/// Furthest point on from→to that stays traversable; `true` if the full segment is clear.
fn farthest_traversable(
    sampler: &TerrainSampler,
    roads: &[Road],
    from: (f32, f32),
    to: (f32, f32),
) -> ((f32, f32), bool) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 1e-4 {
        return (from, point_traversable(sampler, roads, from.0, from.1));
    }
    let n = ((dist / 0.5).ceil() as usize).clamp(2, 200);
    let mut last = from;
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let x = from.0 + dx * t;
        let y = from.1 + dy * t;
        if !point_traversable(sampler, roads, x, y) {
            return (last, false);
        }
        last = (x, y);
    }
    (to, true)
}

/// Clamp a waypoint chain so each leg stays traversable; stop at the first block.
fn prepare_walk_path(
    sampler: &TerrainSampler,
    roads: &[Road],
    from: (f32, f32),
    waypoints: &[(f32, f32)],
) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    let mut cur = from;
    for &wp in waypoints {
        if player_at(cur, wp) {
            continue;
        }
        let (dest, clear) = farthest_traversable(sampler, roads, cur, wp);
        if player_at(cur, dest) {
            break;
        }
        out.push(dest);
        if !clear {
            break;
        }
        cur = dest;
    }
    out
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct RouteNode {
    cost: u32,
    idx: usize,
}

impl Ord for RouteNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.idx.cmp(&other.idx))
    }
}

impl PartialOrd for RouteNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn nearest_road_node(nodes: &[(f32, f32)], p: (f32, f32)) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;
    for (i, &(x, y)) in nodes.iter().enumerate() {
        let d = dist2(p.0, p.1, x, y).sqrt();
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((i, d));
        }
    }
    best
}

/// Shortest path along road polylines from near `from` to near `to`.
/// Returns waypoints on the network, ending at `to` when reachable.
fn road_route(from: (f32, f32), to: (f32, f32), roads: &[Road]) -> Option<Vec<(f32, f32)>> {
    let mut nodes: Vec<(f32, f32)> = Vec::new();
    for road in roads {
        for p in &road.points {
            nodes.push((p.x, p.y));
        }
    }
    if nodes.len() < 2 {
        return None;
    }

    let mut edges: Vec<Vec<(usize, u32)>> = vec![Vec::new(); nodes.len()];
    let mut offset = 0usize;
    for road in roads {
        let n = road.points.len();
        for i in 0..n.saturating_sub(1) {
            let a = offset + i;
            let b = offset + i + 1;
            let d = (dist2(nodes[a].0, nodes[a].1, nodes[b].0, nodes[b].1).sqrt() * 100.0)
                .round() as u32;
            let d = d.max(1);
            edges[a].push((b, d));
            edges[b].push((a, d));
        }
        offset += n;
    }
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let d = dist2(nodes[i].0, nodes[i].1, nodes[j].0, nodes[j].1).sqrt();
            if d > 0.05 && d <= ROAD_NODE_LINK_EPS {
                let c = (d * 100.0).round() as u32;
                let c = c.max(1);
                edges[i].push((j, c));
                edges[j].push((i, c));
            }
        }
    }

    let (start, start_d) = nearest_road_node(&nodes, from)?;
    let (goal, goal_d) = nearest_road_node(&nodes, to)?;
    if start_d > ROAD_SNAP_EPS || goal_d > ROAD_SNAP_EPS {
        return None;
    }

    let n = nodes.len();
    let mut dist = vec![u32::MAX; n];
    let mut prev = vec![None; n];
    let mut heap = BinaryHeap::new();
    dist[start] = 0;
    heap.push(RouteNode {
        cost: 0,
        idx: start,
    });
    while let Some(RouteNode { cost, idx }) = heap.pop() {
        if cost != dist[idx] {
            continue;
        }
        if idx == goal {
            break;
        }
        for &(next, w) in &edges[idx] {
            let next_cost = cost.saturating_add(w);
            if next_cost < dist[next] {
                dist[next] = next_cost;
                prev[next] = Some(idx);
                heap.push(RouteNode {
                    cost: next_cost,
                    idx: next,
                });
            }
        }
    }
    if dist[goal] == u32::MAX {
        return None;
    }

    let mut chain = Vec::new();
    let mut cur = Some(goal);
    while let Some(i) = cur {
        chain.push(nodes[i]);
        if i == start {
            break;
        }
        cur = prev[i];
    }
    chain.reverse();
    if chain.is_empty() {
        return None;
    }

    // Skip the entry node when the player is already there.
    if player_at(from, chain[0]) {
        chain.remove(0);
    }
    if chain
        .last()
        .map(|p| !player_at(*p, to))
        .unwrap_or(true)
    {
        chain.push(to);
    }
    if chain.is_empty() {
        None
    } else {
        Some(chain)
    }
}

fn generate_job(req: GenRequest) -> GenResult {
    let mut config = WorldConfig::default();
    config.seed = req.seed;
    req.params.apply_to(&mut config);

    let sampler = TerrainSampler::new(config.clone());

    let settlements = req
        .settlements
        .unwrap_or_else(|| generate_settlements_sampled(&sampler));
    let npcs = req
        .npcs
        .unwrap_or_else(|| generate_npcs(req.seed, &settlements.settlements));

    let (grid, habitat, life) = match req.lod {
        LodLevel::Global => (generate_global_grid_sampled(&sampler), None, None),
        LodLevel::Region(region) => (
            generate_region_grid_sampled(&sampler, region),
            Some(generate_region_ecology_sampled(&sampler, region)),
            None,
        ),
        LodLevel::Chunk { region, chunk } => {
            let x0 = region.rx as f32 * config.region_size as f32
                + chunk.cx as f32 * config.chunk_size as f32;
            let y0 = region.ry as f32 * config.region_size as f32
                + chunk.cy as f32 * config.chunk_size as f32;
            (
                generate_view_grid_sampled(
                    &sampler,
                    region,
                    x0,
                    y0,
                    config.chunk_size as f32,
                ),
                None,
                Some(generate_chunk_ecology_sampled(&sampler, region, chunk)),
            )
        }
    };

    GenResult {
        id: req.id,
        seed: req.seed,
        lod: req.lod,
        grid,
        settlements,
        npcs,
        habitat,
        life,
    }
}


pub(crate) fn biome_label(biome: TileType) -> &'static str {
    for (tile, info) in BIOMES {
        if *tile == biome {
            return info.label;
        }
    }
    "?"
}

pub(crate) fn is_land_biome(biome: TileType) -> bool {
    !matches!(biome, TileType::Water | TileType::DeepWater)
}

/// Grid cell under the click → world center of that cell + biome.
pub(crate) fn cell_world_at_pixel(
    grid: &TerrainGrid,
    bounds: &WorldBounds,
    px: f32,
    py: f32,
    canvas_width: f32,
    canvas_height: f32,
) -> Option<(f32, f32, TileType)> {
    if grid.width == 0 || grid.height == 0 || grid.cells.is_empty() {
        return None;
    }
    let gx = ((px / canvas_width) * grid.width as f32)
        .floor()
        .clamp(0.0, (grid.width - 1) as f32) as usize;
    let gy = ((py / canvas_height) * grid.height as f32)
        .floor()
        .clamp(0.0, (grid.height - 1) as f32) as usize;
    let biome = grid.cells.get(gy * grid.width as usize + gx)?.biome;
    let wx = bounds.x0 + ((gx as f32 + 0.5) / grid.width as f32) * bounds.span;
    let wy = bounds.y0 + ((gy as f32 + 0.5) / grid.height as f32) * bounds.span;
    Some((wx, wy, biome))
}


pub(crate) fn format_pop(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    ((t ^ (t >> 33)).wrapping_mul(0xff51afd7ed558ccd) >> 32) as u32
}
