use thrive::camera::{FreeFlightCamera, FreeFlightCameraPlugin};

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
    picking::pointer::PointerInteraction
};

#[derive(Component)]
struct Ground;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_plugins(FreeFlightCameraPlugin)
        .add_systems(Startup, setup)
        .add_systems(PostUpdate, draw_mesh_intersections)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Create a texture (256x256 RGBA image with a solid red color)
    let size = Extent3d {
        width: 256,
        height: 256,
        depth_or_array_layers: 1,
    };
    let mut data = Vec::with_capacity((size.width * size.height * 4) as usize);
    for y in 0..size.height {
        for x in 0..size.width {
            let is_red = (x / 32 + y / 32) % 2 == 0;
            if is_red {
                data.extend_from_slice(&[255, 0, 0, 255]);
            } else {
                data.extend_from_slice(&[0, 0, 255, 255]);
            }
        }
    }
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        data.as_slice(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let texture_handle = images.add(image);

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5., 5.))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(texture_handle),
            ..default()
        })),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Ground
    ));

    // Light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        FreeFlightCamera::default()
    ));
}

fn draw_mesh_intersections(
    pointers: Query<&PointerInteraction>,
    ground_query: Query<(&MeshMaterial3d<StandardMaterial>, &Transform), With<Ground>>,
    materials: Res<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    mut gizmos: Gizmos,
) {
    let Ok((ground_material, ground_transform)) = ground_query.single() else { return; };
    let Some(material) = materials.get(&ground_material.0) else { return; };
    let Some(texture_handle) = material.base_color_texture.as_ref() else { return; };
    let Some(image) = images.get(texture_handle) else { return; };

    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        // Transform world point to ground local space
        let local = ground_transform.compute_matrix().inverse().transform_point3(point);

        // Plane is centered at (0,0,0), size 5x5
        let u = ((local.x / 5.0) + 0.5).clamp(0.0, 1.0);
        // Flip v to match image data orientation
        let v = (1.0 - ((-local.z / 5.0) + 0.5)).clamp(0.0, 1.0);

        let x = (u * (image.size().x as f32 - 1.0)).round() as usize;
        let y = (v * (image.size().y as f32 - 1.0)).round() as usize;
        let idx = (y * image.size().x as usize + x) * 4;

        let color = if let Some(pixel) = image.data.as_ref().and_then(|d| d.get(idx..idx + 4)) {
            Color::srgba_u8(pixel[0], pixel[1], pixel[2], pixel[3])
        } else {
            Color::WHITE
        };

        gizmos.sphere(point, 0.05, color);
        gizmos.arrow(point, point + normal.normalize() * 0.5, Color::srgb(1.0, 1.0, 0.0));
    }
}