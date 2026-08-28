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
