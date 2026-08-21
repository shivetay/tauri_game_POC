pub fn hash_seed(seed: &str) -> u32 {
  let mut state: u32 = 0;
  for byte in seed.bytes() {
    state = state.wrapping_mul(31).wrapping_add(byte as u32);
  }
  state
}

pub fn hash_seed_u64(seed: &str) -> u64 {
  let a = hash_seed(seed) as u64;
  let b = hash_seed(&format!("{seed}:b")) as u64;
  (a << 32) | b
}