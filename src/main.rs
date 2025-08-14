// src/main.rs
mod camera;
use camera::{FreeFlightCamera, FreeFlightCameraPlugin};

mod terrain;
use terrain::TerrainPlugin;
use crate::terrain::systems::TileLoader;

use bevy::{
    pbr::Atmosphere, prelude::*, window::PresentMode
};
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Terrain Part 1".into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TerrainPlugin)
        .add_plugins(FreeFlightCameraPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Camera{hdr: true, ..default()},
        Transform::from_xyz(40.0, 45.0, 80.0).looking_at(Vec3::new(16.0, 0.0, 16.0), Vec3::Y),
        Atmosphere::EARTH,
        DistanceFog {
            falloff: FogFalloff::from_visibility_colors(
                4000.0, // distance in world units up to which objects retain visibility (>= 5% contrast)
                Color::srgb(1., 1., 1.), // atmospheric extinction color (after light is lost due to absorption by atmospheric particles)
                Color::srgb(1., 1., 1.), // atmospheric inscattering color (light gained due to scattering from the sun)
            ),
            ..default()
        },
        FreeFlightCamera::default(),
        TileLoader{radius_tiles: 6}
    ));

    // Directional Light
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform {
            rotation: Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                std::f32::consts::FRAC_PI_4,   // yaw: 45° toward the scene
                -std::f32::consts::FRAC_PI_4,  // pitch: 45° above horizon
            ),
            ..default()
        },
        Visibility::default(),
    ));

    // Sphere
    commands.spawn((
        Name::new("Sphere"),
        Mesh3d(meshes.add(Mesh::from(Sphere::new(0.5)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1., 1., 1.),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Visibility::default(),
    ));
}
