use bevy_egui::egui::{self, pos2, Align2, Color32, FontId, RichText, Sense, Vec2};

use crate::colors::BIOMES;
use crate::ecology_draw::{
    life_area_summary, FAUNA_BIRD, FAUNA_LARGE_MAMMAL, FAUNA_SMALL, VEGETATION_RGB,
};
use crate::game_time::{GameTime, SkyBody, SPEED_MULTIPLIERS};
use crate::render::{compose_map_rgba, ComposeInput};
use crate::session::{
    biome_label, cell_world_at_pixel, format_pop, is_land_biome, player_at, MapSession,
    Cardinal, LodLevel, VISION_RADIUS,
};
use crate::settlements_draw::{
    district_info, format_settlement_label, hit_settlement, road_info, settlement_info,
    SettlementHit, DISTRICT_KIND_ORDER, ROAD_KIND_ORDER, SETTLEMENT_KIND_ORDER,
};
use crate::world::settlement::{SettlementKind, SettlementMap};
use crate::world::types::{ChunkId, RegionId};

pub struct MapApp {
    pub session: MapSession,
}

impl Default for MapApp {
    fn default() -> Self {
        Self::new()
    }
}

impl MapApp {
    pub fn new() -> Self {
        Self {
            session: MapSession::new(),
        }
    }

    pub fn mark_clean(&mut self) {
        self.session.texture_dirty = false;
    }

    pub fn poll_results(&mut self) {
        self.session.poll_results();
    }

    pub fn tick_simulation(&mut self) {
        self.session.tick_simulation();
    }

    pub fn tick_perf(&mut self) {
        self.session.perf.tick();
    }

    pub fn compose_rgba(&self) -> Option<(usize, usize, Vec<u8>)> {
        let s = &self.session;
        let grid = s.grid.as_ref()?;
        let config = s.config();
        let chunks_per_side = (config.region_size / config.chunk_size).max(1);
        let cell_size = grid.width as f32 / chunks_per_side as f32;
        let settlements = s.view_settlements();
        let explored = s.known_bounds_list(&config);
        let (fog_explored, fog_vision) = if s.player_spawn.is_some() {
            (
                Some(explored.as_slice()),
                s.player_pos.map(|(x, y)| (x, y, VISION_RADIUS)),
            )
        } else {
            (None, None)
        };
        Some(compose_map_rgba(ComposeInput {
            grid,
            cell_size,
            show_grid: !matches!(s.lod, LodLevel::Chunk { .. }),
            bounds: s.lod.bounds(&config),
            settlements: &settlements,
            roads: s.roads(),
            road_detail: s.road_detail(),
            settlement_style: s.settlement_style(),
            habitat: s.habitat.as_ref(),
            life: s.life.as_ref(),
            hovered: s.selected_hit.as_ref(),
            player_spawn: s.player_pos.or(s.player_spawn),
            fog_explored,
            fog_vision,
        }))
    }

    pub fn draw_ui(
        &mut self,
        viewport_ui: &mut egui::Ui,
        map_texture: Option<egui::TextureId>,
        map_size: egui::Vec2,
    ) {
        viewport_ui.ctx().request_repaint();

        egui::Panel::left("controls")
            .resizable(true)
            .default_size(280.0)
            .show(viewport_ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.ui_controls(ui));
            });

        let right_width = if self.session.show_place_list() { 248.0 } else { 200.0 };
        egui::Panel::right("game_clock")
            .resizable(false)
            .default_size(right_width)
            .show(viewport_ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.ui_game_clock(ui);
                });
            });

        egui::CentralPanel::default().show(viewport_ui, |ui| {
            self.ui_map(ui, map_texture, map_size);
        });
    }

    fn ui_place_list(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(RichText::new("Miejsca").strong());
        ui.label(
            RichText::new("Wybierz start. Po spawnie: kierunki i klik w ląd.")
                .small()
                .weak(),
        );
        ui.add_space(4.0);
        let rows = self.session.places_in_view();
        if rows.is_empty() {
            ui.label(RichText::new("Brak osad w tym widoku.").small());
            return;
        }
        for row in rows {
            let open = self.session
                .selected_hit
                .is_some_and(|h| h.settlement_index == row.index);
            ui.group(|ui| {
                let title = ui.selectable_label(open, RichText::new(&row.name).strong());
                if title.clicked() {
                    if open {
                        self.session.selected_hit = None;
                        self.session.selected_info = None;
                    } else {
                        self.session.selected_hit = Some(SettlementHit {
                            settlement_index: row.index,
                            district_index: None,
                        });
                        self.session.selected_info = Some(format!(
                            "{} · {} · {} mieszk.",
                            row.name,
                            row.kind,
                            format_pop(row.population)
                        ));
                    }
                    self.session.texture_dirty = true;
                }
                ui.label(format!(
                    "{} · {} mieszk.",
                    row.kind,
                    format_pop(row.population)
                ));
                if row.walled {
                    ui.label(RichText::new("Mury obronne").small());
                }
                if open {
                    let district_rows: Option<Vec<(String, String)>> = self
                        .session.settlements
                        .as_ref()
                        .and_then(|m| m.settlements.get(row.index))
                        .map(|settlement| {
                            settlement
                                .districts
                                .iter()
                                .map(|d| {
                                    (
                                        format!("{} — {}", district_info(d.kind).label, d.name),
                                        d.name.clone(),
                                    )
                                })
                                .collect()
                        });
                    if let Some(district_rows) = district_rows {
                        ui.label(RichText::new("Dzielnice").small().strong());
                        if district_rows.is_empty() {
                            ui.label(RichText::new("Brak podziału na dzielnice.").small());
                            ui.add_space(4.0);
                            ui.label(RichText::new("NPC").small().strong());
                            let roster = self
                                .session.npcs
                                .as_ref()
                                .map(|m| m.in_settlement(row.index))
                                .unwrap_or(&[]);
                            if roster.is_empty() {
                                ui.label(RichText::new("Brak NPC.").small());
                            } else {
                                for npc in roster {
                                    ui.label(
                                        RichText::new(format!("{} — {}", npc.name, npc.role))
                                            .small(),
                                    );
                                }
                            }
                        } else {
                            let selected_di = self.session.selected_hit.and_then(|h| {
                                if h.settlement_index == row.index {
                                    h.district_index
                                } else {
                                    None
                                }
                            });
                            for (di, (label, name)) in district_rows.iter().enumerate() {
                                let selected = selected_di == Some(di);
                                if ui
                                    .selectable_label(selected, RichText::new(label).small())
                                    .clicked()
                                {
                                    self.session.selected_hit = Some(SettlementHit {
                                        settlement_index: row.index,
                                        district_index: Some(di),
                                    });
                                    self.session.selected_info = Some(format!(
                                        "{} · {} · {} mieszk.",
                                        row.name,
                                        name,
                                        format_pop(row.population)
                                    ));
                                    self.session.texture_dirty = true;
                                }
                            }
                            if let Some(di) = selected_di {
                                ui.add_space(4.0);
                                ui.label(RichText::new("NPC").small().strong());
                                let roster = self
                                    .session.npcs
                                    .as_ref()
                                    .map(|m| m.in_district(row.index, di))
                                    .unwrap_or(&[]);
                                if roster.is_empty() {
                                    ui.label(RichText::new("Brak NPC.").small());
                                } else {
                                    for npc in roster {
                                        ui.label(
                                            RichText::new(format!("{} — {}", npc.name, npc.role))
                                                .small(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    ui.add_space(4.0);
                    if ui
                        .add(
                            egui::Button::new("Wystartuj tutaj")
                                .min_size(Vec2::new(ui.available_width(), 0.0)),
                        )
                        .clicked()
                    {
                        self.session.pending_spawn = Some((row.x, row.y));
                        self.session.confirm_spawn();
                        self.session.focus_player();
                    }
                }
            });
            ui.add_space(4.0);
        }
    }

    fn ui_travel(&mut self, ui: &mut egui::Ui) {
        if self.session.player_spawn.is_none() {
            return;
        }
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        ui.label(RichText::new("Podróż").strong());

        ui.label(RichText::new("Znane miejsca").small());
        let places = self.session.known_travel_dests();
        if places.is_empty() {
            ui.label(RichText::new("Odwiedź osadę, by ją zapamiętać").small().weak());
        } else {
            for dest in &places {
                let here = self
                    .session
                    .player_pos
                    .map(|p| player_at(p, (dest.x, dest.y)))
                    .unwrap_or(false);
                let label = if here {
                    format!("{} (tu)", dest.label)
                } else {
                    dest.label.clone()
                };
                if ui
                    .add_enabled(
                        !here && !self.session.loading,
                        egui::Button::new(label).min_size(Vec2::new(ui.available_width(), 0.0)),
                    )
                    .clicked()
                {
                    self.session.go_to_destination(dest.x, dest.y, dest.region, dest.chunk);
                }
            }
        }

        ui.add_space(8.0);
        ui.label(RichText::new("Kierunki").small());
        for dir in Cardinal::ALL {
            if ui
                .add_enabled(
                    !self.session.loading,
                    egui::Button::new(dir.label()).min_size(Vec2::new(ui.available_width(), 0.0)),
                )
                .clicked()
            {
                self.session.walk_toward_cardinal(dir);
            }
        }
    }

    fn ui_game_clock(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Czas gry").strong());
                ui.add_space(4.0);
                ui.label(
                    RichText::new(self.session.game.time().format_label())
                        .monospace()
                        .size(16.0),
                );
                let status = if self.session.game.paused() {
                    "Pauza".to_string()
                } else {
                    format!("×{:.0}", self.session.game.speed_mult())
                };
                ui.label(RichText::new(status).small());
            });

            if self.session.player_spawn.is_some() {
                ui.add_space(6.0);
                draw_sky_path_indicator(ui, self.session.game.time());
            }

            ui.add_space(8.0);
            let pause_label = if self.session.game.paused() { "Wznów" } else { "Pauza" };
            if ui
                .add(egui::Button::new(pause_label).min_size(Vec2::new(ui.available_width(), 0.0)))
                .clicked()
            {
                self.session.game.toggle_pause();
            }

            ui.add_space(4.0);
            ui.label(RichText::new("Prędkość").small());
            ui.horizontal(|ui| {
                for &mult in &SPEED_MULTIPLIERS {
                    let selected =
                        !self.session.game.paused() && (self.session.game.speed_mult() - mult).abs() < f64::EPSILON;
                    if ui
                        .selectable_label(selected, format!("×{:.0}", mult))
                        .clicked()
                    {
                        self.session.game.set_speed(mult);
                    }
                }
            });

            ui.add_space(4.0);
            if ui
                .add(egui::Button::new("+1 h").min_size(Vec2::new(ui.available_width(), 0.0)))
                .clicked()
            {
                self.session.game.skip_hours(1);
                self.session.advance_movement(3600.0);
            }

            if let Some((wx, wy)) = self.session.pending_spawn {
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Start gracza").strong());
                    ui.label(
                        RichText::new(format!("({:.0}, {:.0})", wx, wy))
                            .monospace()
                            .small(),
                    );
                });
                ui.add_space(4.0);
                if ui
                    .add_enabled(
                        !self.session.loading,
                        egui::Button::new("Spawn").min_size(Vec2::new(ui.available_width(), 0.0)),
                    )
                    .clicked()
                {
                    self.session.confirm_spawn();
                }
                if ui
                    .add(
                        egui::Button::new("Wróć do listy")
                            .min_size(Vec2::new(ui.available_width(), 0.0)),
                    )
                    .clicked()
                {
                    self.session.cancel_pending_spawn();
                }
            } else if let Some((wx, wy)) = self.session.player_pos {
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Gracz").strong());
                    ui.label(
                        RichText::new(format!("({:.1}, {:.1})", wx, wy))
                            .monospace()
                            .small(),
                    );
                    if !self.session.move_path.is_empty() {
                        ui.label(RichText::new("w drodze · piechota").small());
                    } else {
                        ui.label(RichText::new("piechota 5 km/h").small());
                    }
                });
                ui.add_space(4.0);
                if ui
                    .add(
                        egui::Button::new("Ja").min_size(Vec2::new(ui.available_width(), 0.0)),
                    )
                    .clicked()
                {
                    self.session.focus_player();
                }
                self.ui_travel(ui);
            } else if self.session.show_place_list() {
                ui.separator();
                self.ui_place_list(ui);
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Wydajność").strong());
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("FPS  {:.0}", self.session.perf.fps()))
                        .monospace()
                        .size(15.0),
                );
                ui.label(
                    RichText::new(format!("CPU  {:.0}%", self.session.perf.cpu_percent()))
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
            ui.text_edit_singleline(&mut self.session.draft_seed);
        });
        if let Some(err) = &self.session.seed_error {
            ui.colored_label(Color32::from_rgb(0xcc, 0x44, 0x44), err);
        }

        for (label, hint, value) in [
            (
                "Ukształtowanie",
                "0 = płasko, 1 = pełne pasma (góry i depresje z seeda)",
                &mut self.session.draft_params.orogeny_strength,
            ),
            (
                "Wilgotność",
                "0 = jednolita, 1 = pełny zasięg biomów",
                &mut self.session.draft_params.moisture_strength,
            ),
            (
                "Detal micro",
                "0 = gładko, 1 = pełny detal w zbliżeniu",
                &mut self.session.draft_params.detail_strength,
            ),
            (
                "Falistość terenu",
                "0 = równiny, 1 = pełne wzniesienia bazowe",
                &mut self.session.draft_params.terrain_roughness,
            ),
            (
                "Powierzchnia lądu",
                "0 = mniej lądu, 1 = więcej lądu",
                &mut self.session.draft_params.land_size,
            ),
            (
                "Nieregularność brzegu",
                "0 = gładkie wybrzeże, 1 = pełne zatoki i półwyspy",
                &mut self.session.draft_params.coast_distortion,
            ),
        ] {
            ui.add(egui::Slider::new(value, 0.0..=1.0).text(label).step_by(0.1))
                .on_hover_text(hint);
        }

        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.session.loading, egui::Button::new("Losuj seed"))
                .clicked()
            {
                self.session.random_seed();
            }
            let regen_label = if self.session.loading {
                "Generowanie…"
            } else {
                "Regenerate"
            };
            if ui
                .add_enabled(!self.session.loading, egui::Button::new(regen_label))
                .clicked()
            {
                self.session.apply_seed_and_params();
            }
            let info_label = if self.session.show_seed_info {
                "Ukryj opis"
            } else {
                "Pokaż opis"
            };
            if ui.button(info_label).clicked() {
                self.session.show_seed_info = !self.session.show_seed_info;
            }
        });

        if self.session.show_seed_info {
            ui.label("Seed to nieujemna liczba całkowita. Z niej generator buduje cały świat:");
            ui.label("• profil kształtu — seed % 6 (0 Radial … 5 Continents)");
            ui.label("• pasma górskie i depresje — losowe z seeda");
            ui.label("• wilgotność / detal / wybrzeże — pochodne seeda");
            ui.label("Suwaki skalują intensywność (0–1). Ten sam seed + suwaki → ten sam świat.");
        }

        if self.session.loading {
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
        let mode = match self.session.lod {
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
        let counts = settlement_counts(self.session.settlements.as_ref());
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

    fn handle_map_click(&mut self, pixel: (f32, f32), canvas: (f32, f32)) {
        if self.session.loading || self.session.grid.is_none() {
            return;
        }
        let config = self.session.config();
        let bounds = self.session.lod.bounds(&config);
        let settlements = self.session.view_settlements();
        let (px, py) = pixel;
        let (cw, ch) = canvas;

        if let Some(hit) = hit_settlement(
            px,
            py,
            cw as usize,
            ch as usize,
            &settlements,
            &bounds,
            self.session.settlement_style(),
        ) {
            let settlement = &settlements[hit.settlement_index];
            if self.session.world_revealed(settlement.x, settlement.y, &config) {
                let district = hit
                    .district_index
                    .and_then(|di| settlement.districts.get(di));
                self.session.selected_info = Some(format_settlement_label(settlement, district));
                self.session.selected_hit = Some(hit);
                if matches!(self.session.lod, LodLevel::Chunk { .. }) {
                    if self.session.player_spawn.is_none() {
                        self.session.pending_spawn = Some((settlement.x, settlement.y));
                        self.session.texture_dirty = true;
                    } else {
                        self.session.set_walk_target(settlement.x, settlement.y);
                    }
                    return;
                }
                self.session.texture_dirty = true;
                return;
            }
            // Settlement under fog — ignore hit and continue.
        }

        let chunks_per_side = (config.region_size / config.chunk_size).max(1);
        let cell_size = cw / chunks_per_side as f32;
        let cx = ((px / cell_size).floor() as i32).clamp(0, chunks_per_side as i32 - 1) as u32;
        let cy = ((py / cell_size).floor() as i32).clamp(0, chunks_per_side as i32 - 1) as u32;

        match self.session.lod {
            LodLevel::Global => {
                let region = RegionId { rx: cx, ry: cy };
                self.session.set_lod(LodLevel::Region(region));
            }
            LodLevel::Region(region) => {
                let chunk = ChunkId { cx, cy };
                self.session.set_lod(LodLevel::Chunk { region, chunk });
            }
            LodLevel::Chunk { .. } => {
                let mut parts = Vec::new();
                let mut land_cell: Option<(f32, f32)> = None;
                if let Some(grid) = &self.session.grid {
                    if let Some((wx, wy, biome)) = cell_world_at_pixel(grid, &bounds, px, py, cw, ch)
                    {
                        parts.push(format!("Biom: {}", biome_label(biome)));
                        if is_land_biome(biome) {
                            land_cell = Some((wx, wy));
                        }
                    }
                }
                if let Some(life) = &self.session.life {
                    if let Some(hint) =
                        life_area_summary(life, cw as usize, ch as usize, px, py, &bounds)
                    {
                        parts.push(hint);
                    }
                }
                self.session.selected_hit = None;
                if self.session.player_spawn.is_none() {
                    self.session.pending_spawn = land_cell;
                    self.session.selected_info = Some(if parts.is_empty() {
                        self.session.lod.idle_label(false)
                    } else {
                        parts.join(" · ")
                    });
                    self.session.texture_dirty = true;
                } else if let Some((wx, wy)) = land_cell {
                    if self.session.world_revealed(wx, wy, &config) {
                        self.session.set_walk_target(wx, wy);
                    } else {
                        self.session.selected_info =
                            Some("Poza zasięgiem widzenia.".to_string());
                    }
                } else {
                    self.session.selected_info = Some(if parts.is_empty() {
                        self.session.lod.idle_label(true)
                    } else {
                        parts.join(" · ")
                    });
                    self.session.texture_dirty = true;
                }
            }
        }
    }

    fn ui_map(
        &mut self,
        ui: &mut egui::Ui,
        map_texture: Option<egui::TextureId>,
        size: egui::Vec2,
    ) {
        ui.horizontal(|ui| {
            if let Some(back) = self.session.lod.back_label() {
                if ui
                    .add_enabled(!self.session.loading, egui::Button::new(back))
                    .clicked()
                {
                    self.session.go_back();
                }
            }
            let status = self.session
                .selected_info
                .clone()
                .unwrap_or_else(|| self.session.lod.idle_label(self.session.player_spawn.is_some()));
            ui.label(status);
        });

        let Some(texture_id) = map_texture else {
            ui.centered_and_justified(|ui| ui.spinner());
            return;
        };

        let available = ui.available_size();
        let scale = (available.x / size.x)
            .min(available.y / size.y)
            .max(0.1);
        let display = size * scale;
        let (rect, response) = ui.allocate_exact_size(
            display,
            if matches!(self.session.lod, LodLevel::Chunk { .. }) {
                Sense::click()
            } else {
                Sense::click()
            },
        );

        ui.painter().image(
            texture_id,
            rect,
            egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        {
            let config = self.session.config();
            let world_span = self.session.lod.bounds(&config).span;
            draw_map_scale(ui.painter(), rect, world_span);
        }

        if self.session.loading {
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120));
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                match self.session.lod {
                    LodLevel::Global => "Generowanie mapy świata…",
                    LodLevel::Region(_) => "Generowanie regionu…",
                    LodLevel::Chunk { .. } => "Generowanie obszaru…",
                },
                FontId::proportional(18.0),
                Color32::WHITE,
            );
        }

        if response.hovered() && !matches!(self.session.lod, LodLevel::Chunk { .. }) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }

        if response.clicked() && !self.session.loading {
            if let Some(pos) = response.interact_pointer_pos() {
                let px = ((pos.x - rect.left()) / rect.width()) * size.x;
                let py = ((pos.y - rect.top()) / rect.height()) * size.y;
                self.handle_map_click((px, py), (size.x, size.y));
            }
        }
    }
}

fn nice_scale_km(raw_km: f32) -> f32 {
    if !raw_km.is_finite() || raw_km <= 0.0 {
        return 1.0;
    }
    let exp = raw_km.log10().floor();
    let base = 10f32.powf(exp);
    let n = raw_km / base;
    let nice = if n < 1.5 {
        1.0
    } else if n < 3.5 {
        2.0
    } else if n < 7.5 {
        5.0
    } else {
        10.0
    };
    nice * base
}

/// Map scale bar in the bottom-right corner (1 world unit = 1 km).
fn draw_map_scale(painter: &egui::Painter, map_rect: egui::Rect, world_span_km: f32) {
    if world_span_km <= 0.0 || map_rect.width() <= 1.0 {
        return;
    }
    let px_per_km = map_rect.width() / world_span_km;
    let nice_km = nice_scale_km(80.0 / px_per_km);
    let bar_w = (nice_km * px_per_km).clamp(24.0, map_rect.width() * 0.45);
    let margin = 10.0;
    let label = if nice_km >= 1.0 {
        format!("{:.0} km", nice_km)
    } else {
        format!("{:.0} m", nice_km * 1000.0)
    };

    let x1 = map_rect.right() - margin;
    let x0 = x1 - bar_w;
    let y_bar = map_rect.bottom() - margin - 4.0;
    let y_top = y_bar - 8.0;

    let bg = egui::Rect::from_min_max(
        pos2(x0 - 6.0, y_top - 16.0),
        pos2(x1 + 6.0, map_rect.bottom() - margin + 4.0),
    );
    painter.rect_filled(
        bg,
        3.0,
        Color32::from_rgba_unmultiplied(0, 0, 0, 140),
    );

    let white = Color32::WHITE;
    let stroke = egui::Stroke::new(1.5, white);
    painter.line_segment([pos2(x0, y_bar), pos2(x1, y_bar)], stroke);
    painter.line_segment([pos2(x0, y_top), pos2(x0, y_bar)], stroke);
    painter.line_segment([pos2(x1, y_top), pos2(x1, y_bar)], stroke);
    let xm = (x0 + x1) * 0.5;
    painter.line_segment([pos2(xm, y_bar - 4.0), pos2(xm, y_bar)], stroke);

    painter.text(
        pos2((x0 + x1) * 0.5, y_top - 2.0),
        Align2::CENTER_BOTTOM,
        label,
        FontId::proportional(12.0),
        white,
    );
}

/// Horizontal sky path under the clock: east → south → west, sun by day / moon by night.
fn draw_sky_path_indicator(ui: &mut egui::Ui, time: GameTime) {
    let height = 44.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);

    let pad_x = 4.0;
    let track_y = rect.top() + 16.0;
    let x0 = rect.left() + pad_x;
    let x1 = rect.right() - pad_x;
    let mid_x = (x0 + x1) * 0.5;

    painter.line_segment(
        [pos2(x0, track_y), pos2(x1, track_y)],
        egui::Stroke::new(1.5, Color32::from_gray(120)),
    );
    for x in [x0, mid_x, x1] {
        painter.line_segment(
            [pos2(x, track_y - 4.0), pos2(x, track_y + 4.0)],
            egui::Stroke::new(1.0, Color32::from_gray(160)),
        );
    }

    let label_font = FontId::proportional(10.0);
    let label_color = Color32::from_gray(180);
    painter.text(
        pos2(x0, rect.bottom() - 2.0),
        Align2::LEFT_BOTTOM,
        "Wschód",
        label_font.clone(),
        label_color,
    );
    painter.text(
        pos2(mid_x, rect.bottom() - 2.0),
        Align2::CENTER_BOTTOM,
        "Południe",
        label_font.clone(),
        label_color,
    );
    painter.text(
        pos2(x1, rect.bottom() - 2.0),
        Align2::RIGHT_BOTTOM,
        "Zachód",
        label_font,
        label_color,
    );

    let progress = time.sky_path_progress().clamp(0.0, 1.0);
    let marker_x = x0 + (x1 - x0) * progress;
    let (symbol, color) = match time.sky_body() {
        SkyBody::Sun => ("☀", Color32::from_rgb(255, 200, 64)),
        SkyBody::Moon => ("☾", Color32::from_rgb(180, 200, 255)),
    };
    painter.text(
        pos2(marker_x, track_y),
        Align2::CENTER_CENTER,
        symbol,
        FontId::proportional(16.0),
        color,
    );
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
