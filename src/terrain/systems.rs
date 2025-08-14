use bevy::prelude::*;
use bevy::pbr::{MaterialPipeline, MeshMaterial3d};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::tasks::futures::check_ready;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::render_asset::RenderAssetUsages;
use std::collections::{HashMap, HashSet};

use super::flatmesh::SharedMeshes;
use super::material::{TerrainMaterial, TileParams};
use super::meshgen::{generate_height_field, normalmap_from_height};

#[derive(Component)]
pub struct TileLoader {
    pub radius_tiles: i32,
}

#[derive(Resource)]
pub struct TerrainConfig {
    pub tile_size: f32,
    pub tile_resolution: usize,
    pub seed: u32,
    pub noise_octaves: u32,
    pub noise_lacunarity: f32,
    pub noise_persistence: f32,
    pub noise_frequency: f32,
    pub noise_amplitude: f32,
    pub despawn_grace_seconds: f32,
    pub max_spawns_per_frame: usize,
    pub max_in_flight_tasks: usize,
}
impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            tile_size: 32.0,
            tile_resolution: 129,
            seed: 12345,
            noise_octaves: 6,
            noise_lacunarity: 2.0,
            noise_persistence: 0.5,
            noise_frequency: 0.08,
            noise_amplitude: 10.0,
            despawn_grace_seconds: 1.0,
            max_spawns_per_frame: 8,
            max_in_flight_tasks: 16,
        }
    }
}

#[derive(Resource, Default)]
pub struct TerrainState {
    pub tiles: HashMap<IVec2, Entity>,
    pub pending: HashMap<IVec2, Entity>,
    pub last_touched: HashMap<IVec2, f32>,
}

#[derive(Component)]
pub struct Tile {
    pub coord: IVec2,
}

#[derive(Component)]
pub struct TileBuildTask {
    pub coord: IVec2,
    pub origin: Vec2,
    pub task: Task<TileBuildResult>,
}

pub struct TileBuildResult {
    pub coord: IVec2,
    pub height_bytes: Vec<u8>, // R32f
    pub normal_bytes: Vec<u8>, // RGBA8
}

fn color_for_coord(c: IVec2) -> Color {
    let palette = [
        Color::hsl(  2.0, 0.65, 0.55),
        Color::hsl(120.0, 0.55, 0.50),
        Color::hsl(230.0, 0.60, 0.52),
        Color::hsl( 45.0, 0.70, 0.55),
        Color::hsl(280.0, 0.55, 0.56),
        Color::hsl(180.0, 0.55, 0.52),
    ];
    let idx = ((c.x & 1) + ((c.y & 1) << 1)) as usize;
    palette[idx % palette.len()]
}

fn world_to_coord(p: Vec3, tile_size: f32) -> IVec2 {
    IVec2::new((p.x / tile_size).floor() as i32, (p.z / tile_size).floor() as i32)
}

pub fn queue_and_spawn_tasks_system(
    time: Res<Time>,
    mut commands: Commands,
    mut state: ResMut<TerrainState>,
    cfg: Res<TerrainConfig>,
    q_loaders: Query<(&Transform, &TileLoader)>,
) {
    // Desired tiles from all loaders
    let mut desired: HashSet<IVec2> = HashSet::new();
    for (xf, loader) in &q_loaders {
        let center = world_to_coord(xf.translation, cfg.tile_size);
        let r = loader.radius_tiles;
        for dz in -r..=r {
            for dx in -r..=r {
                desired.insert(IVec2::new(center.x + dx, center.y + dz));
            }
        }
    }

    // Keep alive tiles we've touched
    let now = time.elapsed_secs();
    for c in desired.iter() {
        if state.tiles.contains_key(c) || state.pending.contains_key(c) {
            state.last_touched.insert(*c, now);
        }
    }

    // Missing tiles
    let mut missing: Vec<IVec2> = desired
        .iter()
        .filter(|c| !state.tiles.contains_key(*c) && !state.pending.contains_key(*c))
        .copied()
        .collect();

    // Sort by distance to nearest loader
    let centers: Vec<IVec2> = q_loaders
        .iter()
        .map(|(t, _)| world_to_coord(t.translation, cfg.tile_size))
        .collect();
    missing.sort_by_key(|c| {
        centers
            .iter()
            .map(|cc| (cc.x - c.x).abs() + (cc.y - c.y).abs())
            .min()
            .unwrap_or(0)
    });

    // Task capacity
    let available = cfg.max_in_flight_tasks.saturating_sub(state.pending.len());
    let capacity = available.min(cfg.max_spawns_per_frame);
    if capacity == 0 { return; }

    // Spawn tile build tasks
    let pool = AsyncComputeTaskPool::get();
    for coord in missing.into_iter().take(capacity) {
        let origin = Vec2::new(coord.x as f32 * cfg.tile_size, coord.y as f32 * cfg.tile_size);
        let n = cfg.tile_resolution;
        let size = cfg.tile_size;

        let seed = cfg.seed;
        let (oct, lac, per, freq, amp) = (
            cfg.noise_octaves,
            cfg.noise_lacunarity,
            cfg.noise_persistence,
            cfg.noise_frequency,
            cfg.noise_amplitude,
        );

        let task: Task<TileBuildResult> = pool.spawn(async move {
            let heights = generate_height_field(n, size, origin, seed, oct, lac, per, freq, amp);
            let height_bytes: Vec<u8> = heights.iter().flat_map(|h| h.to_le_bytes()).collect();
            let step = size / (n as f32 - 1.0);
            let normal_bytes = normalmap_from_height(n, step, &heights);
            TileBuildResult { coord, height_bytes, normal_bytes }
        });

        let e = commands.spawn(TileBuildTask { coord, origin, task }).id();
        state.pending.insert(coord, e);
        state.last_touched.insert(coord, now);
    }

    // Mark out-of-range for GC after grace (avoid borrow conflict by two-phase)
    let cutoff = now - cfg.despawn_grace_seconds;
    let mut to_unmark: Vec<IVec2> = Vec::new();
    for c in state.tiles.keys() {
        if !desired.contains(c) && state.last_touched.get(c).copied().unwrap_or(0.0) < cutoff {
            to_unmark.push(*c);
        }
    }
    for c in to_unmark {
        state.last_touched.remove(&c);
    }
}

pub fn collect_finished_tasks_system(
    time: Res<Time>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    shared: Res<SharedMeshes>,
    mut state: ResMut<TerrainState>,
    cfg: Res<TerrainConfig>,
    mut q_tasks: Query<(Entity, &mut TileBuildTask)>,
) {
    let now = time.elapsed_secs();

    for (e, mut t) in q_tasks.iter_mut() {
        if let Some(result) = bevy::tasks::futures::check_ready(&mut t.task) {
            let size_u = cfg.tile_resolution as u32;

            let height_img = Image::new(
                Extent3d { width: size_u, height: size_u, depth_or_array_layers: 1 },
                TextureDimension::D2,
                result.height_bytes,
                TextureFormat::R32Float,
                RenderAssetUsages::RENDER_WORLD,
            );
            let normal_img = Image::new(
                Extent3d { width: size_u, height: size_u, depth_or_array_layers: 1 },
                TextureDimension::D2,
                result.normal_bytes,
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::RENDER_WORLD,
            );
            let height_h = images.add(height_img);
            let normal_h = images.add(normal_img);

            let c = color_for_coord(result.coord).to_linear();
            let tile_color = Vec4::new(c.red, c.green, c.blue, c.alpha);

            let params = TileParams {
                tile_size: cfg.tile_size,
                height_scale: 1.0,                      // try 0.0 first if you want purely flat debug
                texels_per_side: cfg.tile_resolution as u32,
                _pad: 0,
                tile_color,
            };

            let mat = materials.add(TerrainMaterial {
                params,
                height_tex: height_h,
                normal_tex: normal_h,
            });

            commands.entity(e)
                .remove::<TileBuildTask>()
                .insert((
                    Tile { coord: result.coord },
                    Mesh3d(shared.flat.clone()),
                    MeshMaterial3d(mat),
                    Transform::from_translation(Vec3::new(t.origin.x, 0.0, t.origin.y)),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                ));

            state.pending.remove(&result.coord);
            state.tiles.insert(result.coord, e);
            state.last_touched.insert(result.coord, now);
        }
    }
}

pub fn garbage_collect_tiles_system(
    mut commands: Commands,
    mut state: ResMut<TerrainState>,
    q_tiles: Query<(Entity, &Tile)>,
) {
    let mut to_despawn: Vec<(IVec2, Entity)> = Vec::new();
    for (e, tile) in &q_tiles {
        if !state.last_touched.contains_key(&tile.coord) {
            to_despawn.push((tile.coord, e));
        }
    }
    for (c, e) in to_despawn {
        state.tiles.remove(&c);
        commands.entity(e).despawn();
    }
}
