use bevy::asset::Asset;
use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};

pub struct TerrainMaterialPlugin;
impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default());
    }
}

#[derive(Clone, Copy, ShaderType, Default)]
pub struct TileParams {
    pub tile_size: f32,
    pub height_scale: f32,
    pub texels_per_side: u32,
    pub _pad: u32,
    pub tile_color: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct TerrainMaterial {
    #[uniform(0)]
    pub params: TileParams,

    // Heightmap (R32Float). No sampler; we use textureLoad().
    #[texture(1, sample_type = "float")]
    pub height_tex: Handle<Image>,

    // Normal map (RGBA8Unorm). Bound now; used later for lighting.
    #[texture(2, sample_type = "float")]
    pub normal_tex: Handle<Image>,
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef { "shaders/terrain.wgsl".into() }
    fn fragment_shader() -> ShaderRef { "shaders/terrain.wgsl".into() }
}
