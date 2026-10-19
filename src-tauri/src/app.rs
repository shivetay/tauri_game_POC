use bevy_egui::egui::{self, pos2, Align2, Color32, FontId, RichText, Sense, Vec2};

use crate::borders_draw::BorderDrawMode;
use crate::colors::BIOMES;
use crate::ecology_draw::{
    life_area_summary, FAUNA_BIRD, FAUNA_LARGE_MAMMAL, FAUNA_SMALL, VEGETATION_RGB,
};
use crate::game_time::{
    GameTime, SkyBody, DAYS_PER_MONTH, DAYS_PER_YEAR, SPEED_MULTIPLIERS,
};
use crate::render::{compose_map_rgba, BordersOverlay, ComposeInput};
use crate::session::{
    biome_label, cell_world_at_pixel, format_pop, is_land_biome, player_at, MapSession,
    Cardinal, LocalAreaTab, LodLevel, VISION_RADIUS,
};
use crate::settlements_draw::{
    district_info, format_settlement_label, hit_settlement, road_info, settlement_info,
    SettlementHit, WorldBounds, DISTRICT_KIND_ORDER, ROAD_KIND_ORDER, SETTLEMENT_KIND_ORDER,
};
use crate::world::borders::Realm;
use crate::world::settlement::{Settlement, SettlementKind, SettlementMap};
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
        let borders = match (s.border_draw_mode(), s.territory()) {
            (Some(mode), Some(territory)) => Some(BordersOverlay {
                mode,
                territory,
                realms: s.realms(),
                realm_borders: s.realm_borders(),
                city_borders: s.city_borders(),
                diplomacy: Some(&s.diplomacy),
            }),
            _ => None,
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
            borders,
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
                        self.session.selected_realm = None;
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
                            let buildings = self.session.buildings.as_ref();
                            let roster = self
                                .session.npcs
                                .as_ref()
                                .map(|m| m.in_settlement(row.index))
                                .unwrap_or(&[]);
                            if roster.is_empty() {
                                ui.label(RichText::new("Brak NPC.").small());
                            } else {
                                for npc in roster {
                                    let line = match buildings {
                                        Some(b) => npc.preview_line(b, None),
                                        None => format!("{} — {}", npc.name, npc.role),
                                    };
                                    ui.label(RichText::new(line).small());
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
                                let buildings = self.session.buildings.as_ref();
                                let roster = self
                                    .session.npcs
                                    .as_ref()
                                    .map(|m| m.in_district(row.index, di))
                                    .unwrap_or(&[]);
                                if roster.is_empty() {
                                    ui.label(RichText::new("Brak NPC.").small());
                                } else {
                                    for npc in roster {
                                        let line = match buildings {
                                            Some(b) => npc.preview_line(b, None),
                                            None => format!("{} — {}", npc.name, npc.role),
                                        };
                                        ui.label(RichText::new(line).small());
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

    fn ui_local_area(&mut self, ui: &mut egui::Ui) {
        let Some(area) = self.session.player_area else {
            return;
        };
        let Some(settlement) = self
            .session
            .settlements
            .as_ref()
            .and_then(|m| m.settlements.get(area.settlement_index))
        else {
            return;
        };
        let settlement_name = settlement.name.clone();
        let district_rows: Vec<(String, String)> = settlement
            .districts
            .iter()
            .map(|d| {
                (
                    format!("{} — {}", district_info(d.kind).label, d.name),
                    d.name.clone(),
                )
            })
            .collect();

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        ui.label(RichText::new("Lokalizacja").strong());
        ui.label(RichText::new(&settlement_name).small());
        if !district_rows.is_empty() {
            ui.label(
                RichText::new("Przełącz obszar, by zobaczyć mieszkańców")
                    .small()
                    .weak(),
            );
            ui.add_space(4.0);
            ui.label(RichText::new("Dzielnice").small().strong());
            for (di, (label, _)) in district_rows.iter().enumerate() {
                let selected = area.district_index == Some(di);
                if ui
                    .selectable_label(selected, RichText::new(label).small())
                    .clicked()
                {
                    self.session.set_player_district(di);
                }
            }
        } else {
            ui.label(RichText::new("Brak podziału na dzielnice.").small().weak());
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.session.local_area_tab == LocalAreaTab::Npcs,
                    RichText::new("NPC").small(),
                )
                .clicked()
            {
                self.session.local_area_tab = LocalAreaTab::Npcs;
            }
            if ui
                .selectable_label(
                    self.session.local_area_tab == LocalAreaTab::Buildings,
                    RichText::new("Budynki").small(),
                )
                .clicked()
            {
                self.session.local_area_tab = LocalAreaTab::Buildings;
            }
        });
        ui.add_space(4.0);

        match self.session.local_area_tab {
            LocalAreaTab::Npcs => {
                let buildings = self.session.buildings.as_ref();
                if let Some(npc_map) = self.session.npcs.as_ref() {
                    let mut shown = 0usize;
                    for (i, npc) in npc_map.npcs.iter().enumerate() {
                        if npc.settlement_index != area.settlement_index {
                            continue;
                        }
                        if let Some(di) = area.district_index {
                            if npc.district_index != di {
                                continue;
                            }
                        }
                        shown += 1;
                        let supply = self.session.npc_supply_dest(i);
                        let mut line = match buildings {
                            Some(b) => npc.preview_line(b, supply),
                            None => format!("{} — {}", npc.name, npc.role),
                        };
                        if let Some(status) = self.session.npc_status_label(i) {
                            line.push_str(" · ");
                            line.push_str(&status);
                        }
                        ui.label(RichText::new(line).small());
                    }
                    if shown == 0 {
                        ui.label(RichText::new("Brak NPC.").small());
                    }
                } else {
                    ui.label(RichText::new("Brak NPC.").small());
                }
            }
            LocalAreaTab::Buildings => {
                let roster = match area.district_index {
                    Some(di) => self
                        .session
                        .buildings
                        .as_ref()
                        .map(|m| m.in_district(area.settlement_index, di))
                        .unwrap_or(&[]),
                    None => self
                        .session
                        .buildings
                        .as_ref()
                        .map(|m| m.in_settlement(area.settlement_index))
                        .unwrap_or(&[]),
                };
                if roster.is_empty() {
                    ui.label(RichText::new("Brak budynków.").small());
                } else {
                    for b in roster {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(b.title_line()).small());
                            if let Some(stock) = b.stock_label() {
                                ui.label(
                                    RichText::new(format!("[{stock}]"))
                                        .small()
                                        .weak(),
                                );
                            }
                        });
                        if let Some(needs) = b.needs_label() {
                            ui.label(RichText::new(needs).small().weak());
                        }
                    }
                }
            }
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

            if self.session.simulation_active() {
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
            ui.label(RichText::new("Przeskocz").small());
            if ui
                .add(egui::Button::new("+1 h").min_size(Vec2::new(ui.available_width(), 0.0)))
                .clicked()
            {
                self.session.skip_game_hours(1);
            }
            ui.horizontal(|ui| {
                let skips = [
                    ("+1 d", 1u32),
                    ("+1 tyg", 7),
                    ("+1 mies", DAYS_PER_MONTH),
                    ("+1 rok", DAYS_PER_YEAR),
                ];
                for (label, days) in skips {
                    if ui.button(label).clicked() {
                        self.session.skip_game_days(days);
                    }
                }
            });

            if self.session.player_spawn.is_none() {
                ui.add_space(8.0);
                let sim_label = if self.session.world_sim {
                    "Zatrzymaj symulację"
                } else {
                    "Symulacja (bez gracza)"
                };
                if ui
                    .add_enabled(
                        !self.session.loading && self.session.grid.is_some(),
                        egui::Button::new(sim_label).min_size(Vec2::new(ui.available_width(), 0.0)),
                    )
                    .clicked()
                {
                    self.session.toggle_world_sim();
                }
                if self.session.world_sim {
                    ui.label(
                        RichText::new("Świat żyje bez markera gracza.")
                            .small()
                            .weak(),
                    );
                }
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
                self.ui_local_area(ui);
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
        if ui
            .checkbox(&mut self.session.show_borders, "Granice")
            .changed()
        {
            if !self.session.show_borders {
                self.session.selected_realm = None;
            }
            self.session.texture_dirty = true;
        }
        self.ui_realm_dossier(ui);
        ui.separator();
        self.ui_ecology_legend(ui);
        self.ui_settlement_legend(ui);
        self.ui_biome_legend(ui);
    }

    fn ui_realm_dossier(&mut self, ui: &mut egui::Ui) {
        let Some(realm_id) = self.session.selected_realm else {
            return;
        };
        let Some(map) = self.session.settlements.as_ref() else {
            return;
        };
        let Some(realm) = map.realms.get(realm_id) else {
            return;
        };
        let capital_name = map
            .settlements
            .get(realm.capital_idx)
            .map(|s| s.name.as_str())
            .unwrap_or("—");
        let population = realm.population(&map.settlements);
        let relations: Vec<(String, String, String)> = realm
            .relations
            .iter()
            .map(|rel| {
                let other = map
                    .realms
                    .get(rel.other_realm_id)
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| format!("#{}", rel.other_realm_id));
                let (kind_label, tension_txt) = match self
                    .session
                    .diplomacy
                    .edge(realm_id, rel.other_realm_id)
                {
                    Some(edge) => {
                        let war = if edge.at_war { " · WOJNA" } else { "" };
                        (
                            format!("{}{war}", edge.kind.label()),
                            format!("napięcie {:+}", edge.tension),
                        )
                    }
                    None => (rel.kind.label().to_string(), "napięcie —".into()),
                };
                (other, kind_label, tension_txt)
            })
            .collect();
        let wars: Vec<String> = self
            .session
            .diplomacy
            .edges
            .iter()
            .filter(|e| e.at_war && (e.realm_a == realm_id || e.realm_b == realm_id))
            .map(|e| {
                let other_id = if e.realm_a == realm_id {
                    e.realm_b
                } else {
                    e.realm_a
                };
                let other = map
                    .realms
                    .get(other_id)
                    .map(|r| r.name.as_str())
                    .unwrap_or("?");
                format!("{other} ({} mies.)", e.war_months)
            })
            .collect();
        let embargoes_out: Vec<String> = self
            .session
            .diplomacy
            .embargoes
            .iter()
            .filter(|e| e.from == realm_id)
            .map(|e| {
                map.realms
                    .get(e.against)
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| format!("#{}", e.against))
            })
            .collect();
        let embargoes_in: Vec<String> = self
            .session
            .diplomacy
            .embargoes
            .iter()
            .filter(|e| e.against == realm_id)
            .map(|e| {
                map.realms
                    .get(e.from)
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| format!("#{}", e.from))
            })
            .collect();
        let occupied = self.session.diplomacy.occupied_count(realm_id);
        let last_event = self.session.diplomacy.last_event.clone();
        let goals = realm.goals.clone();
        let exports: Vec<(String, u32)> = realm
            .exports
            .iter()
            .map(|c| (c.good.label().to_string(), c.amount))
            .collect();
        let imports: Vec<(String, u32)> = realm
            .imports
            .iter()
            .map(|c| (c.good.label().to_string(), c.amount))
            .collect();
        let routes: Vec<String> = realm
            .trade_routes
            .iter()
            .map(|route| {
                let partner = map
                    .realms
                    .get(route.partner_realm_id)
                    .map(|r| r.name.as_str())
                    .unwrap_or("?");
                let good = route.good.label();
                let blocked = self
                    .session
                    .diplomacy
                    .trade_blocked(realm_id, route.partner_realm_id);
                let line = if route.exporting {
                    format!("eksport {good} → {partner}")
                } else {
                    format!("import {good} ← {partner}")
                };
                if blocked {
                    format!("{line} (zawieszony)")
                } else {
                    line
                }
            })
            .collect();
        let ruler = realm.ruler_name.clone();
        let name = realm.name.clone();
        let government = realm.government.label();
        let culture = realm.culture.label();
        let religion = realm.religion.label();
        let ambition = realm.ambition.label();
        let fame = self.session.diplomacy.fame_of(realm);
        let manpower = realm.manpower;
        let military = realm.military_strength;
        let laws: Vec<String> = realm
            .laws
            .iter()
            .map(|law| {
                let sources = law
                    .sources
                    .iter()
                    .map(|s| s.label())
                    .collect::<Vec<_>>()
                    .join(", ");
                let status = if law.enforced {
                    "egzekwowane"
                } else {
                    "bez egzekucji"
                };
                format!(
                    "{} · {} · {} · {} ({})",
                    law.id.label(),
                    law.domain.label(),
                    law.strength.label(),
                    status,
                    sources
                )
            })
            .collect();

        ui.add_space(8.0);
        ui.separator();
        ui.label(RichText::new("Państwo").strong());
        ui.label(RichText::new(&name).heading());
        ui.label(format!("Władca: {ruler}"));
        ui.label(format!("Stolica: {capital_name}"));
        ui.label(format!("Populacja: {}", format_pop(population)));
        ui.label(format!("Kultura: {culture}"));
        ui.label(format!("Władza: {government}"));
        ui.label(format!("Religia: {religion}"));
        ui.label(format!(
            "Siła wojskowa: {military}/100 (sława {fame} · liczebność {manpower})"
        ));
        ui.label(format!("Ambicja władcy: {ambition}"));
        ui.add_space(4.0);
        ui.label(RichText::new("Prawa").small().strong());
        if laws.is_empty() {
            ui.label(RichText::new("Brak").small());
        } else {
            for line in &laws {
                ui.label(RichText::new(format!("• {line}")).small());
            }
        }
        ui.add_space(4.0);
        ui.label(RichText::new("Eksport").small().strong());
        if exports.is_empty() {
            ui.label(RichText::new("Brak nadwyżek").small());
        } else {
            for (good, amount) in &exports {
                ui.label(RichText::new(format!("• {good} ({amount})")).small());
            }
        }
        ui.label(RichText::new("Import").small().strong());
        if imports.is_empty() {
            ui.label(RichText::new("Brak niedoborów").small());
        } else {
            for (good, amount) in &imports {
                ui.label(RichText::new(format!("• {good} ({amount})")).small());
            }
        }
        ui.label(RichText::new("Handel").small().strong());
        if routes.is_empty() {
            ui.label(RichText::new("Brak szlaków z partnerami").small());
        } else {
            for line in &routes {
                ui.label(RichText::new(format!("• {line}")).small());
            }
        }
        if !embargoes_out.is_empty() {
            ui.label(RichText::new("Embargo (nakłada)").small().strong());
            for name in &embargoes_out {
                ui.label(RichText::new(format!("• {name}")).small());
            }
        }
        if !embargoes_in.is_empty() {
            ui.label(RichText::new("Embargo (otrzymuje)").small().strong());
            for name in &embargoes_in {
                ui.label(RichText::new(format!("• {name}")).small());
            }
        }
        ui.add_space(4.0);
        ui.label(RichText::new("Cele").small().strong());
        if goals.is_empty() {
            ui.label(RichText::new("Brak").small());
        } else {
            for goal in &goals {
                ui.label(RichText::new(format!("• {goal}")).small());
            }
        }
        if !relations.is_empty() || !wars.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new("Dyplomacja").small().strong());
            ui.label(
                RichText::new("Relacje / wojny w czasie gry (co miesiąc).")
                    .small()
                    .weak(),
            );
            if let Some(ev) = &last_event {
                ui.label(RichText::new(ev).small().italics());
            }
            if !wars.is_empty() {
                ui.label(RichText::new("W wojnie z:").small());
                for w in &wars {
                    ui.label(RichText::new(format!("• {w}")).small());
                }
            }
            if occupied > 0 {
                ui.label(
                    RichText::new(format!("Okupacja (de facto): {occupied} komórek"))
                        .small(),
                );
            }
            for (other, kind, tension) in &relations {
                ui.label(RichText::new(format!("• {other}: {kind} ({tension})")).small());
            }
        }
        ui.add_space(4.0);
        if ui.button("Zamknij").clicked() {
            self.session.selected_realm = None;
        }
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
                self.session.selected_realm = None;
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

        if !self.session.loading {
            let config = self.session.config();
            let bounds = self.session.lod.bounds(&config);
            let settlements = self.session.view_settlements();
            draw_settlement_labels(
                ui.painter(),
                rect,
                &bounds,
                &settlements,
                self.session.lod,
                |wx, wy| self.session.world_revealed(wx, wy, &config),
            );

            if matches!(
                self.session.border_draw_mode(),
                Some(BorderDrawMode::Realms) | Some(BorderDrawMode::RealmsAndCities)
            ) {
                let realms_settlements = self
                    .session
                    .settlements
                    .as_ref()
                    .map(|m| m.settlements.as_slice())
                    .unwrap_or(&[]);
                draw_realm_labels(
                    ui.painter(),
                    rect,
                    &bounds,
                    self.session.realms(),
                    realms_settlements,
                );
            }
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
                let realm_hit = matches!(
                    self.session.border_draw_mode(),
                    Some(BorderDrawMode::Realms) | Some(BorderDrawMode::RealmsAndCities)
                )
                .then(|| {
                    let config = self.session.config();
                    let bounds = self.session.lod.bounds(&config);
                    let settlements = self
                        .session
                        .settlements
                        .as_ref()
                        .map(|m| m.settlements.as_slice())
                        .unwrap_or(&[]);
                    hit_realm_label(
                        ui,
                        pos,
                        rect,
                        &bounds,
                        self.session.realms(),
                        settlements,
                    )
                })
                .flatten();

                if let Some(realm_id) = realm_hit {
                    let realm_name = self
                        .session
                        .realms()
                        .get(realm_id)
                        .map(|r| r.name.clone());
                    self.session.selected_realm = Some(realm_id);
                    self.session.selected_hit = None;
                    if let Some(name) = realm_name {
                        self.session.selected_info = Some(format!("Państwo: {name}"));
                    }
                    return;
                }

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

fn world_to_map_pos(wx: f32, wy: f32, bounds: &WorldBounds, map_rect: egui::Rect) -> egui::Pos2 {
    let sx = map_rect.left() + ((wx - bounds.x0) / bounds.span) * map_rect.width();
    let sy = map_rect.top() + ((wy - bounds.y0) / bounds.span) * map_rect.height();
    pos2(sx, sy)
}

fn realm_label_anchor(realm: &Realm, settlements: &[Settlement]) -> Option<(f32, f32)> {
    let mut sx = 0.0f32;
    let mut sy = 0.0f32;
    let mut n = 0usize;
    for &si in &realm.settlement_idxs {
        let Some(s) = settlements.get(si) else {
            continue;
        };
        sx += s.x;
        sy += s.y;
        n += 1;
    }
    if n == 0 {
        return None;
    }
    Some((sx / n as f32, sy / n as f32))
}

fn draw_outlined_text(
    painter: &egui::Painter,
    pos: egui::Pos2,
    align: Align2,
    text: &str,
    font: FontId,
    fill: Color32,
) {
    let outline = Color32::from_rgba_unmultiplied(0, 0, 0, 200);
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        painter.text(pos + Vec2::new(dx, dy), align, text, font.clone(), outline);
    }
    painter.text(pos, align, text, font, fill);
}

fn settlement_label_rank(kind: SettlementKind) -> u8 {
    match kind {
        SettlementKind::City => 0,
        SettlementKind::Town => 1,
        SettlementKind::Village => 2,
        SettlementKind::Hamlet => 3,
    }
}

fn settlement_label_allowed(kind: SettlementKind, lod: LodLevel) -> bool {
    match lod {
        LodLevel::Global => matches!(kind, SettlementKind::City | SettlementKind::Town),
        LodLevel::Region(_) => {
            matches!(
                kind,
                SettlementKind::City | SettlementKind::Town | SettlementKind::Village
            )
        }
        LodLevel::Chunk { .. } => true,
    }
}

fn settlement_label_font(kind: SettlementKind, map_w: f32) -> FontId {
    let size = match kind {
        SettlementKind::City => (map_w * 0.016).clamp(11.0, 15.0),
        SettlementKind::Town => (map_w * 0.014).clamp(10.0, 13.0),
        SettlementKind::Village => (map_w * 0.012).clamp(9.0, 12.0),
        SettlementKind::Hamlet => (map_w * 0.011).clamp(8.0, 11.0),
    };
    FontId::proportional(size)
}

/// Names above settlement markers; denser LOD shows more kinds; overlaps drop lesser ranks.
fn draw_settlement_labels(
    painter: &egui::Painter,
    map_rect: egui::Rect,
    bounds: &WorldBounds,
    settlements: &[Settlement],
    lod: LodLevel,
    revealed: impl Fn(f32, f32) -> bool,
) {
    if bounds.span <= 0.0 || settlements.is_empty() {
        return;
    }

    let mut candidates: Vec<&Settlement> = settlements
        .iter()
        .filter(|s| settlement_label_allowed(s.kind, lod))
        .filter(|s| {
            s.x >= bounds.x0
                && s.y >= bounds.y0
                && s.x <= bounds.x0 + bounds.span
                && s.y <= bounds.y0 + bounds.span
                && revealed(s.x, s.y)
        })
        .collect();
    candidates.sort_by(|a, b| {
        settlement_label_rank(a.kind)
            .cmp(&settlement_label_rank(b.kind))
            .then_with(|| a.name.cmp(&b.name))
    });

    let min_sep = match lod {
        LodLevel::Global => 52.0,
        LodLevel::Region(_) => 40.0,
        LodLevel::Chunk { .. } => 28.0,
    };
    let color = Color32::from_rgb(0xf2, 0xec, 0xde);
    let mut placed: Vec<egui::Pos2> = Vec::new();

    for s in candidates {
        let marker = world_to_map_pos(s.x, s.y, bounds, map_rect);
        // Above the marker so the name stays clear of the settlement glyph.
        let label_pos = marker - Vec2::new(0.0, 8.0);
        if placed
            .iter()
            .any(|p| (*p - label_pos).length() < min_sep)
        {
            continue;
        }
        placed.push(label_pos);
        draw_outlined_text(
            painter,
            label_pos,
            Align2::CENTER_BOTTOM,
            &s.name,
            settlement_label_font(s.kind, map_rect.width()),
            color,
        );
    }
}

fn realm_label_font(map_w: f32) -> FontId {
    FontId::proportional((map_w * 0.018).clamp(11.0, 18.0))
}

fn realm_label_screen_pos(
    realm: &Realm,
    settlements: &[Settlement],
    bounds: &WorldBounds,
    map_rect: egui::Rect,
) -> Option<egui::Pos2> {
    let (wx, wy) = realm_label_anchor(realm, settlements)?;
    if wx < bounds.x0
        || wy < bounds.y0
        || wx > bounds.x0 + bounds.span
        || wy > bounds.y0 + bounds.span
    {
        return None;
    }
    Some(world_to_map_pos(wx, wy, bounds, map_rect))
}

fn hit_realm_label(
    ui: &egui::Ui,
    pointer: egui::Pos2,
    map_rect: egui::Rect,
    bounds: &WorldBounds,
    realms: &[Realm],
    settlements: &[Settlement],
) -> Option<usize> {
    if bounds.span <= 0.0 || realms.is_empty() {
        return None;
    }
    let font = realm_label_font(map_rect.width());
    let mut best: Option<(usize, f32)> = None;
    for realm in realms {
        let Some(pos) = realm_label_screen_pos(realm, settlements, bounds, map_rect) else {
            continue;
        };
        let galley =
            ui.fonts_mut(|f| f.layout_no_wrap(realm.name.clone(), font.clone(), Color32::WHITE));
        let rect = Align2::CENTER_CENTER
            .anchor_size(pos, galley.size())
            .expand(3.0);
        if rect.contains(pointer) {
            let d = pointer.distance(pos);
            if best.map_or(true, |(_, bd)| d < bd) {
                best = Some((realm.id, d));
            }
        }
    }
    best.map(|(id, _)| id)
}

/// Realm name near territory center (settlement names are drawn separately).
fn draw_realm_labels(
    painter: &egui::Painter,
    map_rect: egui::Rect,
    bounds: &WorldBounds,
    realms: &[Realm],
    settlements: &[Settlement],
) {
    if bounds.span <= 0.0 || realms.is_empty() {
        return;
    }

    let realm_font = realm_label_font(map_rect.width());
    let realm_color = Color32::from_rgb(0xf4, 0xee, 0xe0);

    for realm in realms {
        let Some(pos) = realm_label_screen_pos(realm, settlements, bounds, map_rect) else {
            continue;
        };
        draw_outlined_text(
            painter,
            pos,
            Align2::CENTER_CENTER,
            &realm.name,
            realm_font.clone(),
            realm_color,
        );
    }
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
