
use thrive::camera::{FreeFlightCamera, FreeFlightCameraPlugin};

use bevy::{
    prelude::*,
    pbr::Atmosphere
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FreeFlightCameraPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Camera{hdr: true, ..default()},
        Transform {
            translation: Vec3::new(0.0, 3.0, 8.0),
            rotation: Quat::from_euler(EulerRot::ZYX, 0.0, 0.0, 0.0)
                * Quat::from_rotation_arc(Vec3::Y, (Vec3::ZERO - Vec3::new(0.0, 3.0, 8.0)).normalize()),
            ..default()
        },
        Atmosphere::EARTH,
        DistanceFog {
            falloff: FogFalloff::from_visibility_colors(
                4000.0, // distance in world units up to which objects retain visibility (>= 5% contrast)
                Color::srgb(1., 1., 1.), // atmospheric extinction color (after light is lost due to absorption by atmospheric particles)
                Color::srgb(1., 1., 1.), // atmospheric inscattering color (light gained due to scattering from the sun)
            ),
            ..default()
        },
        FreeFlightCamera::default()
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

    // Ground Plane
    commands.spawn((
        Name::new("Ground Plane"),
        Mesh3d(meshes.add(Mesh::from(Plane3d::new(
            Vec3::Y,
            Vec2::new(10000.0, 10000.0),
        )))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.35, 0.25),
            perceptual_roughness: 0.6,
            ..default()
        })),
        Transform::from_xyz(0., 0., 0.),
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