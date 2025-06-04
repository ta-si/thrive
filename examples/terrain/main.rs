use thrive::camera::{FreeFlightCamera, FreeFlightCameraPlugin};
use thrive::terrain::{TerrainManager, TerrainPlugin, TerrainSpawner};

use bevy::{
    prelude::*,
    pbr::Atmosphere
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FreeFlightCameraPlugin)
        .add_plugins(TerrainPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let shared_mat = materials.add(StandardMaterial {
        base_color:          Color::WHITE,
        perceptual_roughness: 0.6,
        ..default()
    });

    commands.insert_resource(TerrainManager {
        material_handle: shared_mat.clone(),
        max_height: 1000.,
        max_concurrent_tasks: 12,
        tile_size: 512.,
        resolution: 249,
        ..Default::default()
    });

    // camera
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Camera{hdr: true, ..default()},
        Transform {
            translation: Vec3::new(0., 750.0, 0.),
            rotation: Quat::from_euler(EulerRot::ZYX, 0.0, 0.0, 0.0),
            ..default()
        },
        Atmosphere::EARTH,
        DistanceFog {
            falloff: FogFalloff::from_visibility_colors(
                40000.0,
                Color::srgb(1., 1., 1.), 
                Color::srgb(1., 1., 1.),
            ),
            ..default()
        },
        FreeFlightCamera{
            speed: 1000.,
            boost_speed: 100.,
            ..default()
        },
        TerrainSpawner{ radius: 20, coord: IVec2::ZERO }
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

    // Water
    commands.spawn((
        Name::new("Ground Plane"),
        Mesh3d(meshes.add(Mesh::from(Plane3d::new(
            Vec3::Y,
            Vec2::new(100000.0, 100000.0),
        )))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.25, 0.25, 0.4, 0.1),
            perceptual_roughness: 0.01,
            ..default()
        })),
        Transform::from_xyz(0., 300., 0.),
        Visibility::default(),
    ));
}