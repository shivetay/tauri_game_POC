use egui::ColorImage;

use crate::colors::biome_rgb;
use crate::ecology_draw::{draw_chunk_habitat_overlay, draw_habitat_overlay};
use crate::settlements_draw::{
    draw_roads, draw_settlements, RoadDetail, SettlementDrawStyle, SettlementHit, WorldBounds,
};
use crate::world::ecology::{ChunkEcologyMap, RegionEcologyMap};
use crate::world::settlement::{Road, Settlement};
use crate::world::types::{TerrainCell, TerrainGrid, TileType};

const MAX_ELEVATION_M: f32 = 10_000.0;
const HIGH_ELEVATION_M: f32 = 5_385.0;

fn lerp_byte(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

fn lerp_f(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn is_water_biome(biome: TileType) -> bool {
    matches!(biome, TileType::Water | TileType::DeepWater)
}

fn shade_high_elevation(biome: TileType, elevation_m: f32, base: [u8; 3]) -> [u8; 3] {
    if !matches!(
        biome,
        TileType::Mountain | TileType::Snow | TileType::Tundra
    ) {
        return base;
    }

    let t = ((elevation_m - HIGH_ELEVATION_M) / (MAX_ELEVATION_M - HIGH_ELEVATION_M)).clamp(0.0, 1.0);
    let [r, g, b] = base;

    if biome == TileType::Snow {
        [
            lerp_byte(r, 0xf8, t),
            lerp_byte(g, 0xf8, t),
            lerp_byte(b, 0xff, t),
        ]
    } else {
        [
            lerp_byte(r, 0x9a, t),
            lerp_byte(g, 0x9a, t),
            lerp_byte(b, 0xa8, t),
        ]
    }
}

fn cell_rgb(cell: &TerrainCell) -> [f32; 3] {
    let [r, g, b] = shade_high_elevation(cell.biome, cell.elevation, biome_rgb(cell.biome));
    [r as f32, g as f32, b as f32]
}

/// Sharp biome fills; only a light land↔water fringe for coast AA.
fn cell_rgb_with_coast_aa(grid: &TerrainGrid, x: usize, y: usize) -> [u8; 3] {
    let w = grid.width as usize;
    let h = grid.height as usize;
    let center = &grid.cells[y * w + x];
    let base = cell_rgb(center);
    let center_water = is_water_biome(center.biome);

    let mut opposite = [0.0f32; 3];
    let mut opposite_n = 0.0f32;
    for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
        let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
        let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
        let n = &grid.cells[ny * w + nx];
        if is_water_biome(n.biome) != center_water {
            let rgb = cell_rgb(n);
            opposite[0] += rgb[0];
            opposite[1] += rgb[1];
            opposite[2] += rgb[2];
            opposite_n += 1.0;
        }
    }

    let rgb = if opposite_n > 0.0 {
        // Mild fringe only — keeps biomes crisp inland.
        let t = (0.22 * opposite_n / 4.0).clamp(0.0, 0.22);
        [
            lerp_f(base[0], opposite[0] / opposite_n, t),
            lerp_f(base[1], opposite[1] / opposite_n, t),
            lerp_f(base[2], opposite[2] / opposite_n, t),
        ]
    } else {
        base
    };

    [
        rgb[0].round().clamp(0.0, 255.0) as u8,
        rgb[1].round().clamp(0.0, 255.0) as u8,
        rgb[2].round().clamp(0.0, 255.0) as u8,
    ]
}

pub fn terrain_grid_to_rgba(grid: &TerrainGrid) -> Vec<u8> {
    let w = grid.width as usize;
    let h = grid.height as usize;
    let mut rgba = Vec::with_capacity(grid.cells.len() * 4);
    for y in 0..h {
        for x in 0..w {
            let [r, g, b] = cell_rgb_with_coast_aa(grid, x, y);
            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            rgba.push(255);
        }
    }
    rgba
}

fn blend_pixel(pixels: &mut [u8], i: usize, r: u8, g: u8, b: u8, a: u8) {
    if a == 0 {
        return;
    }
    let af = a as f32 / 255.0;
    let inv = 1.0 - af;
    pixels[i] = (r as f32 * af + pixels[i] as f32 * inv).round() as u8;
    pixels[i + 1] = (g as f32 * af + pixels[i + 1] as f32 * inv).round() as u8;
    pixels[i + 2] = (b as f32 * af + pixels[i + 2] as f32 * inv).round() as u8;
}

fn draw_region_grid(pixels: &mut [u8], width: usize, height: usize, cell_size: f32) {
    let a = (0.5 * 255.0) as u8;
    let mut x = cell_size;
    while x < width as f32 {
        let xi = x.round() as usize;
        if xi < width {
            for y in 0..height {
                let i = (y * width + xi) * 4;
                blend_pixel(pixels, i, 255, 255, 255, a);
            }
        }
        x += cell_size;
    }
    let mut y = cell_size;
    while y < height as f32 {
        let yi = y.round() as usize;
        if yi < height {
            for x in 0..width {
                let i = (yi * width + x) * 4;
                blend_pixel(pixels, i, 255, 255, 255, a);
            }
        }
        y += cell_size;
    }
}

pub struct ComposeInput<'a> {
    pub grid: &'a TerrainGrid,
    pub cell_size: f32,
    pub show_grid: bool,
    pub bounds: WorldBounds,
    pub settlements: &'a [Settlement],
    pub roads: &'a [Road],
    pub road_detail: RoadDetail,
    pub settlement_style: SettlementDrawStyle,
    pub habitat: Option<&'a RegionEcologyMap>,
    pub life: Option<&'a ChunkEcologyMap>,
    pub hovered: Option<&'a SettlementHit>,
}

pub fn compose_map_image(input: ComposeInput<'_>) -> ColorImage {
    let w = input.grid.width as usize;
    let h = input.grid.height as usize;
    let mut rgba = terrain_grid_to_rgba(input.grid);

    if let Some(habitat) = input.habitat {
        draw_habitat_overlay(&mut rgba, w, h, habitat);
    } else if let Some(life) = input.life {
        if !life.cells.is_empty() {
            draw_chunk_habitat_overlay(&mut rgba, w, h, life);
        }
    }

    if input.show_grid {
        draw_region_grid(&mut rgba, w, h, input.cell_size);
    }

    draw_roads(
        &mut rgba,
        w,
        h,
        input.roads,
        &input.bounds,
        input.road_detail,
        Some(input.grid),
    );
    draw_settlements(
        &mut rgba,
        w,
        h,
        input.settlements,
        &input.bounds,
        input.hovered,
        input.settlement_style,
        Some(input.grid),
    );

    ColorImage::from_rgba_unmultiplied([w, h], &rgba)
}

pub fn biome_at_pixel(
    grid: &TerrainGrid,
    px: f32,
    py: f32,
    canvas_width: f32,
    canvas_height: f32,
) -> Option<TileType> {
    if grid.width == 0 || grid.height == 0 || grid.cells.is_empty() {
        return None;
    }
    let gx = ((px / canvas_width) * grid.width as f32)
        .floor()
        .clamp(0.0, (grid.width - 1) as f32) as usize;
    let gy = ((py / canvas_height) * grid.height as f32)
        .floor()
        .clamp(0.0, (grid.height - 1) as f32) as usize;
    grid.cells.get(gy * grid.width as usize + gx).map(|c| c.biome)
}
