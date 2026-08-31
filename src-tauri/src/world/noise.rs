use noise::{NoiseFn, SuperSimplex};

/// Normalizacja z [-1, 1] do [0, 1]
pub fn to_01(v: f64) -> f64 {
    v * 0.5 + 0.5
}

/// fBm — suma oktaw simplex noise
pub fn fbm(
    noise: &SuperSimplex,
    x: f64,
    y: f64,
    octaves: u32,
    persistence: f64,
    lacunarity: f64,
) -> f64 {
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut value = 0.0;
    let mut max_value = 0.0;

    for _ in 0..octaves {
        value += to_01(noise.get([x * frequency, y * frequency])) * amplitude;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    if max_value == 0.0 {
        0.0
    } else {
        value / max_value
    }
}

pub fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub fn redistribute(elevation: f64, exponent: f64, fudge: f64) -> f64 {
    let e = clamp01(elevation * fudge);
    e.powf(exponent)
}

pub fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if (edge1 - edge0).abs() < f64::EPSILON {
        return if x >= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Ridged multifractal — connected crest lines for mountain belts.
pub fn ridged_fbm(
    noise: &SuperSimplex,
    x: f64,
    y: f64,
    octaves: u32,
    persistence: f64,
    lacunarity: f64,
) -> f64 {
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    let mut total = 0.0;
    let mut max_amp = 0.0;
    let mut weight = 1.0;

    for _ in 0..octaves {
        let n = noise.get([x * frequency, y * frequency]);
        let signal = (1.0 - n.abs()).clamp(0.0, 1.0);
        let ridged = signal * signal * weight;
        total += ridged * amplitude;
        max_amp += amplitude;
        weight = (signal * 2.0).clamp(0.0, 1.0);
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    if max_amp == 0.0 {
        0.0
    } else {
        total / max_amp
    }
}
