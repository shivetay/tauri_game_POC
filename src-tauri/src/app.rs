use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use eframe::egui::{self, Color32, Sense, TextureHandle, TextureOptions, Vec2};
use egui::{pos2, Align2, FontId, RichText};

use crate::colors::BIOMES;
use crate::game_loop::GameLoop;
use crate::game_time::SPEED_MULTIPLIERS;
use crate::perf::PerfStats;
use crate::ecology_draw::{
    life_area_summary, FAUNA_BIRD, FAUNA_LARGE_MAMMAL, FAUNA_SMALL, VEGETATION_RGB,
};
use crate::render::{biome_at_pixel, compose_map_image, ComposeInput};
use crate::settlements_draw::{
    district_info, format_settlement_label, hit_settlement, road_info, settlement_info, RoadDetail,
    SettlementDrawStyle, SettlementHit, WorldBounds, DISTRICT_KIND_ORDER, ROAD_KIND_ORDER,
    SETTLEMENT_KIND_ORDER,
};
use crate::world::config::{TerrainGenParams, WorldConfig};
use crate::world::ecology::{
    generate_chunk_ecology_sampled, generate_region_ecology_sampled, ChunkEcologyMap,
    RegionEcologyMap,
};
use crate::world::grid::{
    generate_global_grid_sampled, generate_region_grid_sampled, generate_view_grid_sampled,
};
use crate::world::sampler::TerrainSampler;
use crate::world::settlement::{
    generate_sampled as generate_settlements_sampled, Road, Settlement, SettlementKind,
    SettlementMap,
};
use crate::world::types::{ChunkId, RegionId, TerrainGrid, TileType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LodLevel {
    Global,
    Region(RegionId),
    Chunk { region: RegionId, chunk: ChunkId },
}

impl LodLevel {
    fn bounds(self, config: &WorldConfig) -> WorldBounds {
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

    fn idle_label(self) -> String {
        match self {
            Self::Global => "Mapa świata — kliknij region · kliknij miasto po info".to_string(),
            Self::Region(r) => {
                format!("Region ({}, {}) — kliknij obszar · kliknij miasto po info", r.rx, r.ry)
            }
            Self::Chunk { chunk, .. } => {
                format!(
                    "Obszar ({}, {}) — kliknij biom/miasto lub gatunki",
                    chunk.cx, chunk.cy
                )
            }
        }
    }

    fn back_label(self) -> Option<String> {
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
}

struct GenResult {
    id: u64,
    seed: u64,
    lod: LodLevel,
    grid: TerrainGrid,
    settlements: SettlementMap,
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

pub struct MapApp {
    seed: u64,
    draft_seed: String,
    seed_error: Option<String>,
    params: TerrainGenParams,
    draft_params: TerrainGenParams,
    show_seed_info: bool,
    lod: LodLevel,
    grid: Option<TerrainGrid>,
    settlements: Option<SettlementMap>,
    habitat: Option<RegionEcologyMap>,
    life: Option<ChunkEcologyMap>,
    texture: Option<TextureHandle>,
    selected_info: Option<String>,
    selected_hit: Option<SettlementHit>,
    loading: bool,
    request_id: u64,
    lod_cache: LodCache,
    game: GameLoop,
    last_frame: Instant,
    perf: PerfStats,
    tx: Sender<GenRequest>,
    rx: Receiver<GenResult>,
}

impl Default for MapApp {
    fn default() -> Self {
        Self::new()
    }
}

impl MapApp {
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
            habitat: None,
            life: None,
            texture: None,
            selected_info: None,
            selected_hit: None,
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

    fn config(&self) -> WorldConfig {
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

    fn apply_view(&mut self, lod: LodLevel, view: LodView, ctx: &egui::Context) {
        self.lod = lod;
        self.grid = Some(view.grid);
        self.habitat = view.habitat;
        self.life = view.life;
        self.loading = false;
        self.selected_info = None;
        self.selected_hit = None;
        self.rebuild_texture(ctx);
    }

    /// Zoom / back: reuse cached LOD when possible; otherwise generate.
    fn set_lod(&mut self, lod: LodLevel, ctx: &egui::Context) {
        if let Some(view) = self.current_view() {
            self.lod_cache.store(self.lod, view);
        }
        if let Some(view) = self.lod_cache.get(lod).cloned() {
            self.apply_view(lod, view, ctx);
            return;
        }
        self.lod = lod;
        self.queue_generate(false);
    }

    fn queue_generate(&mut self, refresh_settlements: bool) {
        if refresh_settlements {
            self.lod_cache.clear();
        }
        self.request_id += 1;
        self.loading = true;
        self.selected_info = None;
        self.selected_hit = None;
        let settlements = if refresh_settlements {
            None
        } else {
            self.settlements.clone()
        };
        let _ = self.tx.send(GenRequest {
            id: self.request_id,
            seed: self.seed,
            params: self.params,
            lod: self.lod,
            settlements,
        });
    }

    fn apply_seed_and_params(&mut self) {
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

    fn random_seed(&mut self) {
        let seed = rand_u32() as u64;
        self.draft_seed = seed.to_string();
        self.seed = seed;
        self.seed_error = None;
        self.lod = LodLevel::Global;
        self.game.reset_time();
        self.queue_generate(true);
    }

    fn go_back(&mut self, ctx: &egui::Context) {
        let next = match self.lod {
            LodLevel::Chunk { region, .. } => LodLevel::Region(region),
            LodLevel::Region(_) => LodLevel::Global,
            LodLevel::Global => return,
        };
        self.set_lod(next, ctx);
    }

    fn view_settlements(&self) -> Vec<Settlement> {
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

    fn roads(&self) -> &[Road] {
        self.settlements
            .as_ref()
            .map(|m| m.roads.as_slice())
            .unwrap_or(&[])
    }

    fn road_detail(&self) -> RoadDetail {
        match self.lod {
            LodLevel::Global => RoadDetail::Main,
            LodLevel::Region(_) => RoadDetail::Region,
            LodLevel::Chunk { .. } => RoadDetail::Close,
        }
    }

    fn settlement_style(&self) -> SettlementDrawStyle {
        match self.lod {
            LodLevel::Global => SettlementDrawStyle::Marker,
            _ => SettlementDrawStyle::Plan,
        }
    }

    fn rebuild_texture(&mut self, ctx: &egui::Context) {
        let Some(grid) = &self.grid else {
            return;
        };
        let config = self.config();
        let chunks_per_side = (config.region_size / config.chunk_size).max(1);
        let cell_size = grid.width as f32 / chunks_per_side as f32;
        let settlements = self.view_settlements();
        let image = compose_map_image(ComposeInput {
            grid,
            cell_size,
            show_grid: !matches!(self.lod, LodLevel::Chunk { .. }),
            bounds: self.lod.bounds(&config),
            settlements: &settlements,
            roads: self.roads(),
            road_detail: self.road_detail(),
            settlement_style: self.settlement_style(),
            habitat: self.habitat.as_ref(),
            life: self.life.as_ref(),
            hovered: self.selected_hit.as_ref(),
        });
        self.texture = Some(ctx.load_texture("terrain", image, TextureOptions::NEAREST));
    }

    fn poll_results(&mut self, ctx: &egui::Context) {
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
            self.habitat = view.habitat;
            self.life = view.life;
            self.lod = result.lod;
            self.seed = result.seed;
            self.loading = false;
            self.selected_info = None;
            self.selected_hit = None;
            self.rebuild_texture(ctx);
        }
    }

    fn tick_simulation(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;
        // Clock starts only once the map is ready (not while generating).
        if !self.loading {
            self.game.tick(dt);
        }
    }

    fn ui_game_clock(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Czas gry").strong());
                ui.add_space(4.0);
                ui.label(
                    RichText::new(self.game.time().format_label())
                        .monospace()
                        .size(16.0),
                );
                let status = if self.game.paused() {
                    "Pauza".to_string()
                } else {
                    format!("×{:.0}", self.game.speed_mult())
                };
                ui.label(RichText::new(status).small());
            });

            ui.add_space(8.0);
            let pause_label = if self.game.paused() { "Wznów" } else { "Pauza" };
            if ui
                .add(egui::Button::new(pause_label).min_size(Vec2::new(ui.available_width(), 0.0)))
                .clicked()
            {
                self.game.toggle_pause();
            }

            ui.add_space(4.0);
            ui.label(RichText::new("Prędkość").small());
            ui.horizontal(|ui| {
                for &mult in &SPEED_MULTIPLIERS {
                    let selected =
                        !self.game.paused() && (self.game.speed_mult() - mult).abs() < f64::EPSILON;
                    if ui
                        .selectable_label(selected, format!("×{:.0}", mult))
                        .clicked()
                    {
                        self.game.set_speed(mult);
                    }
                }
            });

            ui.add_space(4.0);
            if ui
                .add(egui::Button::new("+1 h").min_size(Vec2::new(ui.available_width(), 0.0)))
                .clicked()
            {
                self.game.skip_hours(1);
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Wydajność").strong());
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("FPS  {:.0}", self.perf.fps()))
                        .monospace()
                        .size(15.0),
                );
                ui.label(
                    RichText::new(format!("CPU  {:.0}%", self.perf.cpu_percent()))
                        .monospace()
                        .size(15.0),
                );
            });
        });
    }

    fn ui_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("World Map Generator");

        ui.horizontal(|ui| {
            ui.label("Seed:");
            ui.text_edit_singleline(&mut self.draft_seed);
        });
        if let Some(err) = &self.seed_error {
            ui.colored_label(Color32::from_rgb(0xcc, 0x44, 0x44), err);
        }

        for (label, hint, value) in [
            (
                "Ukształtowanie",
                "0 = płasko, 1 = pełne pasma (góry i depresje z seeda)",
                &mut self.draft_params.orogeny_strength,
            ),
            (
                "Wilgotność",
                "0 = jednolita, 1 = pełny zasięg biomów",
                &mut self.draft_params.moisture_strength,
            ),
            (
                "Detal micro",
                "0 = gładko, 1 = pełny detal w zbliżeniu",
                &mut self.draft_params.detail_strength,
            ),
            (
                "Falistość terenu",
                "0 = równiny, 1 = pełne wzniesienia bazowe",
                &mut self.draft_params.terrain_roughness,
            ),
            (
                "Powierzchnia lądu",
                "0 = mniej lądu, 1 = więcej lądu",
                &mut self.draft_params.land_size,
            ),
            (
                "Nieregularność brzegu",
                "0 = gładkie wybrzeże, 1 = pełne zatoki i półwyspy",
                &mut self.draft_params.coast_distortion,
            ),
        ] {
            ui.add(egui::Slider::new(value, 0.0..=1.0).text(label).step_by(0.1))
                .on_hover_text(hint);
        }

        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.loading, egui::Button::new("Losuj seed"))
                .clicked()
            {
                self.random_seed();
            }
            let regen_label = if self.loading {
                "Generowanie…"
            } else {
                "Regenerate"
            };
            if ui
                .add_enabled(!self.loading, egui::Button::new(regen_label))
                .clicked()
            {
                self.apply_seed_and_params();
            }
            let info_label = if self.show_seed_info {
                "Ukryj opis"
            } else {
                "Pokaż opis"
            };
            if ui.button(info_label).clicked() {
                self.show_seed_info = !self.show_seed_info;
            }
        });

        if self.show_seed_info {
            ui.label("Seed to nieujemna liczba całkowita. Z niej generator buduje cały świat:");
            ui.label("• profil kształtu — seed % 6 (0 Radial … 5 Continents)");
            ui.label("• pasma górskie i depresje — losowe z seeda");
            ui.label("• wilgotność / detal / wybrzeże — pochodne seeda");
            ui.label("Suwaki skalują intensywność (0–1). Ten sam seed + suwaki → ten sam świat.");
        }

        if self.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Generowanie mapy…");
            });
        }

        ui.separator();
        self.ui_ecology_legend(ui);
        self.ui_settlement_legend(ui);
        self.ui_biome_legend(ui);
    }

    fn ui_ecology_legend(&self, ui: &mut egui::Ui) {
        let mode = match self.lod {
            LodLevel::Global => return,
            LodLevel::Region(_) => "region",
            LodLevel::Chunk { .. } => "chunk",
        };
        ui.label(RichText::new(if mode == "region" {
            "Siedliska (region)"
        } else {
            "Życie (obszar)"
        })
        .strong());
        swatch_row(
            ui,
            VEGETATION_RGB,
            if mode == "region" {
                "Potencjał biomu"
            } else {
                "Roślinność"
            },
        );
        swatch_row(ui, FAUNA_LARGE_MAMMAL.rgb, FAUNA_LARGE_MAMMAL.label);
        swatch_row(ui, FAUNA_BIRD.rgb, FAUNA_BIRD.label);
        swatch_row(ui, FAUNA_SMALL.rgb, FAUNA_SMALL.label);
        if mode == "chunk" {
            ui.label(RichText::new("Kliknij obszar, by zobaczyć biom i gatunki.").small());
        } else {
            ui.label(
                RichText::new("Kliknij obszar, by powiększyć · kliknij miasto po szczegóły.")
                    .small(),
            );
        }
        ui.separator();
    }

    fn ui_settlement_legend(&self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Osady (mieszkańcy)").strong());
        let counts = settlement_counts(self.settlements.as_ref());
        for kind in SETTLEMENT_KIND_ORDER {
            let info = settlement_info(kind);
            let count = match kind {
                SettlementKind::City => counts[0],
                SettlementKind::Town => counts[1],
                SettlementKind::Village => counts[2],
                SettlementKind::Hamlet => counts[3],
            };
            ui.horizontal(|ui| {
                swatch(ui, [info.fill.r, info.fill.g, info.fill.b]);
                ui.label(format!(
                    "{} · {}–{} · {}",
                    info.label,
                    format_pop(info.pop_min),
                    format_pop(info.pop_max),
                    count
                ));
            });
        }
        ui.label(RichText::new("Drogi").strong());
        for kind in ROAD_KIND_ORDER {
            let info = road_info(kind);
            ui.horizontal(|ui| {
                swatch(ui, [info.stroke.r, info.stroke.g, info.stroke.b]);
                ui.label(info.label);
            });
        }
        ui.label(RichText::new("Dzielnice").strong());
        for kind in DISTRICT_KIND_ORDER {
            let info = district_info(kind);
            ui.horizontal(|ui| {
                swatch(ui, [info.fill.r, info.fill.g, info.fill.b]);
                ui.label(info.label);
            });
        }
        ui.separator();
    }

    fn ui_biome_legend(&self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Biomy").strong());
        for (_, info) in BIOMES {
            swatch_row(ui, info.rgb, info.label);
        }
    }

    fn handle_map_click(&mut self, ctx: &egui::Context, pixel: (f32, f32), canvas: (f32, f32)) {
        if self.loading || self.grid.is_none() {
            return;
        }
        let config = self.config();
        let bounds = self.lod.bounds(&config);
        let settlements = self.view_settlements();
        let (px, py) = pixel;
        let (cw, ch) = canvas;

        if let Some(hit) = hit_settlement(
            px,
            py,
            cw as usize,
            ch as usize,
            &settlements,
            &bounds,
            self.settlement_style(),
        ) {
            let settlement = &settlements[hit.settlement_index];
            let district = hit
                .district_index
                .and_then(|di| settlement.districts.get(di));
            self.selected_info = Some(format_settlement_label(settlement, district));
            self.selected_hit = Some(hit);
            self.rebuild_texture(ctx);
            return;
        }

        let chunks_per_side = (config.region_size / config.chunk_size).max(1);
        let cell_size = cw / chunks_per_side as f32;
        let cx = ((px / cell_size).floor() as i32).clamp(0, chunks_per_side as i32 - 1) as u32;
        let cy = ((py / cell_size).floor() as i32).clamp(0, chunks_per_side as i32 - 1) as u32;

        match self.lod {
            LodLevel::Global => {
                self.set_lod(LodLevel::Region(RegionId { rx: cx, ry: cy }), ctx);
            }
            LodLevel::Region(region) => {
                self.set_lod(
                    LodLevel::Chunk {
                        region,
                        chunk: ChunkId { cx, cy },
                    },
                    ctx,
                );
            }
            LodLevel::Chunk { .. } => {
                let mut parts = Vec::new();
                if let Some(grid) = &self.grid {
                    if let Some(biome) = biome_at_pixel(grid, px, py, cw, ch) {
                        parts.push(format!("Biom: {}", biome_label(biome)));
                    }
                }
                if let Some(life) = &self.life {
                    if let Some(hint) =
                        life_area_summary(life, cw as usize, ch as usize, px, py, &bounds)
                    {
                        parts.push(hint);
                    }
                }
                self.selected_hit = None;
                self.selected_info = Some(if parts.is_empty() {
                    self.lod.idle_label()
                } else {
                    parts.join(" · ")
                });
                self.rebuild_texture(ctx);
            }
        }
    }

    fn ui_map(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            if let Some(back) = self.lod.back_label() {
                if ui
                    .add_enabled(!self.loading, egui::Button::new(back))
                    .clicked()
                {
                    self.go_back(ctx);
                }
            }
            let status = self
                .selected_info
                .clone()
                .unwrap_or_else(|| self.lod.idle_label());
            ui.label(status);
        });

        let Some(texture) = self.texture.clone() else {
            ui.centered_and_justified(|ui| ui.spinner());
            return;
        };

        let available = ui.available_size();
        let size = texture.size_vec2();
        let scale = (available.x / size.x)
            .min(available.y / size.y)
            .max(0.1);
        let display = size * scale;
        let (rect, response) = ui.allocate_exact_size(
            display,
            if matches!(self.lod, LodLevel::Chunk { .. }) {
                Sense::click()
            } else {
                Sense::click()
            },
        );

        ui.painter().image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        if self.loading {
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120));
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                match self.lod {
                    LodLevel::Global => "Generowanie mapy świata…",
                    LodLevel::Region(_) => "Generowanie regionu…",
                    LodLevel::Chunk { .. } => "Generowanie obszaru…",
                },
                FontId::proportional(18.0),
                Color32::WHITE,
            );
        }

        if response.hovered() && !matches!(self.lod, LodLevel::Chunk { .. }) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }

        if response.clicked() && !self.loading {
            if let Some(pos) = response.interact_pointer_pos() {
                let px = ((pos.x - rect.left()) / rect.width()) * size.x;
                let py = ((pos.y - rect.top()) / rect.height()) * size.y;
                self.handle_map_click(ctx, (px, py), (size.x, size.y));
            }
        }
    }
}

impl eframe::App for MapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_results(ctx);
        self.tick_simulation();
        self.perf.tick();
        // Keep repainting so the clock / FPS tick even when the map is idle.
        ctx.request_repaint();

        egui::SidePanel::left("controls")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.ui_controls(ui));
            });

        egui::SidePanel::right("game_clock")
            .resizable(false)
            .exact_width(176.0)
            .show(ctx, |ui| {
                self.ui_game_clock(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui_map(ui, ctx);
        });
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
        habitat,
        life,
    }
}

fn biome_label(biome: TileType) -> &'static str {
    for (tile, info) in BIOMES {
        if *tile == biome {
            return info.label;
        }
    }
    "?"
}

fn swatch(ui: &mut egui::Ui, rgb: [u8; 3]) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 2.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
}

fn swatch_row(ui: &mut egui::Ui, rgb: [u8; 3], label: &str) {
    ui.horizontal(|ui| {
        swatch(ui, rgb);
        ui.label(label);
    });
}

fn settlement_counts(map: Option<&SettlementMap>) -> [usize; 4] {
    let mut counts = [0usize; 4];
    let Some(map) = map else {
        return counts;
    };
    for s in &map.settlements {
        match s.kind {
            SettlementKind::City => counts[0] += 1,
            SettlementKind::Town => counts[1] += 1,
            SettlementKind::Village => counts[2] += 1,
            SettlementKind::Hamlet => counts[3] += 1,
        }
    }
    counts
}

fn format_pop(n: u32) -> String {
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
