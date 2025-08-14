// src/terrain/material.rs
use bevy::asset::Asset;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::pbr::{Material, MaterialPlugin};

pub struct TerrainMaterialPlugin;
impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        // In Bevy 0.16, add_plugins expects a tuple; this actually registers the material
        app.add_plugins((MaterialPlugin::<TerrainMaterial>::default(),));
    }
}

/// Per-tile params
#[derive(Clone, Copy, ShaderType, Default)]
pub struct TileParams {
    pub tile_size: f32,        // world X/Z span of the plane
    pub height_scale: f32,     // meters per height unit
    pub texels_per_side: u32,  // e.g. 129
    pub _pad: u32,
    pub tile_color: Vec4,      // RGBA
}

/// Terrain material with height + normal textures.
/// We will read them with `textureLoad` (no samplers needed).
#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct TerrainMaterial {
    #[uniform(0)]
    pub params: TileParams,

    // Heightmap: R32Float (non-filterable float texture)
    #[texture(1, sample_type = "float")]
    pub height_tex: Handle<Image>,

    // Normal map: Rgba8Unorm (we'll load as float and remap in shader)
    #[texture(2, sample_type = "float")]
    pub normal_tex: Handle<Image>,
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef    { "shaders/terrain.wgsl".into() }
    fn fragment_shader() -> ShaderRef  { "shaders/terrain.wgsl".into() }

    // Opt into depth prepass (uses the same file; the shader handles both via PREPASS_PIPELINE)
    fn prepass_vertex_shader() -> ShaderRef   { "shaders/terrain.wgsl".into() }
    fn prepass_fragment_shader() -> ShaderRef { "shaders/terrain.wgsl".into() }
}
