//! free_flight_camera.rs – Bevy 0.16.1
//! Add with `.add_plugins(FreeFlightCameraPlugin)`
//! Controls: RMB look | WASD move | Space up | Ctrl down | Shift boost | Esc release

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::transform::TransformSystem;
use bevy::window::{CursorGrabMode, PrimaryWindow};

pub struct FreeFlightCameraPlugin;
impl Plugin for FreeFlightCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (cursor_grab, flight_camera_move).chain().before(TransformSystem::TransformPropagate),
        );
    }
}

/// Tunables / state for a free-flight camera
#[derive(Component)]
pub struct FreeFlightCamera {
    pub speed:       f32, // units/s
    pub boost_speed: f32, // when Shift is held
    pub mouse_sens:  f32, // radians per pixel
    pub yaw:   f32,       // internal state
    pub pitch: f32,
}
impl Default for FreeFlightCamera {
    fn default() -> Self {
        Self {
            speed: 10.0,
            boost_speed: 50.0,
            mouse_sens: 0.0002,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

fn cursor_grab(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mouse:       Res<ButtonInput<MouseButton>>,
    keys:        Res<ButtonInput<KeyCode>>,
) {
    let Some(mut window) = windows.iter_mut().next() else { return };

    if mouse.just_pressed(MouseButton::Right) {
        window.cursor_options.visible   = false;
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
    }
    if mouse.just_released(MouseButton::Right) || keys.just_pressed(KeyCode::Escape) {
        window.cursor_options.visible   = true;
        window.cursor_options.grab_mode = CursorGrabMode::None;
    }
}

fn flight_camera_move(
    time:        Res<Time>,
    mouse:       Res<ButtonInput<MouseButton>>,
    mut motion:  EventReader<MouseMotion>,
    keys:        Res<ButtonInput<KeyCode>>,
    mut q_cam:   Query<(&mut Transform, &mut FreeFlightCamera)>,
) {
    let Some((mut transform, mut cam)) = q_cam.iter_mut().next() else { return };

    // Look
    if mouse.pressed(MouseButton::Right) {
        let mut delta = Vec2::ZERO;
        for ev in motion.read() { delta += ev.delta; }
        cam.yaw   -= delta.x * cam.mouse_sens;
        cam.pitch  = (cam.pitch - delta.y * cam.mouse_sens).clamp(-1.54, 1.54);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    }

    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { dir.z -= 1.0; }
    if keys.pressed(KeyCode::KeyS) { dir.z += 1.0; }
    if keys.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if keys.pressed(KeyCode::Space)        { dir.y += 1.0; }
    if keys.pressed(KeyCode::ControlLeft)  { dir.y -= 1.0; }
    if keys.pressed(KeyCode::ControlRight) { dir.y -= 1.0; }

    if dir != Vec3::ZERO {
        let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            cam.boost_speed
        } else { cam.speed };

        let rot = transform.rotation;
        transform.translation += rot * dir.normalize() * speed * time.delta_secs();
    }
}
