//! Port of former src/map/ecology.ts (canvas → RGBA).

use crate::world::ecology::{ChunkEcologyMap, HabitatCell, LifeInstance, RegionEcologyMap};
use crate::settlements_draw::WorldBounds;

pub const VEGETATION_RGB: [u8; 3] = [0x4a, 0x8a, 0x5a];

pub struct FaunaGroupInfo {
    pub label: &'static str,
    pub rgb: [u8; 3],
}

pub const FAUNA_LARGE_MAMMAL: FaunaGroupInfo = FaunaGroupInfo {
    label: "Duże ssaki",
    rgb: [0xc4, 0x7a, 0x2c],
};
pub const FAUNA_BIRD: FaunaGroupInfo = FaunaGroupInfo {
    label: "Ptaki",
    rgb: [0x3a, 0x7c, 0xb8],
};
pub const FAUNA_SMALL: FaunaGroupInfo = FaunaGroupInfo {
    label: "Mała fauna",
    rgb: [0x8a, 0x4a, 0x9a],
};

const HABITAT_THRESHOLD: f32 = 0.28;
const GROUP_THRESHOLD: f32 = 0.38;

fn blend_src_over(dst: &mut [u8], i: usize, r: u8, g: u8, b: u8, a: u8) {
    if a == 0 {
        return;
    }
    let af = a as f32 / 255.0;
    let inv = 1.0 - af;
    dst[i] = (r as f32 * af + dst[i] as f32 * inv).round() as u8;
    dst[i + 1] = (g as f32 * af + dst[i + 1] as f32 * inv).round() as u8;
    dst[i + 2] = (b as f32 * af + dst[i + 2] as f32 * inv).round() as u8;
}

fn paint_habitat_pixels(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    width: usize,
    height: usize,
    cells: &[HabitatCell],
) {
    if width == 0 || height == 0 || cells.is_empty() {
        return;
    }
    let scale_x = canvas_width as f32 / width as f32;
    let scale_y = canvas_height as f32 / height as f32;

    for py in 0..canvas_height {
        let gy = ((py as f32 / scale_y) as usize).min(height - 1);
        for px in 0..canvas_width {
            let gx = ((px as f32 / scale_x) as usize).min(width - 1);
            let cell = &cells[gy * width + gx];
            if cell.potential < HABITAT_THRESHOLD {
                continue;
            }

            let mut r = VEGETATION_RGB[0] as f32;
            let mut g = VEGETATION_RGB[1] as f32;
            let mut b = VEGETATION_RGB[2] as f32;
            let mut a = (cell.potential * 55.0).floor() as u8;

            let mut groups = [
                (cell.large_mammals, FAUNA_LARGE_MAMMAL.rgb),
                (cell.birds, FAUNA_BIRD.rgb),
                (cell.small_fauna, FAUNA_SMALL.rgb),
            ];
            groups.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal));

            let top = groups[0];
            if top.0 >= GROUP_THRESHOLD {
                let t = ((top.0 - GROUP_THRESHOLD) / 0.45).min(1.0);
                r = r * (1.0 - t) + top.1[0] as f32 * t;
                g = g * (1.0 - t) + top.1[1] as f32 * t;
                b = b * (1.0 - t) + top.1[2] as f32 * t;
                a = a.max((40.0 + top.0 * 70.0).floor() as u8);
                let second = groups[1];
                if second.0 >= GROUP_THRESHOLD && (px + py) % 6 < 2 {
                    r = r * 0.55 + second.1[0] as f32 * 0.45;
                    g = g * 0.55 + second.1[1] as f32 * 0.45;
                    b = b * 0.55 + second.1[2] as f32 * 0.45;
                }
            }

            let i = (py * canvas_width + px) * 4;
            blend_src_over(
                pixels,
                i,
                r.round() as u8,
                g.round() as u8,
                b.round() as u8,
                a,
            );
        }
    }
}

pub fn draw_habitat_overlay(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    habitat: &RegionEcologyMap,
) {
    paint_habitat_pixels(
        pixels,
        canvas_width,
        canvas_height,
        habitat.width as usize,
        habitat.height as usize,
        &habitat.cells,
    );
}

pub fn draw_chunk_habitat_overlay(
    pixels: &mut [u8],
    canvas_width: usize,
    canvas_height: usize,
    life: &ChunkEcologyMap,
) {
    paint_habitat_pixels(
        pixels,
        canvas_width,
        canvas_height,
        life.width as usize,
        life.height as usize,
        &life.cells,
    );
}

fn canvas_to_world(px: f32, py: f32, canvas_w: f32, canvas_h: f32, bounds: &WorldBounds) -> (f32, f32) {
    (
        bounds.x0 + (px / canvas_w) * bounds.span,
        bounds.y0 + (py / canvas_h) * bounds.span,
    )
}

fn uniq_species(items: &[LifeInstance]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for inst in items {
        let key = format!("{}|{}", inst.species, inst.subspecies);
        if !seen.insert(key) {
            continue;
        }
        out.push(format!("{} ({})", inst.species, inst.subspecies));
        if out.len() >= 4 {
            break;
        }
    }
    out
}

pub fn life_area_summary(
    life: &ChunkEcologyMap,
    canvas_width: usize,
    canvas_height: usize,
    px: f32,
    py: f32,
    bounds: &WorldBounds,
) -> Option<String> {
    let (x, y) = canvas_to_world(px, py, canvas_width as f32, canvas_height as f32, bounds);
    let radius = bounds.span * 0.12;
    let r2 = radius * radius;

    let flora_nearby: Vec<&LifeInstance> = life
        .flora
        .iter()
        .filter(|inst| {
            let dx = inst.x - x;
            let dy = inst.y - y;
            dx * dx + dy * dy <= r2
        })
        .collect();
    let fauna_nearby: Vec<&LifeInstance> = life
        .fauna
        .iter()
        .filter(|inst| {
            let dx = inst.x - x;
            let dy = inst.y - y;
            dx * dx + dy * dy <= r2
        })
        .collect();

    if flora_nearby.is_empty() && fauna_nearby.is_empty() {
        if life.width == 0 || life.height == 0 {
            return None;
        }
        let gx = ((px / canvas_width as f32) * life.width as f32)
            .floor()
            .clamp(0.0, (life.width - 1) as f32) as usize;
        let gy = ((py / canvas_height as f32) * life.height as f32)
            .floor()
            .clamp(0.0, (life.height - 1) as f32) as usize;
        let cell = life.cells.get(gy * life.width as usize + gx)?;
        if cell.potential < HABITAT_THRESHOLD {
            return None;
        }
        return Some("Obszar życia — brak zidentyfikowanych gatunków w zasięgu".to_string());
    }

    let flora_owned: Vec<LifeInstance> = flora_nearby.into_iter().cloned().collect();
    let fauna_owned: Vec<LifeInstance> = fauna_nearby.into_iter().cloned().collect();
    let flora_part = uniq_species(&flora_owned);
    let fauna_part = uniq_species(&fauna_owned);
    let mut parts = Vec::new();
    if !flora_part.is_empty() {
        parts.push(format!("Flora: {}", flora_part.join(", ")));
    }
    if !fauna_part.is_empty() {
        parts.push(format!("Fauna: {}", fauna_part.join(", ")));
    }
    Some(parts.join(" · "))
}

pub fn region_tile_summary(habitat: &RegionEcologyMap, cx: u32, cy: u32) -> Option<String> {
    let across = habitat.tiles_across;
    if across == 0 || habitat.tiles.is_empty() {
        return None;
    }
    if cx >= across || cy >= across {
        return None;
    }
    let tile = habitat.tiles.get((cy * across + cx) as usize)?;
    let mut parts = Vec::new();
    if !tile.flora.is_empty() {
        parts.push(format!("Flora: {}", tile.flora.join(", ")));
    }
    if !tile.fauna.is_empty() {
        parts.push(format!("Fauna: {}", tile.fauna.join(", ")));
    }
    if parts.is_empty() {
        return Some(format!("Kafel ({cx}, {cy}) — brak spodziewanego życia"));
    }
    Some(format!("Kafel ({cx}, {cy}) — {}", parts.join(" · ")))
}
