use bevy::prelude::*;
use crate::terrain::material::TerrainMaterialPlugin;
use crate::terrain::flatmesh::init_shared_mesh;
use crate::terrain::systems::{
    TerrainConfig, TerrainState,
    queue_and_spawn_tasks_system,
    collect_finished_tasks_system,
    garbage_collect_tiles_system,
};

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<TerrainConfig>()
            .init_resource::<TerrainState>()
            .add_plugins(TerrainMaterialPlugin) // <- this must be the new one
            .add_systems(Startup, init_shared_mesh)
            .add_systems(
                Update,
                (
                    queue_and_spawn_tasks_system,
                    collect_finished_tasks_system,
                    garbage_collect_tiles_system,
                ).chain(),
            );
    }
}
