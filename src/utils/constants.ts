export const REGION_SIZE = 64;
export const CHUNK_SIZE = 8;
export const CHUNKS_PER_REGION = REGION_SIZE / CHUNK_SIZE;
export const MACRO_RESOLUTION = 512;
export const MICRO_RESOLUTION = 512;
export const CHUNK_RESOLUTION = 256;

/** Maximum terrain elevation in meters (sea level = 0 m). */
export const MAX_ELEVATION_M = 10_000;

/** Mountain biome starts around this elevation (m). */
export const HIGH_ELEVATION_M = 5_385;
