//! Port of former src/map/settlements.ts (canvas → RGBA).

use crate::world::settlement::{
    District, DistrictKind, Road, RoadApproach, RoadCrossing, RoadKind, RoadPoint, RoadSurface,
    Settlement, SettlementKind,
};
use crate::world::types::{TerrainGrid, TileType};

// ── Public types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadDetail {
    Main,
    Region,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementDrawStyle {
    Marker,
    Plan,
}

#[derive(Debug, Clone, Copy)]
pub struct WorldBounds {
    pub x0: f32,
    pub y0: f32,
    pub span: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettlementHit {
    pub settlement_index: usize,
    pub district_index: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct SettlementInfo {
    pub label: &'static str,
    pub pop_min: u32,
    pub pop_max: u32,
    pub fill: Rgba,
    pub stroke: Rgba,
    pub min_px: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct DistrictInfo {
    pub label: &'static str,
    pub fill: Rgba,
}

#[derive(Debug, Clone, Copy)]
pub struct RoadInfo {
    pub label: &'static str,
    pub stroke: Rgba,
}

#[derive(Debug, Clone, Copy)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

}

// ── Legend constants (order matches TS) ────────────────────────────────────

pub const SETTLEMENT_KIND_ORDER: [SettlementKind; 4] = [
    SettlementKind::City,
    SettlementKind::Town,
    SettlementKind::Village,
    SettlementKind::Hamlet,
];

pub const SETTLEMENT_INFO: [SettlementInfo; 4] = [
    SettlementInfo {
        label: "Miasto",
        pop_min: 8_000,
        pop_max: 50_000,
        fill: Rgba::rgb(0xc9, 0xa3, 0x6a),
        stroke: Rgba::rgb(0x3a, 0x2a, 0x08),
        min_px: 22.0,
    },
    SettlementInfo {
        label: "Miasteczko",
        pop_min: 1_200,
        pop_max: 8_000,
        fill: Rgba::rgb(0xd4, 0xa0, 0x70),
        stroke: Rgba::rgb(0x3a, 0x22, 0x0c),
        min_px: 14.0,
    },
    SettlementInfo {
        label: "Wieś",
        pop_min: 150,
        pop_max: 1_200,
        fill: Rgba::rgb(0xe2, 0xd0, 0xa8),
        stroke: Rgba::rgb(0x2a, 0x24, 0x18),
        min_px: 9.0,
    },
    SettlementInfo {
        label: "Wioska",
        pop_min: 20,
        pop_max: 150,
        fill: Rgba::rgb(0xd8, 0xd0, 0xc0),
        stroke: Rgba::rgb(0x22, 0x22, 0x22),
        min_px: 6.0,
    },
];

pub const DISTRICT_KIND_ORDER: [DistrictKind; 9] = [
    DistrictKind::Center,
    DistrictKind::Market,
    DistrictKind::Craft,
    DistrictKind::Port,
    DistrictKind::Temple,
    DistrictKind::Noble,
    DistrictKind::Residential,
    DistrictKind::Forest,
    DistrictKind::Outskirts,
];

pub const DISTRICT_INFO: [DistrictInfo; 9] = [
    DistrictInfo {
        label: "Centrum / rynek",
        fill: Rgba::rgb(0x7a, 0x4e, 0x36),
    },
    DistrictInfo {
        label: "Handlowa",
        fill: Rgba::rgb(0xc9, 0xa2, 0x4a),
    },
    DistrictInfo {
        label: "Rzemieślnicza",
        fill: Rgba::rgb(0x8c, 0x5a, 0x3c),
    },
    DistrictInfo {
        label: "Portowa",
        fill: Rgba::rgb(0x6a, 0x84, 0x90),
    },
    DistrictInfo {
        label: "Świątynna",
        fill: Rgba::rgb(0xd4, 0xc8, 0xae),
    },
    DistrictInfo {
        label: "Zamożna",
        fill: Rgba::rgb(0xc4, 0xa0, 0x7a),
    },
    DistrictInfo {
        label: "Mieszkaniowa",
        fill: Rgba::rgb(0xcb, 0xb8, 0x9a),
    },
    DistrictInfo {
        label: "Leśna / park",
        fill: Rgba::rgb(0x5d, 0x7a, 0x48),
    },
    DistrictInfo {
        label: "Obrzeża",
        fill: Rgba::rgb(0xb5, 0x9a, 0x72),
    },
];

pub const ROAD_KIND_ORDER: [RoadKind; 3] = [
    RoadKind::Highway,
    RoadKind::Secondary,
    RoadKind::Local,
];

pub const ROAD_INFO: [RoadInfo; 3] = [
    RoadInfo {
        label: "Szlak główny",
        stroke: Rgba::rgb(0xc4, 0xa3, 0x6a),
    },
    RoadInfo {
        label: "Droga boczna",
        stroke: Rgba::rgb(0x8a, 0x6a, 0x48),
    },
    RoadInfo {
        label: "Droga poboczna",
        stroke: Rgba::rgb(0x6a, 0x53, 0x40),
    },
];

const STREET_BED: Rgba = Rgba::rgb(0xea, 0xd9, 0xb0);

const DRAW_ORDER: [SettlementKind; 4] = [
    SettlementKind::Hamlet,
    SettlementKind::Village,
    SettlementKind::Town,
    SettlementKind::City,
];

const TAU: f32 = std::f32::consts::TAU;

// ── Lookup helpers ─────────────────────────────────────────────────────────

pub fn settlement_info(kind: SettlementKind) -> SettlementInfo {
    match kind {
        SettlementKind::City => SETTLEMENT_INFO[0],
        SettlementKind::Town => SETTLEMENT_INFO[1],
        SettlementKind::Village => SETTLEMENT_INFO[2],
        SettlementKind::Hamlet => SETTLEMENT_INFO[3],
    }
}

pub fn district_info(kind: DistrictKind) -> DistrictInfo {
    match kind {
        DistrictKind::Center => DISTRICT_INFO[0],
        DistrictKind::Market => DISTRICT_INFO[1],
        DistrictKind::Craft => DISTRICT_INFO[2],
        DistrictKind::Port => DISTRICT_INFO[3],
        DistrictKind::Temple => DISTRICT_INFO[4],
        DistrictKind::Noble => DISTRICT_INFO[5],
        DistrictKind::Residential => DISTRICT_INFO[6],
        DistrictKind::Forest => DISTRICT_INFO[7],
        DistrictKind::Outskirts => DISTRICT_INFO[8],
    }
}

pub fn road_info(kind: RoadKind) -> RoadInfo {
    match kind {
        RoadKind::Highway => ROAD_INFO[0],
        RoadKind::Secondary => ROAD_INFO[1],
        RoadKind::Local => ROAD_INFO[2],
    }
}

// ── Pixel primitives ───────────────────────────────────────────────────────

fn blend_pixel(pixels: &mut [u8], width: usize, height: usize, x: i32, y: i32, color: Rgba) {
    if x < 0 || y < 0 || (x as usize) >= width || (y as usize) >= height {
        return;
    }
    if color.a == 0 {
        return;
    }
    let i = ((y as usize) * width + (x as usize)) * 4;
    if color.a == 255 {
        pixels[i] = color.r;
        pixels[i + 1] = color.g;
        pixels[i + 2] = color.b;
        pixels[i + 3] = 255;
        return;
    }
    let sa = color.a as f32 / 255.0;
    let da = pixels[i + 3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a < 1e-6 {
        pixels[i] = 0;
        pixels[i + 1] = 0;
        pixels[i + 2] = 0;
        pixels[i + 3] = 0;
        return;
    }
    let inv = 1.0 - sa;
    pixels[i] = ((color.r as f32 * sa + pixels[i] as f32 * da * inv) / out_a).round() as u8;
    pixels[i + 1] =
        ((color.g as f32 * sa + pixels[i + 1] as f32 * da * inv) / out_a).round() as u8;
    pixels[i + 2] =
        ((color.b as f32 * sa + pixels[i + 2] as f32 * da * inv) / out_a).round() as u8;
    pixels[i + 3] = (out_a * 255.0).round() as u8;
}

fn clear_alpha(pixels: &mut [u8], width: usize, x: usize, y: usize) {
    let i = (y * width + x) * 4;
    pixels[i + 3] = 0;
}

fn fill_disk(pixels: &mut [u8], width: usize, height: usize, cx: f32, cy: f32, radius: f32, color: Rgba) {
    if radius <= 0.0 {
        return;
    }
    let r2 = radius * radius;
    let x0 = (cx - radius).floor() as i32;
    let x1 = (cx + radius).ceil() as i32;
    let y0 = (cy - radius).floor() as i32;
    let y1 = (cy + radius).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= r2 {
                blend_pixel(pixels, width, height, x, y, color);
            }
        }
    }
}

/// Thick line segment as a stadium (rectangle + round ends via disks at joints).
fn stroke_segment(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    half: f32,
    color: Rgba,
) {
    let dx = bx - ax;
    let dy = by - ay;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        fill_disk(pixels, width, height, ax, ay, half, color);
        return;
    }
    // Cover the capsule with a bounding box scan.
    let pad = half + 1.0;
    let min_x = ax.min(bx) - pad;
    let max_x = ax.max(bx) + pad;
    let min_y = ay.min(by) - pad;
    let max_y = ay.max(by) + pad;
    let x0 = min_x.floor() as i32;
    let x1 = max_x.ceil() as i32;
    let y0 = min_y.floor() as i32;
    let y1 = max_y.ceil() as i32;
    let half2 = half * half;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let t = ((px - ax) * dx + (py - ay) * dy) / (len * len);
            let t = t.clamp(0.0, 1.0);
            let qx = ax + dx * t;
            let qy = ay + dy * t;
            let ddx = px - qx;
            let ddy = py - qy;
            if ddx * ddx + ddy * ddy <= half2 {
                blend_pixel(pixels, width, height, x, y, color);
            }
        }
    }
}

fn stroke_polyline(pixels: &mut [u8], width: usize, height: usize, pts: &[(f32, f32)], line_w: f32, color: Rgba) {
    if pts.is_empty() || line_w <= 0.0 {
        return;
    }
    let half = line_w * 0.5;
    for w in pts.windows(2) {
        stroke_segment(
            pixels,
            width,
            height,
            w[0].0,
            w[0].1,
            w[1].0,
            w[1].1,
            half,
            color,
        );
    }
    // Round caps / joins (canvas lineCap/lineJoin = round).
    for &(x, y) in pts {
        fill_disk(pixels, width, height, x, y, half, color);
    }
}

fn fill_poly(pixels: &mut [u8], width: usize, height: usize, pts: &[(f32, f32)], color: Rgba) {
    if pts.len() < 3 {
        return;
    }
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for &(_, y) in pts {
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let y0 = min_y.floor() as i32;
    let y1 = max_y.ceil() as i32;
    let n = pts.len();
    for y in y0..=y1 {
        let fy = y as f32 + 0.5;
        let mut xs: Vec<f32> = Vec::new();
        for i in 0..n {
            let (xi, yi) = pts[i];
            let (xj, yj) = pts[(i + n - 1) % n];
            if (yi > fy) == (yj > fy) {
                continue;
            }
            let t = (fy - yi) / (yj - yi);
            xs.push(xi + (xj - xi) * t);
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut k = 0;
        while k + 1 < xs.len() {
            let x_start = xs[k].ceil() as i32;
            let x_end = xs[k + 1].floor() as i32;
            for x in x_start..=x_end {
                blend_pixel(pixels, width, height, x, y, color);
            }
            k += 2;
        }
    }
}

fn stroke_poly(pixels: &mut [u8], width: usize, height: usize, pts: &[(f32, f32)], line_w: f32, color: Rgba) {
    if pts.len() < 2 {
        return;
    }
    let mut closed: Vec<(f32, f32)> = pts.to_vec();
    if closed.first() != closed.last() {
        closed.push(closed[0]);
    }
    stroke_polyline(pixels, width, height, &closed, line_w, color);
}

fn blit_overlay(
    dst: &mut [u8],
    dst_w: usize,
    dst_h: usize,
    src: &[u8],
    src_w: usize,
    src_h: usize,
    ox: i32,
    oy: i32,
) {
    for y in 0..src_h {
        for x in 0..src_w {
            let si = (y * src_w + x) * 4;
            let a = src[si + 3];
            if a == 0 {
                continue;
            }
            blend_pixel(
                dst,
                dst_w,
                dst_h,
                ox + x as i32,
                oy + y as i32,
                Rgba::rgba(src[si], src[si + 1], src[si + 2], a),
            );
        }
    }
}

// ── Hash / geometry helpers ────────────────────────────────────────────────

fn u32hash(n: i32) -> u32 {
    let mut x = n.wrapping_mul(1_597_334_677);
    let xu = x as u32;
    x = (xu ^ (xu >> 16)) as i32;
    x = x.wrapping_mul(2_246_822_519u32 as i32);
    x as u32
}

fn unit(x: f32, y: f32, i: i32) -> f32 {
    let xi = (x * 1000.0) as i32;
    let yi = (y * 1000.0) as i32;
    let sum = (u32hash(xi) as i32)
        .wrapping_add((u32hash(yi) as i32).wrapping_mul(3))
        .wrapping_add(i);
    u32hash(sum) as f32 / 4_294_967_296.0
}

pub fn settlement_footprint_radius(settlement: &Settlement, px_per_world: f32) -> f32 {
    settlement_info(settlement.kind)
        .min_px
        .max(settlement.radius * px_per_world)
}

fn settlement_world_radius(settlement: &Settlement) -> f32 {
    let mut max_r = 1.0f32;
    for p in plan_outline(settlement) {
        let r = (p.0 * p.0 + p.1 * p.1).sqrt();
        if r > max_r {
            max_r = r;
        }
    }
    settlement.radius * max_r * 1.06
}

fn settlement_intersects_bounds(settlement: &Settlement, bounds: &WorldBounds) -> bool {
    let r = settlement_world_radius(settlement);
    !(settlement.x + r < bounds.x0
        || settlement.x - r > bounds.x0 + bounds.span
        || settlement.y + r < bounds.y0
        || settlement.y - r > bounds.y0 + bounds.span)
}

struct MarkerSize {
    half: f32,
    hit: f32,
}

fn marker_size(kind: SettlementKind) -> MarkerSize {
    match kind {
        SettlementKind::City => MarkerSize {
            half: 4.5,
            hit: 8.0,
        },
        _ => MarkerSize {
            half: 2.4,
            hit: 6.0,
        },
    }
}

fn footprint_frame() -> (f32, f32) {
    (1.0, 0.0) // stretch, rot
}

fn to_local_norm(px: f32, py: f32, sx: f32, sy: f32, r: f32) -> (f32, f32) {
    let (stretch, rot) = footprint_frame();
    let dx = px - sx;
    let dy = py - sy;
    let c = (-rot).cos();
    let s = (-rot).sin();
    let lx = (dx * c - dy * s) / stretch;
    let ly = dx * s + dy * c;
    (lx / r, ly / r)
}

fn norm_angle(a: f32) -> f32 {
    let mut x = a % TAU;
    if x < 0.0 {
        x += TAU;
    }
    x
}

fn outer_scale(settlement: &Settlement) -> f32 {
    let mut m = 0.0f32;
    for d in &settlement.districts {
        if d.outer > m {
            m = d.outer;
        }
    }
    if m > 0.01 {
        m
    } else {
        1.0
    }
}

fn is_planned(settlement: &Settlement) -> bool {
    settlement.kind == SettlementKind::City && unit(settlement.x, settlement.y, 12) < 0.38
}

fn road_approaches(settlement: &Settlement) -> Vec<RoadApproach> {
    if !settlement.road_approaches.is_empty() {
        return settlement.road_approaches.clone();
    }
    road_axes_fallback(settlement)
        .into_iter()
        .map(|angle| RoadApproach {
            angle,
            kind: RoadKind::Secondary,
        })
        .collect()
}

fn road_axes_fallback(settlement: &Settlement) -> Vec<f32> {
    let base = unit(settlement.x, settlement.y, 13) * TAU;
    match settlement.kind {
        SettlementKind::Hamlet => vec![base],
        SettlementKind::Village => {
            if unit(settlement.x, settlement.y, 14) < 0.5 {
                vec![base]
            } else {
                vec![
                    base,
                    base + std::f32::consts::PI
                        * (0.1 + unit(settlement.x, settlement.y, 15) * 0.2),
                ]
            }
        }
        _ => {
            let mut axes = vec![
                base,
                base + std::f32::consts::PI
                    * (0.4 + unit(settlement.x, settlement.y, 14) * 0.32),
            ];
            if settlement.kind == SettlementKind::City
                && unit(settlement.x, settlement.y, 16) < 0.58
            {
                axes.push(
                    base + TAU * (0.63 + unit(settlement.x, settlement.y, 17) * 0.1),
                );
            }
            axes
        }
    }
}

fn road_axes(settlement: &Settlement) -> Vec<f32> {
    let approaches = road_approaches(settlement);
    let hw: Vec<f32> = approaches
        .iter()
        .filter(|a| a.kind == RoadKind::Highway)
        .map(|a| a.angle)
        .collect();
    if !hw.is_empty() {
        hw
    } else {
        approaches.iter().map(|a| a.angle).collect()
    }
}

fn lobe_weight(kind: RoadKind) -> f32 {
    match kind {
        RoadKind::Highway => 1.55,
        RoadKind::Secondary => 1.05,
        RoadKind::Local => 0.55,
    }
}

fn core_pos(settlement: &Settlement) -> (f32, f32) {
    (settlement.core_dx, settlement.core_dy)
}

fn outline_r(settlement: &Settlement, angle: f32) -> f32 {
    let a = norm_angle(angle);
    let axes = road_axes(settlement);
    let planned = is_planned(settlement);
    let elong_a = axes.first().copied().unwrap_or(0.0);
    let da = a - elong_a;
    let c = da.cos();
    let s = da.sin();
    let elong = if planned {
        1.1 + unit(settlement.x, settlement.y, 18) * 0.12
    } else if settlement.kind == SettlementKind::Hamlet {
        1.2 + unit(settlement.x, settlement.y, 18) * 0.22
    } else {
        1.48 + unit(settlement.x, settlement.y, 18) * 0.5
    };
    let perp = if planned {
        0.9
    } else {
        0.5 + unit(settlement.x, settlement.y, 19) * 0.22
    };
    let mut r = (elong * perp) / ((perp * c).powi(2) + (elong * s).powi(2)).sqrt();
    r *= if planned { 0.76 } else { 0.68 };
    let approaches = road_approaches(settlement);
    if planned {
        let sq = 0.8 / c.abs().max(s.abs()).max(0.25);
        r = r * 0.38 + sq.min(1.22) * 0.62;
        for ap in &approaches {
            let k = (a - ap.angle).cos();
            if k > 0.2 {
                r += 0.08 * lobe_weight(ap.kind) * k * k;
            }
        }
    } else {
        for (i, ap) in approaches.iter().enumerate() {
            let k = (a - ap.angle).cos();
            if k > 0.1 {
                let lobe = (0.16 + unit(settlement.x, settlement.y, 21 + i as i32) * 0.14)
                    * lobe_weight(ap.kind);
                r += lobe * k * k;
            }
        }
    }
    let aq = ((a / TAU) * 16.0) as i32;
    let mut jitter = 0.9 + unit(settlement.x, settlement.y, 30 + aq) * 0.22;
    for ap in &approaches {
        let k = (a - ap.angle).cos();
        if k > 0.72 {
            let w = (k - 0.72) / 0.28;
            jitter = jitter * (1.0 - w) + w;
        }
    }
    r *= jitter;
    r.clamp(0.36, 1.72)
}

fn plan_outline(settlement: &Settlement) -> Vec<(f32, f32)> {
    let n = match settlement.kind {
        SettlementKind::City => 22,
        SettlementKind::Town => 18,
        _ => 14,
    };
    let rot = road_axes(settlement).first().copied().unwrap_or(0.0);
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let a = rot + (i as f32 / n as f32) * TAU;
        let r = outline_r(settlement, a);
        pts.push((a.cos() * r, a.sin() * r));
    }
    pts
}

fn district_site(settlement: &Settlement, district: &District) -> (f32, f32) {
    let c = core_pos(settlement);
    if district.span >= TAU - 1e-3 {
        return c;
    }
    let a = district.a0 + district.span * 0.5;
    let r_out = outline_r(settlement, a);
    let mut t = (district.inner + district.outer) * 0.5 / outer_scale(settlement);
    t = (t * 0.85).clamp(0.16, 0.84);
    if matches!(district.kind, DistrictKind::Forest | DistrictKind::Outskirts) {
        t += 0.1;
    }
    if district.kind == DistrictKind::Center {
        t = 0.0;
    }
    t = t.min(0.88);
    (c.0 + a.cos() * r_out * t, c.1 + a.sin() * r_out * t)
}

struct CityBlock {
    poly: Vec<(f32, f32)>,
    district_index: Option<usize>,
}

fn poly_area(poly: &[(f32, f32)]) -> f32 {
    let mut a = 0.0f32;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        a += poly[j].0 * poly[i].1 - poly[i].0 * poly[j].1;
        j = i;
    }
    a * 0.5
}

fn centroid(poly: &[(f32, f32)]) -> (f32, f32) {
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut a = 0.0f32;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let c = poly[j].0 * poly[i].1 - poly[i].0 * poly[j].1;
        x += (poly[j].0 + poly[i].0) * c;
        y += (poly[j].1 + poly[i].1) * c;
        a += c;
        j = i;
    }
    if a.abs() < 1e-10 {
        return poly[0];
    }
    (x / (3.0 * a), y / (3.0 * a))
}

fn split_poly_by_line(
    poly: &[(f32, f32)],
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
) -> (Vec<(f32, f32)>, Vec<(f32, f32)>) {
    let nx = -(by - ay);
    let ny = bx - ax;
    let side = |p: (f32, f32)| nx * (p.0 - ax) + ny * (p.1 - ay);
    let hit = |p: (f32, f32), q: (f32, f32)| {
        let s1 = side(p);
        let s2 = side(q);
        let t = s1 / (if (s1 - s2).abs() < 1e-9 {
            1e-9_f32.copysign(s1 - s2)
        } else {
            s1 - s2
        });
        (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t)
    };
    let mut left = Vec::new();
    let mut right = Vec::new();
    let n = poly.len();
    for i in 0..n {
        let cur = poly[i];
        let prev = poly[(i + n - 1) % n];
        let cin = side(cur) >= 0.0;
        let pin = side(prev) >= 0.0;
        if cin {
            if !pin {
                left.push(hit(prev, cur));
            }
            left.push(cur);
        } else if pin {
            left.push(hit(prev, cur));
        }
        if !cin {
            if pin {
                right.push(hit(prev, cur));
            }
            right.push(cur);
        } else if !pin {
            right.push(hit(prev, cur));
        }
    }
    (left, right)
}

fn inset_poly(poly: &[(f32, f32)], amt: f32) -> Vec<(f32, f32)> {
    let c = centroid(poly);
    poly.iter()
        .map(|&(px, py)| {
            let dx = px - c.0;
            let dy = py - c.1;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 1e-6 {
                return (px, py);
            }
            let k = ((d - amt) / d).max(0.15);
            (c.0 + dx * k, c.1 + dy * k)
        })
        .collect()
}

struct SplitLine {
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
}

fn push_split_line(lines: &mut Vec<SplitLine>, angle: f32, ox: f32, oy: f32) {
    let ux = angle.cos();
    let uy = angle.sin();
    lines.push(SplitLine {
        ax: ox - ux * 2.4,
        ay: oy - uy * 2.4,
        bx: ox + ux * 2.4,
        by: oy + uy * 2.4,
    });
}

fn street_splits(settlement: &Settlement) -> Vec<SplitLine> {
    let planned = is_planned(settlement);
    let approaches = road_approaches(settlement);
    let main = approaches
        .first()
        .map(|a| a.angle)
        .unwrap_or_else(|| road_axes(settlement).first().copied().unwrap_or(0.0));
    let mut lines = Vec::new();
    for ap in &approaches {
        push_split_line(&mut lines, ap.angle, 0.0, 0.0);
    }
    if approaches.is_empty() {
        push_split_line(&mut lines, main, 0.0, 0.0);
    }

    let (extra_para, extra_perp) = match settlement.kind {
        SettlementKind::City => (2, 2),
        SettlementKind::Town => (1, 2),
        SettlementKind::Village => (1, 1),
        SettlementKind::Hamlet => (0, 1),
    };
    let jitter = if planned { 0.04 } else { 0.16 };
    let add_offsets = |lines: &mut Vec<SplitLine>, base: f32, count: i32, salt: i32| {
        for i in 0..count {
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            let rank = (i / 2) + 1;
            let off = sign
                * rank as f32
                * (0.26 + unit(settlement.x, settlement.y, salt + i) * 0.12);
            let ang =
                base + (unit(settlement.x, settlement.y, salt + 20 + i) - 0.5) * jitter;
            let px = -base.sin();
            let py = base.cos();
            push_split_line(lines, ang, px * off, py * off);
        }
    };
    add_offsets(&mut lines, main, extra_para, 80);
    add_offsets(
        &mut lines,
        main + std::f32::consts::FRAC_PI_2,
        extra_perp,
        120,
    );
    lines
}

fn settlement_blocks(settlement: &Settlement) -> Vec<CityBlock> {
    let splits = street_splits(settlement);
    let mut polys: Vec<Vec<(f32, f32)>> = vec![plan_outline(settlement)];
    for line in &splits {
        let mut next = Vec::new();
        for poly in &polys {
            if poly.len() < 3 {
                continue;
            }
            let (a, b) = split_poly_by_line(poly, line.ax, line.ay, line.bx, line.by);
            if a.len() >= 3 && poly_area(&a).abs() > 0.01 {
                next.push(a);
            }
            if b.len() >= 3 && poly_area(&b).abs() > 0.01 {
                next.push(b);
            }
        }
        if !next.is_empty() {
            polys = next;
        }
    }
    let inset_amt = match settlement.kind {
        SettlementKind::City => 0.042,
        SettlementKind::Town => 0.048,
        _ => 0.055,
    };
    let center_idx_opt = settlement
        .districts
        .iter()
        .position(|d| d.kind == DistrictKind::Center);
    let mut blocks = Vec::new();
    let mut center_block = usize::MAX;
    let mut center_best = f32::INFINITY;
    for poly in &polys {
        if poly_area(poly).abs() < 0.014 {
            continue;
        }
        let c = centroid(poly);
        let inset = inset_poly(poly, inset_amt);
        if inset.len() < 3 || poly_area(&inset).abs() < 0.006 {
            continue;
        }
        let mut district_index: Option<usize> = None;
        let mut best = f32::INFINITY;
        for (di, d) in settlement.districts.iter().enumerate() {
            if d.kind == DistrictKind::Center {
                continue;
            }
            let site = district_site(settlement, d);
            let dist = (site.0 - c.0).powi(2) + (site.1 - c.1).powi(2);
            if dist < best {
                best = dist;
                district_index = Some(di);
            }
        }
        let d0 = c.0 * c.0 + c.1 * c.1;
        if center_idx_opt.is_some() && d0 < center_best {
            center_best = d0;
            center_block = blocks.len();
        }
        blocks.push(CityBlock {
            poly: inset,
            district_index,
        });
    }
    if center_block != usize::MAX {
        if let Some(ci) = center_idx_opt {
            if center_best < 0.14 {
                blocks[center_block].district_index = Some(ci);
            }
        }
    }
    blocks
}

fn point_in_poly(x: f32, y: f32, pts: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let n = pts.len();
    let mut j = n - 1;
    for i in 0..n {
        let yi = pts[i].1;
        let yj = pts[j].1;
        if (yi > y) == (yj > y) {
            j = i;
            continue;
        }
        let xi = pts[i].0;
        let xj = pts[j].0;
        if x < ((xj - xi) * (y - yi)) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn district_at(settlement: &Settlement, nx: f32, ny: f32) -> Option<usize> {
    for block in settlement_blocks(settlement) {
        if point_in_poly(nx, ny, &block.poly) {
            return block.district_index;
        }
    }
    if settlement.districts.is_empty() {
        None
    } else {
        Some(0)
    }
}

fn format_population(n: u32) -> String {
    let s = n.to_string();
    let bytes: Vec<u8> = s.bytes().collect();
    let mut out = String::new();
    let len = bytes.len();
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(b as char);
    }
    out
}

pub fn format_settlement_label(settlement: &Settlement, district: Option<&District>) -> String {
    let pop = format_population(settlement.population);
    if let Some(d) = district {
        format!("{} · {} · {} mieszk.", settlement.name, d.name, pop)
    } else {
        format!("{} · {} mieszk.", settlement.name, pop)
    }
}

fn is_water_biome(biome: TileType) -> bool {
    matches!(biome, TileType::Water | TileType::DeepWater)
}

fn biome_at_canvas(grid: &TerrainGrid, canvas_x: f32, canvas_y: f32) -> Option<TileType> {
    let gx = canvas_x.floor() as i32;
    let gy = canvas_y.floor() as i32;
    if gx < 0 || gy < 0 || gx as u32 >= grid.width || gy as u32 >= grid.height {
        return None;
    }
    Some(grid.cells[(gy as u32 * grid.width + gx as u32) as usize].biome)
}

fn local_to_canvas(
    local_x: f32,
    local_y: f32,
    sx: f32,
    sy: f32,
    rot: f32,
    stretch: f32,
) -> (f32, f32) {
    let lx = local_x * stretch;
    let ly = local_y;
    let c = rot.cos();
    let s = rot.sin();
    (sx + lx * c - ly * s, sy + lx * s + ly * c)
}

fn footprint_local_verts(settlement: &Settlement, r: f32) -> Vec<(f32, f32)> {
    plan_outline(settlement)
        .into_iter()
        .map(|(x, y)| (x * r, y * r))
        .collect()
}

fn pull_onto_land(
    verts: &[(f32, f32)],
    sx: f32,
    sy: f32,
    rot: f32,
    stretch: f32,
    grid: Option<&TerrainGrid>,
) -> Vec<(f32, f32)> {
    let Some(grid) = grid else {
        return verts.to_vec();
    };
    verts
        .iter()
        .map(|&(mut x, mut y)| {
            for _ in 0..10 {
                let (px, py) = local_to_canvas(x, y, sx, sy, rot, stretch);
                let biome = biome_at_canvas(grid, px, py);
                if biome.is_none() || !is_water_biome(biome.unwrap()) {
                    return (x, y);
                }
                x *= 0.8;
                y *= 0.8;
            }
            (x, y)
        })
        .collect()
}

fn punch_water(
    overlay: &mut [u8],
    ow: usize,
    oh: usize,
    ox: f32,
    oy: f32,
    grid: &TerrainGrid,
) {
    for y in 0..oh {
        for x in 0..ow {
            let i = (y * ow + x) * 4;
            if overlay[i + 3] == 0 {
                continue;
            }
            let biome = biome_at_canvas(grid, ox + x as f32, oy + y as f32);
            if let Some(b) = biome {
                if is_water_biome(b) {
                    clear_alpha(overlay, ow, x, y);
                }
            }
        }
    }
}

fn same_settlement(a: &Settlement, b: &Settlement) -> bool {
    a.x == b.x && a.y == b.y && a.kind == b.kind
}

fn scaled_block_poly(poly: &[(f32, f32)], scale: f32) -> Vec<(f32, f32)> {
    poly.iter().map(|&(x, y)| (x * scale, y * scale)).collect()
}

fn draw_footprint(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    sx: f32,
    sy: f32,
    settlement: &Settlement,
    px_per_world: f32,
    hovered: bool,
    hovered_district_index: Option<usize>,
    grid: Option<&TerrainGrid>,
) {
    let info = settlement_info(settlement.kind);
    let r = settlement_footprint_radius(settlement, px_per_world);
    let (stretch, rot) = footprint_frame();
    let verts = pull_onto_land(
        &footprint_local_verts(settlement, r),
        sx,
        sy,
        rot,
        stretch,
        grid,
    );
    let pad = (r * stretch.max(1.0) * 2.15 + 10.0).ceil() as i32;
    let size = (pad * 2) as usize;
    let mut overlay = vec![0u8; size * size * 4];

    // Local verts are relative to settlement center; overlay origin is (sx-pad, sy-pad).
    let to_overlay = |lx: f32, ly: f32| -> (f32, f32) {
        let (cx, cy) = local_to_canvas(lx, ly, 0.0, 0.0, rot, stretch);
        (pad as f32 + cx, pad as f32 + cy)
    };

    let outline_ov: Vec<(f32, f32)> = verts.iter().map(|&(x, y)| to_overlay(x, y)).collect();
    fill_poly(&mut overlay, size, size, &outline_ov, STREET_BED);

    // Clip further drawing to footprint: punch outside by only filling inside via clip simulation.
    // Canvas clips then fills street bed + blocks. We fill street bed as poly, then blocks
    // (which are inside). Outer stroke after punch is drawn on main buffer.

    let blocks = settlement_blocks(settlement);
    for block in &blocks {
        if block.poly.len() < 3 {
            continue;
        }
        let poly = scaled_block_poly(&block.poly, r);
        let poly_ov: Vec<(f32, f32)> = poly.iter().map(|&(x, y)| to_overlay(x, y)).collect();
        let fill = block
            .district_index
            .map(|di| district_info(settlement.districts[di].kind).fill)
            .unwrap_or(info.fill);
        fill_poly(&mut overlay, size, size, &poly_ov, fill);
    }

    if hovered {
        if let Some(hdi) = hovered_district_index {
            let hover_name = &settlement.districts[hdi].name;
            for block in &blocks {
                let Some(di) = block.district_index else {
                    continue;
                };
                if settlement.districts[di].name != *hover_name {
                    continue;
                }
                if block.poly.len() < 3 {
                    continue;
                }
                let poly = scaled_block_poly(&block.poly, r);
                let poly_ov: Vec<(f32, f32)> =
                    poly.iter().map(|&(x, y)| to_overlay(x, y)).collect();
                stroke_poly(
                    &mut overlay,
                    size,
                    size,
                    &poly_ov,
                    1.4,
                    Rgba::rgba(255, 255, 255, (0.85 * 255.0) as u8),
                );
            }
        } else {
            stroke_poly(
                &mut overlay,
                size,
                size,
                &outline_ov,
                1.4,
                Rgba::rgba(255, 255, 255, (0.85 * 255.0) as u8),
            );
        }
    }

    // Re-apply clip: clear overlay pixels outside the footprint outline (canvas clip).
    for y in 0..size {
        for x in 0..size {
            let i = (y * size + x) * 4;
            if overlay[i + 3] == 0 {
                continue;
            }
            let lx = x as f32 + 0.5 - pad as f32;
            let ly = y as f32 + 0.5 - pad as f32;
            // Inverse of identity stretch/rot: local == overlay-relative.
            let (ilx, ily) = {
                let c = (-rot).cos();
                let s = (-rot).sin();
                let rx = (lx * c - ly * s) / stretch;
                let ry = lx * s + ly * c;
                (rx, ry)
            };
            // Check against unscaled outline in local R-space: verts are already * R.
            if !point_in_poly(ilx, ily, &verts) {
                clear_alpha(&mut overlay, size, x, y);
            }
        }
    }

    if let Some(grid) = grid {
        punch_water(
            &mut overlay,
            size,
            size,
            sx - pad as f32,
            sy - pad as f32,
            grid,
        );
    }

    blit_overlay(
        pixels,
        width,
        height,
        &overlay,
        size,
        size,
        (sx - pad as f32).round() as i32,
        (sy - pad as f32).round() as i32,
    );

    // Outer stroke on main buffer (not punched).
    let outline_main: Vec<(f32, f32)> = verts
        .iter()
        .map(|&(x, y)| local_to_canvas(x, y, sx, sy, rot, stretch))
        .collect();
    stroke_poly(
        pixels,
        width,
        height,
        &outline_main,
        1.0,
        Rgba::rgba(48, 32, 18, (0.4 * 255.0) as u8),
    );
}

fn draw_city_marker(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    sx: f32,
    sy: f32,
    kind: SettlementKind,
    hovered: bool,
) {
    let info = settlement_info(kind);
    let half = marker_size(kind).half;
    let rot = std::f32::consts::FRAC_PI_4;
    let c = rot.cos();
    let s = rot.sin();
    // Diamond = axis-aligned square rotated 45°.
    let corners = [
        (-half, -half),
        (half, -half),
        (half, half),
        (-half, half),
    ];
    let poly: Vec<(f32, f32)> = corners
        .iter()
        .map(|&(lx, ly)| (sx + lx * c - ly * s, sy + lx * s + ly * c))
        .collect();
    fill_poly(pixels, width, height, &poly, info.fill);
    let lw = if kind == SettlementKind::City {
        1.4
    } else {
        1.0
    };
    stroke_poly(pixels, width, height, &poly, lw, info.stroke);
    if hovered {
        let h = half + 2.0;
        let hpoly: Vec<(f32, f32)> = [(-h, -h), (h, -h), (h, h), (-h, h)]
            .iter()
            .map(|&(lx, ly)| (sx + lx * c - ly * s, sy + lx * s + ly * c))
            .collect();
        stroke_poly(
            pixels,
            width,
            height,
            &hpoly,
            2.0,
            Rgba::rgba(255, 255, 255, (0.95 * 255.0) as u8),
        );
    }
}

fn road_in_view(points: &[RoadPoint], bounds: &WorldBounds) -> bool {
    if points.is_empty() {
        return false;
    }
    let mut min_x = points[0].x;
    let mut max_x = points[0].x;
    let mut min_y = points[0].y;
    let mut max_y = points[0].y;
    for p in points.iter().skip(1) {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    let pad = bounds.span * 0.08;
    !(max_x < bounds.x0 - pad
        || min_x > bounds.x0 + bounds.span + pad
        || max_y < bounds.y0 - pad
        || min_y > bounds.y0 + bounds.span + pad)
}

fn road_to_canvas(
    points: &[RoadPoint],
    bounds: &WorldBounds,
    canvas_w: f32,
    canvas_h: f32,
) -> Vec<(f32, f32)> {
    points
        .iter()
        .map(|p| {
            (
                ((p.x - bounds.x0) / bounds.span) * canvas_w,
                ((p.y - bounds.y0) / bounds.span) * canvas_h,
            )
        })
        .collect()
}

fn road_layer_width(kind: RoadKind, surface: RoadSurface, detail: RoadDetail) -> f32 {
    let base: f32 = match kind {
        RoadKind::Highway => 1.0,
        RoadKind::Secondary => 0.62,
        RoadKind::Local => 0.38,
    };
    let scale: f32 = match detail {
        RoadDetail::Main => 2.3,
        RoadDetail::Region => 2.5,
        RoadDetail::Close => 4.2,
    };
    let surface_mul: f32 = match surface {
        RoadSurface::Paved => 1.0,
        RoadSurface::Packed => 0.88,
        RoadSurface::Dirt => 0.72,
    };
    (scale * base * surface_mul).max(0.7)
}

fn paint_bridge_marks(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    road: &Road,
    bounds: &WorldBounds,
    detail: RoadDetail,
) {
    let has_bridge = road
        .points
        .iter()
        .any(|p| p.crossing == Some(RoadCrossing::Bridge));
    if !has_bridge {
        return;
    }
    let across_world = match detail {
        RoadDetail::Close => 3.2,
        RoadDetail::Region => 2.6,
        RoadDetail::Main => 2.0,
    };
    let canvas_w = width as f32;
    let canvas_h = height as f32;
    let across = (across_world * canvas_w) / bounds.span;
    for i in 0..road.points.len() {
        let p = &road.points[i];
        if p.crossing != Some(RoadCrossing::Bridge) {
            continue;
        }
        let prev = &road.points[i.saturating_sub(1).max(0)];
        let next = &road.points[(i + 1).min(road.points.len() - 1)];
        let dx = next.x - prev.x;
        let dy = next.y - prev.y;
        let dist = (dx * dx + dy * dy).sqrt().max(1.0);
        let nx = -dy / dist;
        let ny = dx / dist;
        let to_c = |x: f32, y: f32| {
            (
                ((x - bounds.x0) / bounds.span) * canvas_w,
                ((y - bounds.y0) / bounds.span) * canvas_h,
            )
        };
        let a = to_c(prev.x, prev.y);
        let b = to_c(next.x, next.y);
        let deck = match detail {
            RoadDetail::Close => Rgba::rgba(96, 72, 40, (0.95 * 255.0) as u8),
            _ => Rgba::rgba(86, 64, 36, (0.88 * 255.0) as u8),
        };
        let deck_w = if detail == RoadDetail::Close { 3.2 } else { 2.4 };
        stroke_polyline(pixels, width, height, &[a, b], deck_w, deck);
        let sx = ((p.x - bounds.x0) / bounds.span) * canvas_w;
        let sy = ((p.y - bounds.y0) / bounds.span) * canvas_h;
        let tie = match detail {
            RoadDetail::Close => Rgba::rgba(42, 28, 14, (0.95 * 255.0) as u8),
            _ => Rgba::rgba(40, 28, 14, (0.85 * 255.0) as u8),
        };
        let tie_w = if detail == RoadDetail::Close { 2.0 } else { 1.5 };
        stroke_polyline(
            pixels,
            width,
            height,
            &[(sx - nx * across, sy - ny * across), (sx + nx * across, sy + ny * across)],
            tie_w,
            tie,
        );
    }
}

fn punch_road_water(
    overlay: &mut [u8],
    ow: usize,
    oh: usize,
    ox: f32,
    oy: f32,
    grid: &TerrainGrid,
    bridge_canvas_pts: &[(f32, f32)],
) {
    let keep_r2 = 7.0f32 * 7.0;
    for y in 0..oh {
        for x in 0..ow {
            let i = (y * ow + x) * 4;
            if overlay[i + 3] == 0 {
                continue;
            }
            let biome = biome_at_canvas(grid, ox + x as f32, oy + y as f32);
            let Some(b) = biome else { continue };
            if !is_water_biome(b) {
                continue;
            }
            let cx = ox + x as f32;
            let cy = oy + y as f32;
            let near_bridge = bridge_canvas_pts.iter().any(|&(bx, by)| {
                let dx = cx - bx;
                let dy = cy - by;
                dx * dx + dy * dy <= keep_r2
            });
            if !near_bridge {
                clear_alpha(overlay, ow, x, y);
            }
        }
    }
}

fn paint_road_stroke_onto(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    road: &Road,
    bounds: &WorldBounds,
    detail: RoadDetail,
) {
    let canvas_w = width as f32;
    let canvas_h = height as f32;
    let pts = road_to_canvas(&road.points, bounds, canvas_w, canvas_h);
    let lw = road_layer_width(road.kind, road.surface, detail);
    match road.surface {
        RoadSurface::Paved => {
            let outline_w = lw
                + if detail == RoadDetail::Close {
                    1.6
                } else {
                    0.9
                };
            stroke_polyline(
                pixels,
                width,
                height,
                &pts,
                outline_w,
                Rgba::rgba(42, 28, 14, (0.72 * 255.0) as u8),
            );
            let fill = if detail == RoadDetail::Close {
                Rgba::rgba(214, 176, 110, (0.95 * 255.0) as u8)
            } else {
                Rgba::rgba(196, 163, 106, (0.88 * 255.0) as u8)
            };
            stroke_polyline(pixels, width, height, &pts, lw, fill);
        }
        RoadSurface::Packed => {
            let col = if detail == RoadDetail::Close {
                Rgba::rgba(122, 90, 54, (0.88 * 255.0) as u8)
            } else {
                Rgba::rgba(110, 82, 50, (0.78 * 255.0) as u8)
            };
            stroke_polyline(pixels, width, height, &pts, lw, col);
        }
        RoadSurface::Dirt => {
            let col = if detail == RoadDetail::Close {
                Rgba::rgba(86, 68, 46, (0.8 * 255.0) as u8)
            } else {
                Rgba::rgba(86, 68, 46, (0.62 * 255.0) as u8)
            };
            stroke_polyline(pixels, width, height, &pts, lw, col);
        }
    }
}

// ── Public draw / hit API ──────────────────────────────────────────────────

pub fn draw_roads(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    roads: &[Road],
    bounds: &WorldBounds,
    detail: RoadDetail,
    grid: Option<&TerrainGrid>,
) {
    let order = [
        RoadKind::Local,
        RoadKind::Secondary,
        RoadKind::Highway,
    ];
    let visible: Vec<&Road> = roads
        .iter()
        .filter(|road| {
            if detail == RoadDetail::Main && road.kind != RoadKind::Highway {
                return false;
            }
            road.points.len() >= 2 && road_in_view(&road.points, bounds)
        })
        .collect();

    let paint_all = |target: &mut [u8]| {
        for kind in order {
            for road in &visible {
                if road.kind != kind {
                    continue;
                }
                paint_road_stroke_onto(target, width, height, road, bounds, detail);
            }
        }
    };

    if grid.is_none() {
        paint_all(pixels);
        for road in &visible {
            paint_bridge_marks(pixels, width, height, road, bounds, detail);
        }
        return;
    }

    let mut overlay = vec![0u8; width * height * 4];
    paint_all(&mut overlay);
    let canvas_w = width as f32;
    let canvas_h = height as f32;
    let mut bridge_pts = Vec::new();
    for road in &visible {
        for p in &road.points {
            if p.crossing != Some(RoadCrossing::Bridge) {
                continue;
            }
            bridge_pts.push((
                ((p.x - bounds.x0) / bounds.span) * canvas_w,
                ((p.y - bounds.y0) / bounds.span) * canvas_h,
            ));
        }
    }
    punch_road_water(
        &mut overlay,
        width,
        height,
        0.0,
        0.0,
        grid.unwrap(),
        &bridge_pts,
    );
    blit_overlay(pixels, width, height, &overlay, width, height, 0, 0);
    for road in &visible {
        paint_bridge_marks(pixels, width, height, road, bounds, detail);
    }
}

pub fn draw_settlements(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    settlements: &[Settlement],
    bounds: &WorldBounds,
    hovered: Option<&SettlementHit>,
    style: SettlementDrawStyle,
    grid: Option<&TerrainGrid>,
) {
    let px_per_world = width as f32 / bounds.span;
    let canvas_w = width as f32;
    let canvas_h = height as f32;

    for kind in DRAW_ORDER {
        for (si, settlement) in settlements.iter().enumerate() {
            if settlement.kind != kind {
                continue;
            }
            if !settlement_intersects_bounds(settlement, bounds) {
                continue;
            }
            let sx = ((settlement.x - bounds.x0) / bounds.span) * canvas_w;
            let sy = ((settlement.y - bounds.y0) / bounds.span) * canvas_h;

            let is_hovered = hovered.is_some_and(|h| {
                h.settlement_index == si
                    || settlements
                        .get(h.settlement_index)
                        .is_some_and(|hs| same_settlement(hs, settlement))
            });
            let hovered_district = if is_hovered {
                hovered.and_then(|h| h.district_index)
            } else {
                None
            };

            match style {
                SettlementDrawStyle::Marker => {
                    draw_city_marker(pixels, width, height, sx, sy, settlement.kind, is_hovered);
                }
                SettlementDrawStyle::Plan => {
                    draw_footprint(
                        pixels,
                        width,
                        height,
                        sx,
                        sy,
                        settlement,
                        px_per_world,
                        is_hovered,
                        hovered_district,
                        grid,
                    );
                }
            }
        }
    }
}

pub fn hit_settlement(
    px: f32,
    py: f32,
    width: usize,
    height: usize,
    settlements: &[Settlement],
    bounds: &WorldBounds,
    style: SettlementDrawStyle,
) -> Option<SettlementHit> {
    let canvas_w = width as f32;
    let canvas_h = height as f32;
    let px_per_world = canvas_w / bounds.span;
    let mut best: Option<SettlementHit> = None;
    let mut best_d = f32::INFINITY;

    for (si, settlement) in settlements.iter().enumerate() {
        let sx = ((settlement.x - bounds.x0) / bounds.span) * canvas_w;
        let sy = ((settlement.y - bounds.y0) / bounds.span) * canvas_h;
        if style == SettlementDrawStyle::Marker {
            let d = ((px - sx).powi(2) + (py - sy).powi(2)).sqrt();
            if d <= marker_size(settlement.kind).hit && d < best_d {
                best = Some(SettlementHit {
                    settlement_index: si,
                    district_index: None,
                });
                best_d = d;
            }
            continue;
        }
        let r = settlement_footprint_radius(settlement, px_per_world);
        let (nx, ny) = to_local_norm(px, py, sx, sy, r);
        let outline = plan_outline(settlement);
        if !point_in_poly(nx, ny, &outline) {
            continue;
        }
        let dx = nx - settlement.core_dx;
        let dy = ny - settlement.core_dy;
        let d = dx * dx + dy * dy;
        if d < best_d {
            best = Some(SettlementHit {
                settlement_index: si,
                district_index: district_at(settlement, nx, ny),
            });
            best_d = d;
        }
    }
    best
}
