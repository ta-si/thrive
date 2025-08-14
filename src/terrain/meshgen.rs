use bevy::prelude::*;
use noiz::prelude::*;

/// Generate an n×n height field over a tile of world-space `tile_world_size`,
/// sampling Perlin fBm at world coordinates starting at `origin`.
pub fn generate_height_field(
    n: usize,
    tile_world_size: f32,
    origin: Vec2,
    seed: u32,
    octaves: u32,
    lacunarity: f32,
    persistence: f32,
    frequency: f32,
    amplitude: f32,
) -> Vec<f32> {
    // Build Perlin-fBm with noiz
    type PerlinBase = MixCellGradients<noiz::cells::OrthoGrid, noiz::curves::Smoothstep, noiz::cell_noise::QuickGradients>;
    type PerlinFbm = Noise<LayeredNoise<Normed<f32>, Persistence, FractalLayers<Octave<PerlinBase>>>>;

    let layered = LayeredNoise::new(
        Normed::default(),
        Persistence(persistence),
        FractalLayers {
            layer: Default::default(),
            lacunarity,
            amount: octaves,
        },
    );
    let mut fbm: PerlinFbm = Noise::from(layered);
    fbm.set_seed(seed);
    fbm.set_frequency(frequency);

    let step = tile_world_size / (n as f32 - 1.0);
    let mut heights = vec![0.0; n * n];
    for z in 0..n {
        for x in 0..n {
            let wx = origin.x + x as f32 * step;
            let wz = origin.y + z as f32 * step;
            let h: f32 = fbm.sample(Vec2::new(wx, wz));
            heights[z * n + x] = h * amplitude;
        }
    }
    heights
}

/// Make an RGBA8 normal map (world-space, encoded 0..1) from heights.
pub fn normalmap_from_height(n: usize, step: f32, heights: &[f32]) -> Vec<u8> {
    let mut out = vec![0u8; n*n*4];
    let idx = |x: isize, z: isize| -> usize {
        let xi = x.clamp(0, (n-1) as isize) as usize;
        let zi = z.clamp(0, (n-1) as isize) as usize;
        zi*n + xi
    };
    for z in 0..n as isize {
        for x in 0..n as isize {
            let h_l = heights[idx(x-1, z)];
            let h_r = heights[idx(x+1, z)];
            let h_d = heights[idx(x, z-1)];
            let h_u = heights[idx(x, z+1)];
            let dx = (h_r - h_l) / (2.0 * step);
            let dz = (h_u - h_d) / (2.0 * step);
            let nvec = Vec3::new(-dx, 1.0, -dz).normalize();
            let i = (z as usize * n + x as usize) * 4;
            out[i+0] = ((nvec.x * 0.5 + 0.5) * 255.0) as u8;
            out[i+1] = ((nvec.y * 0.5 + 0.5) * 255.0) as u8;
            out[i+2] = ((nvec.z * 0.5 + 0.5) * 255.0) as u8;
            out[i+3] = 255;
        }
    }
    out
}
