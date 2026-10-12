//! Border overlay: realm wash + dashed realm lines + dotted city polygons.

use crate::settlements_draw::{Rgba, WorldBounds};
use crate::world::borders::{CityBorder, Realm, RealmBorder, TerritoryMap};
use tiny_skia::{LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, StrokeDash, Transform};

const WASH_ALPHA: u8 = 38;
const CITY_LINE: Rgba = Rgba::rgba(0x48, 0x32, 0x5c, 220);

const REALM_PALETTE: [[u8; 3]; 12] = [
    [0xa0, 0x30, 0x40], // burgundy
    [0x30, 0x48, 0x8c], // navy
    [0x58, 0x78, 0x30], // olive
    [0xb4, 0x78, 0x28], // ochre
    [0x64, 0x3c, 0x78], // purple
    [0x28, 0x6e, 0x6e], // teal
    [0x8c, 0x40, 0x28], // rust
    [0x3c, 0x5c, 0x8c], // steel blue
    [0x6e, 0x68, 0x28], // moss
    [0x78, 0x38, 0x58], // wine
    [0x2c, 0x6c, 0x48], // pine
    [0x8c, 0x5c, 0x2c], // copper
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderDrawMode {
    /// Global: realm wash + dashed realm borders.
    Realms,
    /// Region: realms + city borders.
    RealmsAndCities,
    /// Chunk: city borders only.
    Cities,
}

pub fn realm_color(realm_id: usize) -> Rgba {
    let [r, g, b] = REALM_PALETTE[realm_id % REALM_PALETTE.len()];
    Rgba::rgb(r, g, b)
}

pub fn draw_borders(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    bounds: &WorldBounds,
    territory: &TerritoryMap,
    realms: &[Realm],
    realm_borders: &[RealmBorder],
    city_borders: &[CityBorder],
    mode: BorderDrawMode,
) {
    if width == 0 || height == 0 || bounds.span <= 0.0 {
        return;
    }

    let draw_realms = matches!(
        mode,
        BorderDrawMode::Realms | BorderDrawMode::RealmsAndCities
    );
    let draw_cities = matches!(
        mode,
        BorderDrawMode::RealmsAndCities | BorderDrawMode::Cities
    );

    if draw_realms && !realms.is_empty() {
        draw_territory_wash(pixels, width, height, bounds, territory);
        let line_w = match mode {
            BorderDrawMode::Realms => (width as f32 * 0.0045).clamp(1.8, 3.2),
            _ => (width as f32 * 0.0035).clamp(1.4, 2.6),
        };
        for border in realm_borders {
            let base = realm_color(border.realm_a);
            let color = Rgba::rgba(base.r, base.g, base.b, 230);
            let pts = world_polyline_to_screen(&border.polyline, bounds, width, height);
            stroke_dashed(pixels, width, height, &pts, line_w, color, &[7.0, 5.0]);
        }
    }

    if draw_cities {
        let line_w = (width as f32 * 0.0022).clamp(1.0, 1.8);
        for border in city_borders {
            let pts = world_polyline_to_screen(&border.polygon, bounds, width, height);
            if pts.len() < 3 {
                continue;
            }
            let mut closed = pts;
            if closed.first() != closed.last() {
                let first = closed[0];
                closed.push(first);
            }
            stroke_dashed(pixels, width, height, &closed, line_w, CITY_LINE, &[1.5, 3.0]);
        }
    }
}

fn draw_territory_wash(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    bounds: &WorldBounds,
    territory: &TerritoryMap,
) {
    if territory.cols == 0 || territory.rows == 0 || territory.owners.is_empty() {
        return;
    }
    let cols = territory.cols as usize;
    let rows = territory.rows as usize;
    let step = territory.step;
    if step <= 0.0 {
        return;
    }

    for py in 0..height {
        let wy = bounds.y0 + ((py as f32 + 0.5) / height as f32) * bounds.span;
        let gy = (wy / step).floor() as isize;
        if gy < 0 || gy >= rows as isize {
            continue;
        }
        for px in 0..width {
            let wx = bounds.x0 + ((px as f32 + 0.5) / width as f32) * bounds.span;
            let gx = (wx / step).floor() as isize;
            if gx < 0 || gx >= cols as isize {
                continue;
            }
            let idx = gy as usize * cols + gx as usize;
            let owner = territory.owners.get(idx).copied().unwrap_or(u16::MAX);
            if owner == u16::MAX {
                continue;
            }
            let c = realm_color(owner as usize);
            blend_pixel(
                pixels,
                width,
                height,
                px as i32,
                py as i32,
                Rgba::rgba(c.r, c.g, c.b, WASH_ALPHA),
            );
        }
    }
}

fn world_polyline_to_screen(
    points: &[crate::world::borders::BorderPoint],
    bounds: &WorldBounds,
    width: usize,
    height: usize,
) -> Vec<(f32, f32)> {
    points
        .iter()
        .map(|p| {
            let sx = ((p.x - bounds.x0) / bounds.span) * width as f32;
            let sy = ((p.y - bounds.y0) / bounds.span) * height as f32;
            (sx, sy)
        })
        .collect()
}

fn blend_pixel(pixels: &mut [u8], width: usize, height: usize, x: i32, y: i32, color: Rgba) {
    if x < 0 || y < 0 || (x as usize) >= width || (y as usize) >= height || color.a == 0 {
        return;
    }
    let i = ((y as usize) * width + (x as usize)) * 4;
    let sa = color.a as f32 / 255.0;
    let inv = 1.0 - sa;
    pixels[i] = (color.r as f32 * sa + pixels[i] as f32 * inv).round() as u8;
    pixels[i + 1] = (color.g as f32 * sa + pixels[i + 1] as f32 * inv).round() as u8;
    pixels[i + 2] = (color.b as f32 * sa + pixels[i + 2] as f32 * inv).round() as u8;
}

fn stroke_dashed(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    pts: &[(f32, f32)],
    line_w: f32,
    color: Rgba,
    dash: &[f32],
) {
    if pts.len() < 2 || line_w <= 0.0 || color.a == 0 || width == 0 || height == 0 {
        return;
    }

    let pad = line_w * 0.5 + 2.0;
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for &(x, y) in pts {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    min_x = (min_x - pad).floor().max(0.0);
    min_y = (min_y - pad).floor().max(0.0);
    max_x = (max_x + pad).ceil().min(width as f32);
    max_y = (max_y + pad).ceil().min(height as f32);
    let pm_w = (max_x - min_x).ceil() as u32;
    let pm_h = (max_y - min_y).ceil() as u32;
    if pm_w == 0 || pm_h == 0 {
        return;
    }

    let mut pb = PathBuilder::new();
    pb.move_to(pts[0].0 - min_x, pts[0].1 - min_y);
    for &(x, y) in &pts[1..] {
        pb.line_to(x - min_x, y - min_y);
    }
    let Some(path) = pb.finish() else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = true;

    let mut stroke = Stroke::default();
    stroke.width = line_w;
    stroke.line_cap = LineCap::Round;
    stroke.line_join = LineJoin::Round;
    stroke.dash = StrokeDash::new(dash.to_vec(), 0.0);

    let Some(mut pixmap) = Pixmap::new(pm_w, pm_h) else {
        return;
    };
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

    let ox = min_x as i32;
    let oy = min_y as i32;
    let data = pixmap.data();
    for py in 0..pm_h as i32 {
        for px in 0..pm_w as i32 {
            let si = ((py as u32 * pm_w + px as u32) * 4) as usize;
            let a = data[si + 3];
            if a == 0 {
                continue;
            }
            let (r, g, b) = if a == 255 {
                (data[si], data[si + 1], data[si + 2])
            } else {
                let af = a as f32;
                (
                    ((data[si] as f32 * 255.0) / af).round().clamp(0.0, 255.0) as u8,
                    ((data[si + 1] as f32 * 255.0) / af).round().clamp(0.0, 255.0) as u8,
                    ((data[si + 2] as f32 * 255.0) / af).round().clamp(0.0, 255.0) as u8,
                )
            };
            blend_pixel(
                pixels,
                width,
                height,
                ox + px,
                oy + py,
                Rgba::rgba(r, g, b, a),
            );
        }
    }
}
