fn hash_bytes(bytes: impl IntoIterator<Item = u8>) -> u32 {
  let mut state: u32 = 0;
  for byte in bytes {
    state = state.wrapping_mul(31).wrapping_add(byte as u32);
  }
  state
}

pub fn hash_seed(seed: u64) -> u32 {
  hash_bytes(seed.to_le_bytes())
}

pub fn derive_seed(seed: u64, salt: &str) -> u32 {
  hash_bytes(seed.to_le_bytes().into_iter().chain(salt.bytes()))
}

fn mix64(mut z: u64) -> u64 {
  z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
  z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
  z ^ (z >> 31)
}

/// Deterministic value in [0, 1) from seed + channel + index.
/// Consecutive indices are uncorrelated (unlike the polynomial string hash).
pub fn unit_noise(seed: u64, channel: u64, index: u64) -> f64 {
  let z = mix64(seed)
    .wrapping_add(channel.wrapping_mul(0x9E3779B97F4A7C15))
    .wrapping_add(index.wrapping_mul(0xD1B54A32D192ED03));
  (mix64(z) >> 11) as f64 / ((1u64 << 53) as f64)
}

#[cfg(test)]
mod tests {
  use super::unit_noise;

  #[test]
  fn consecutive_indices_are_spread() {
    let ys: Vec<f64> = (0..6).map(|i| unit_noise(9, 2, i)).collect();
    let min = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let max = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
      max - min > 0.3,
      "y-channels should not cluster; got {ys:?}"
    );
  }
}