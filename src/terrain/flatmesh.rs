use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use super::systems::TerrainConfig;

pub fn flat_grid_mesh(n: usize, size: f32) -> Mesh {
    let step = size / (n as f32 - 1.0);
    let mut positions = Vec::with_capacity(n*n);
    let mut uvs       = Vec::with_capacity(n*n);
    let mut normals   = Vec::with_capacity(n*n);
    let mut tangents  = Vec::with_capacity(n*n);

    for z in 0..n {
        for x in 0..n {
            positions.push([x as f32 * step, 0.0, z as f32 * step]);
            uvs.push([x as f32 / (n as f32 - 1.0), z as f32 / (n as f32 - 1.0)]);
            // Up normals, +X tangent with handedness +1 (needed by PBR vertex layout)
            normals.push([0.0, 1.0, 0.0]);
            tangents.push([1.0, 0.0, 0.0, 1.0]);
        }
    }

    let mut indices = Vec::with_capacity((n-1)*(n-1)*6);
    for z in 0..(n-1) {
        for x in 0..(n-1) {
            let i0 = (z*n + x) as u32;
            let i1 = i0 + 1;
            let i2 = i0 + n as u32;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i2, i1,  i2, i3, i1]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(), // <-- was RENDER_WORLD
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[derive(Resource, Default, Clone)]
pub struct SharedMeshes {
    pub flat: Handle<Mesh>,
}

pub fn init_shared_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    cfg: Res<TerrainConfig>,
) {
    let m = flat_grid_mesh(cfg.tile_resolution, cfg.tile_size);
    let h = meshes.add(m);
    commands.insert_resource(SharedMeshes { flat: h });
}
