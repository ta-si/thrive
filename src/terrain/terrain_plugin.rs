// src/terrain/terrain_plugin.rs
// ──────────────────────────────────────────────────────────────
// Terrain streaming for Bevy 0.16.1 with mesh caching, version-based invalidation,
// limited concurrent tasks (default 8), proximity-based dispatch, and seeded noise-based heights.
//
// Vertex colors now blend smoothly based on configurable TerrainColor entries stored in TerrainManager.
//
// Components:
//   TerrainSpawner { pub radius: u32, pub coord: IVec2 }
//   TerrainTile    { pub coord: IVec2 }
//
// Resources:
//   TerrainManager {
//     tile_size: f32,
//     resolution: u32,
//     max_height: f32,
//     grid_min: IVec2,
//     grid_max: IVec2,
//     max_concurrent_tasks: usize,
//     cache_version: u64,
//     cache: HashMap<IVec2, CachedTile>,
//     tiles_to_load: Vec<IVec2>,
//     pending: HashMap<IVec2, Task<TileResult>>,
//     loaded: HashMap<IVec2, LoadedTile>,
//     material_handle: Handle<StandardMaterial>,
//     terrain_colors: Vec<TerrainColor>,
//   }
//
// Systems:
//   terrain_spawner_tracker      – updates spawner.coord when Transform changes
//   terrain_tile_loader          – computes desired tiles, unloads, spawns from cache or enqueues
//   terrain_tile_task_dispatch   – dispatches new tasks up to limit, prioritised by proximity
//   terrain_tile_task_complete   – finalises tasks: cache meshes & spawn tile entities
//
// Functions:
//   generate_tile_mesh(coord, settings, terrain_colors) → TileResult
//   height_at(x, z, max_height) → f32 (uses noisy_bevy::simplex_noise_2d_seeded)
//   noise_at(x, y, freq, min, max, seed) → f32

use bevy::{
    math::{IVec2, Vec2, Vec3},
    prelude::*,
    render::{
        mesh::{Indices, Mesh, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
    tasks::{futures_lite::future, AsyncComputeTaskPool, Task},
};
use noisy_bevy::simplex_noise_2d_seeded;
use std::collections::{HashMap, HashSet};

/// Defines a terrain color with blending ranges:
/// - `color`: RGB color.
/// - `slope_limits`: [fade_low, solid_low, solid_high, fade_high].
/// - `height_limits`: [fade_low, solid_low, solid_high, fade_high].
#[derive(Clone, Copy)]
struct TerrainColor {
    color:         Vec3,
    slope_limits:  [f32; 4],
    height_limits: [f32; 4],
}

/// Marks an entity that drives tile streaming (e.g., a camera).
#[derive(Component, Debug, Copy, Clone)]
pub struct TerrainSpawner {
    /// Loading radius in *tiles*.
    pub radius: u32,
    /// Current tile‐grid coordinate.
    pub coord:  IVec2,
}

/// Tag component for spawned terrain chunks.
#[derive(Component)]
pub struct TerrainTile {
    pub coord: IVec2,
}

/// Represents a cached mesh and the version it was built under.
struct CachedTile {
    mesh_handle: Handle<Mesh>,
    version:     u64,
}

/// Represents an active, spawned tile with its entity and mesh handle.
struct LoadedTile {
    entity:      Entity,
    mesh_handle: Handle<Mesh>,
}

/// Payload returned by the async mesh generator.
#[derive(Debug)]
pub struct TileResult {
    pub coord: IVec2,
    pub mesh:  Mesh,
}

/// Global terrain settings and bookkeeping.
#[derive(Resource)]
pub struct TerrainManager {
    /// World‐space size of one tile.
    pub tile_size:           f32,
    /// Number of vertices per tile‐edge (minimum 2).
    pub resolution:          u32,
    /// Maximum height displacement.
    pub max_height:          f32,
    /// Inclusive min grid coordinate allowed.
    pub grid_min:            IVec2,
    /// Inclusive max grid coordinate allowed.
    pub grid_max:            IVec2,
    /// Maximum number of concurrent async tasks.
    pub max_concurrent_tasks: usize,
    /// Increment this whenever mesh generation logic changes.
    pub cache_version:       u64,
    /// Stores meshes that have been generated: coord → (mesh_handle, version).
    pub cache:               HashMap<IVec2, CachedTile>,
    /// Coords that need tasks dispatched (filled by loader, drained by dispatch).
    pub tiles_to_load:       Vec<IVec2>,
    /// Maps tile coord → running async mesh task.
    pub pending:             HashMap<IVec2, Task<TileResult>>,
    /// Maps tile coord → spawned entity + its mesh handle.
    pub loaded:              HashMap<IVec2, LoadedTile>,
    /// Shared material handle (unlit, vertex‐colors enabled).
    pub material_handle:     Handle<StandardMaterial>,
    /// Configurable list of TerrainColor entries for blending.
    pub terrain_colors:      Vec<TerrainColor>,
}

impl Default for TerrainManager {
    fn default() -> Self {
        // Define default TerrainColor entries:
        let terrain_colors = vec![
            TerrainColor {
                // Sand: low height, gentle slopes
                color: Vec3::new(0.7, 0.65, 0.45),
                slope_limits:  [0.0, 0.0, 0.2, 0.35],  // full sand if slope ≤ 0.5, fade out by 0.6
                height_limits: [0.0, 0.0, 0.3, 0.32],  // full sand if height ≤ 0.2, fade out by 0.3
            },
            TerrainColor {
                // Grass: mid height, gentle to moderate slopes
                color: Vec3::new(0.10, 0.40, 0.10),
                slope_limits:  [0., 0., 0.3, 0.4],  // fade in at slope 0.4, full by 0.5, fade out by 0.8
                height_limits: [0.2, 0.3, 0.6, 0.7],  // fade in at height 0.2, full by 0.3, fade out by 0.7
            },
            TerrainColor {
                // Rock: moderate slopes or higher elevations
                color: Vec3::new(0.50, 0.50, 0.50),
                slope_limits:  [0.3, 0.4, 1.0, 1.0],  // full rock if slope ≥ 0.6
                height_limits: [0., 0.0, 1., 1.],  // fade in at height 0.5, full by 0.6, fade out by 0.9
            },
            TerrainColor {
                // Snow: high elevations
                color: Vec3::new(1.0,  1.0,  1.0),
                slope_limits:  [0.0, 0.0, 1.0, 1.0],  // slope doesn't matter
                height_limits: [0.8, 0.9, 1.0, 1.0],  // fade in at height 0.8, full by 0.9
            },
        ];

        TerrainManager {
            tile_size:            128.0,
            resolution:           129,
            max_height:           100.0,
            grid_min:             IVec2::new(-50, -50),
            grid_max:             IVec2::new( 50,  50),
            max_concurrent_tasks: 8,
            cache_version:        1,
            cache:                HashMap::new(),
            tiles_to_load:        Vec::new(),
            pending:              HashMap::new(),
            loaded:               HashMap::new(),
            material_handle:      Handle::default(),
            terrain_colors,
        }
    }
}

/// TerrainPlugin ties everything together.
pub struct TerrainPlugin;
impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainManager>()
            // 1) Track each spawner’s tile coordinate (only if Transform changed)
            .add_systems(
                Update,
                terrain_spawner_tracker
                    .run_if(|q: Query<(), Changed<Transform>>| !q.is_empty()),
            )
            // 2) Compute desired vs. loaded → unload, spawn from cache or enqueue
            .add_systems(
                Update,
                terrain_tile_loader
                    .run_if(any_spawner_moved),
            )
            // 3) Dispatch up to max_concurrent_tasks, prioritised by proximity
            .add_systems(Update, terrain_tile_task_dispatch)
            // 4) Complete finished tasks: cache meshes & spawn entities
            .add_systems(Update, terrain_tile_task_complete);
    }
}

/// Run‐condition: true if *any* TerrainSpawner had Changed<TerrainSpawner>.
fn any_spawner_moved(q: Query<&TerrainSpawner, Changed<TerrainSpawner>>) -> bool {
    !q.is_empty()
}

/// 1) terrain_spawner_tracker
///
/// Updates each TerrainSpawner’s `coord` when its Transform crosses a tile boundary.
/// Uses ChangeDetection on Transform to run only when that spawner moved.
fn terrain_spawner_tracker(
    manager: Res<TerrainManager>,
    mut query: Query<(&Transform, &mut TerrainSpawner), Changed<Transform>>,
) {
    let tile_size = manager.tile_size;
    for (tf, mut spawner) in query.iter_mut() {
        // Compute new tile‐grid coordinate from world translation (x, z).
        let new_coord = (tf.translation.xz() / tile_size).floor().as_ivec2();
        if spawner.coord != new_coord {
            spawner.coord = new_coord; // triggers Changed<TerrainSpawner> next frame
        }
    }
}

/// 2) terrain_tile_loader
///
/// - Determine which tile coords *should* exist based on all spawners’ coords & radii,
///   clamped to grid_min..=grid_max.
/// - Unload any loaded tile whose coord is not in that set.
/// - For each desired coord not in loaded:
///    • If in cache with matching version → spawn entity immediately.
///    • Else if not pending → enqueue for async build (tiles_to_load).
fn terrain_tile_loader(
    mut commands: Commands,
    spawners: Query<&TerrainSpawner>,
    mut manager: ResMut<TerrainManager>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // 2A) Build the set of desired coords.
    let mut desired = HashSet::<IVec2>::new();
    for spawner in spawners.iter() {
        for dz in -(spawner.radius as i32)..=(spawner.radius as i32) {
            for dx in -(spawner.radius as i32)..=(spawner.radius as i32) {
                let coord = spawner.coord.saturating_add(IVec2::new(dx, dz));
                if coord.x < manager.grid_min.x
                    || coord.y < manager.grid_min.y
                    || coord.x > manager.grid_max.x
                    || coord.y > manager.grid_max.y
                {
                    continue;
                }
                desired.insert(coord);
            }
        }
    }

    // 2B) Unload loaded tiles not in desired.
    let mut to_unload = Vec::<IVec2>::new();
    for (&coord, _) in manager.loaded.iter() {
        if !desired.contains(&coord) {
            to_unload.push(coord);
        }
    }
    for coord in to_unload {
        if let Some(loaded_tile) = manager.loaded.remove(&coord) {
            commands.entity(loaded_tile.entity).despawn();
            // Retain mesh in cache for reuse; call meshes.remove(...) if evicting.
        }
    }

    // 2C) For each coord in desired that is not loaded:
    manager.tiles_to_load.clear();
    for &coord in desired.iter() {
        if manager.loaded.contains_key(&coord) {
            continue; // Already in world
        }
        if let Some(cached) = manager.cache.get(&coord) {
            // If version matches, spawn immediately from cache.
            if cached.version == manager.cache_version {
                let mesh_handle = cached.mesh_handle.clone();
                let mat_handle = manager.material_handle.clone();
                let entity = commands.spawn((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(mat_handle),
                    Transform::from_xyz(
                        coord.x as f32 * manager.tile_size,
                        0.0,
                        coord.y as f32 * manager.tile_size,
                    ),
                    GlobalTransform::default(),
                    Visibility::default(),
                    TerrainTile { coord },
                ))
                .id();
                manager.loaded.insert(coord, LoadedTile { entity, mesh_handle });
                continue;
            } else {
                // Cached but stale: evict from cache so we regenerate.
                meshes.remove(&cached.mesh_handle);
                manager.cache.remove(&coord);
            }
        }
        // Not loaded, not fresh in cache, not pending → enqueue for async.
        if !manager.pending.contains_key(&coord) {
            manager.tiles_to_load.push(coord);
        }
    }
}

/// 3) terrain_tile_task_dispatch
///
/// Spawns async mesh-generation tasks for coords in `tiles_to_load`, up to
/// `manager.max_concurrent_tasks`, prioritised by proximity to any spawner.
/// Remaining coords stay in `tiles_to_load` for next frame.
fn terrain_tile_task_dispatch(
    mut manager: ResMut<TerrainManager>,
    spawners: Query<&TerrainSpawner>,
) {
    // Build a sorted list: (distance², coord).
    let mut list: Vec<(i32, IVec2)> = Vec::new();
    for &coord in manager.tiles_to_load.iter() {
        let mut min_d2 = i32::MAX;
        for spawner in spawners.iter() {
            let delta = coord - spawner.coord;
            let d2 = delta.x * delta.x + delta.y * delta.y;
            if d2 < min_d2 {
                min_d2 = d2;
            }
        }
        list.push((min_d2, coord));
    }
    list.sort_by_key(|&(d2, _)| d2);

    // Spawn tasks up to max_concurrent_tasks, capturing a clone of terrain_colors.
    let terrain_colors = manager.terrain_colors.clone();
    let mut spawned_set = HashSet::<IVec2>::new();
    for &(_d2, coord) in list.iter() {
        if manager.pending.len() >= manager.max_concurrent_tasks {
            break;
        }
        if manager.loaded.contains_key(&coord) {
            spawned_set.insert(coord);
            continue;
        }
        if manager.cache.contains_key(&coord)
            && manager.cache[&coord].version == manager.cache_version
        {
            spawned_set.insert(coord);
            continue;
        }
        if manager.pending.contains_key(&coord) {
            spawned_set.insert(coord);
            continue;
        }
        let settings = (
            manager.tile_size,
            manager.resolution,
            manager.max_height,
        );
        let colors_clone = terrain_colors.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move {
            generate_tile_mesh(coord, settings, colors_clone)
        });
        manager.pending.insert(coord, task);
        spawned_set.insert(coord);
    }

    // Retain only coords that weren't spawned.
    manager
        .tiles_to_load
        .retain(|coord| !spawned_set.contains(coord));
}

/// 4) terrain_tile_task_complete
///
/// Polls all pending tasks; on completion:
///  1) Cache the new mesh with current version
///  2) Spawn entity using the shared material handle
///  3) Insert into manager.loaded
fn terrain_tile_task_complete(
    mut commands: Commands,
    mut manager: ResMut<TerrainManager>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut still_pending = HashMap::<IVec2, Task<TileResult>>::new();

    let tile_size = manager.tile_size;
    let cache_version = manager.cache_version; // Clone cache_version to avoid immutable borrow
    let pending_tasks: Vec<_> = manager.pending.drain().collect();
    for (coord, mut task) in pending_tasks {
        match future::block_on(future::poll_once(&mut task)) {
            Some(result) => {
                if manager.loaded.contains_key(&coord) {
                    continue;
                }

                // Add mesh to asset store.
                let mesh_handle = meshes.add(result.mesh);
                // Cache it.
                manager.cache.insert(
                    coord,
                    CachedTile {
                        mesh_handle: mesh_handle.clone(),
                        version:     cache_version,
                    },
                );

                // Spawn entity.
                let mat_handle = manager.material_handle.clone();
                let entity = commands.spawn((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(mat_handle),
                    Transform::from_xyz(
                        coord.x as f32 * tile_size,
                        0.0,
                        coord.y as f32 * tile_size,
                    ),
                    GlobalTransform::default(),
                    Visibility::default(),
                    TerrainTile { coord },
                ))
                .id();

                manager.loaded.insert(coord, LoadedTile { entity, mesh_handle });
            }
            None => {
                still_pending.insert(coord, task);
            }
        }
    }

    manager.pending = still_pending;
}

/// Async worker: generate a height-displaced, colour-by-configurable-terrain-colors Mesh for a tile.
///
/// Uses `height_at` to compute height with `simplex_noise_2d_seeded` from noisy_bevy:
///   - noise in [-1..1] → clamp to [0..1] → multiply by max_height.
///
/// Face normal is computed via (p2 - p0) × (p1 - p0), ensuring upward‐facing normals point +Y.
///
/// Vertex colors are computed by blending across multiple `TerrainColor` entries.
fn generate_tile_mesh(
    coord: IVec2,
    (tile_size, resolution, max_height): (f32, u32, f32),
    terrain_colors: Vec<TerrainColor>,
) -> TileResult {
    let verts_per_edge = resolution.max(2);
    let step = tile_size / ((verts_per_edge - 1) as f32);

    // 1) Generate vertex positions.
    let mut positions = Vec::<[f32; 3]>::with_capacity((verts_per_edge * verts_per_edge) as usize);
    for j in 0..verts_per_edge {
        for i in 0..verts_per_edge {
            let world_x = coord.x as f32 * tile_size + i as f32 * step;
            let world_z = coord.y as f32 * tile_size + j as f32 * step;
            let y = height_at(world_x, world_z, max_height);
            positions.push([i as f32 * step, y, j as f32 * step]);
        }
    }

    // 2) Build indices and accumulate normals.
    let mut indices = Vec::<u32>::new();
    let mut normals = vec![[0.0_f32; 3]; positions.len()];
    for j in 0..(verts_per_edge - 1) {
        for i in 0..(verts_per_edge - 1) {
            let i0 =  j      * verts_per_edge + i;
            let i1 =  j      * verts_per_edge + i + 1;
            let i2 = (j + 1) * verts_per_edge + i;
            let i3 = (j + 1) * verts_per_edge + i + 1;

            // Two triangles: (i0, i2, i1) and (i1, i2, i3).
            indices.extend([i0 as u32, i2 as u32, i1 as u32]);
            indices.extend([i1 as u32, i2 as u32, i3 as u32]);

            // Compute face normal using (p2 - p0) × (p1 - p0) so upward-facing normals point +Y.
            let p0 = Vec3::from(positions[i0 as usize]);
            let p1 = Vec3::from(positions[i1 as usize]);
            let p2 = Vec3::from(positions[i2 as usize]);
            let face_norm = (p2 - p0).cross(p1 - p0).normalize_or_zero();

            for &idx in &[i0, i1, i2, i3] {
                let current = Vec3::from(normals[idx as usize]);
                let updated = (current + face_norm).normalize_or_zero();
                normals[idx as usize] = updated.into();
            }
        }
    }

    // 3) Colour vertices by height & steepness with blending from TerrainColor list.
    let mut colors = Vec::<[f32; 4]>::with_capacity(normals.len());
    for (idx, n) in normals.iter().enumerate() {
        let y = positions[idx][1];
        let normalized_height = (y / max_height).clamp(0.0, 1.0);
        let ny = n[1].clamp(0.0, 1.0);
        let steepness = 1.0 - ny; // steeper slopes → higher steepness

        // Accumulate weighted color contributions.
        let mut total_weight = 0.0;
        let mut accum_color = Vec3::ZERO;
        for tc in terrain_colors.iter() {
            // Compute weight from slope:
            let s = steepness;
            let sl = tc.slope_limits;
            let weight_slope = if s <= sl[0] || s >= sl[3] {
                0.0
            } else if s < sl[1] {
                // fade in
                (s - sl[0]) / (sl[1] - sl[0])
            } else if s <= sl[2] {
                // solid region
                1.0
            } else {
                // fade out
                (sl[3] - s) / (sl[3] - sl[2])
            };

            // Compute weight from height:
            let h = normalized_height;
            let hl = tc.height_limits;
            let weight_height = if h <= hl[0] || h >= hl[3] {
                0.0
            } else if h < hl[1] {
                // fade in
                (h - hl[0]) / (hl[1] - hl[0])
            } else if h <= hl[2] {
                // solid region
                1.0
            } else {
                // fade out
                (hl[3] - h) / (hl[3] - hl[2])
            };

            // Combined weight = slope * height
            let w = weight_slope * weight_height;
            if w > 0.0 {
                accum_color += tc.color * w;
                total_weight += w;
            }
        }

        // If total_weight > 0, normalize; else fallback to flat grass:
        let final_color = if total_weight > 0.0 {
            accum_color / total_weight
        } else {
            Vec3::new(0.10, 0.60, 0.10) // default grass
        };

        colors.push([final_color.x, final_color.y, final_color.z, 1.0]);
    }

    // 4) Assemble into a `Mesh`.
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,    colors);
    mesh.insert_indices(Indices::U32(indices));

    TileResult { coord, mesh }
}

/// Compute height at (world_x, world_z) using `simplex_noise_2d_seeded`:
/// 1) noise returns in [-1..1]   
/// 2) clamp to [0..1], drop negatives  
/// 3) sum multiple octaves, clamp final to [0..1], multiply by max_height
fn height_at(x: f32, z: f32, max_height: f32) -> f32 {
    let mut output: f32 = 0.0;
    // Four octave layers, each clamped to control amplitude range
    
    let flat1 = noise_at(x, z, 0.00015,  0.1,  0.4, 1234.);
    let flat2 = noise_at(x, z, 0.00025, -0.2,  0.2, 2345.);
    let flat3 = noise_at(x, z, 0.0005,  -0.05, 0.05, 4567.);
    let flat = flat1 + flat2 + flat3;
    
    // let mtn1 = noise_at(x, z, 0.0002, -0.2,  0.3, 2345.);
    // let mtn2 = noise_at(x, z, 0.0003, -0.02,  0.1, 7766.);
    // let mtn3 = noise_at(x, z, 0.003, -0.005,  0.005, 9999.);
    // let mtn = mtn1 + mtn2 + mtn3;
    
    
    output = flat;  // + mtn;
    // Clamp accumulated noise to [0..1], then scale by max_height
    output.clamp(0.0, 1.0) * max_height
}

/// Sample seeded simplex noise at frequency `freq`, remap from [-1..1] → [min..max].
fn noise_at(x: f32, y: f32, freq: f32, min: f32, max: f32, seed: f32) -> f32 {
    let old_range = 1.0 - -1.0;
    let new_range = max - min;
    let noise = simplex_noise_2d_seeded(Vec2::new(x * freq, y * freq), seed);
    (((noise - -1.0) * new_range) / old_range) + min
}