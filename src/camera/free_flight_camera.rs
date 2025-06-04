//! free_flight_camera.rs
//! Drop into your project and add with `.add_plugins(FreeFlightCameraPlugin)`
//! Controls (as in Blender / Unity scene view)
//!   RMB  · hold to look around (cursor locked)
//!   WASD · horizontal movement      Q / E · down / up
//!   Shift· speed boost              Esc   · release cursor

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};


pub struct FreeFlightCameraPlugin;
impl Plugin for FreeFlightCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (cursor_grab, flight_camera_move).chain(),
        );
    }
}


/// Component holding per-camera state / tunables
#[derive(Component)]
pub struct FreeFlightCamera {
    pub speed:         f32,   // normal speed (units / s)
    pub boost_speed:   f32,   // when ⇧ is held
    pub mouse_sens:    f32,   // radians per pixel
    pub yaw:   f32,           // internal state
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


// -----------------------------------------------------------------------------
// Cursor grab / release (RMB ⇄ Esc)
// -----------------------------------------------------------------------------
fn cursor_grab(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mouse:       Res<ButtonInput<MouseButton>>,
    keys:        Res<ButtonInput<KeyCode>>,
) {
    let mut window = if let Ok(w) = windows.single_mut() { w } else { return };

    // Enter fly-mode
    if mouse.just_pressed(MouseButton::Right) {
        window.cursor_options.visible   = false;
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
    }
    // Exit fly-mode
    if mouse.just_released(MouseButton::Right) || keys.just_pressed(KeyCode::Escape) {
        window.cursor_options.visible   = true;
        window.cursor_options.grab_mode = CursorGrabMode::None;
    }
}

// -----------------------------------------------------------------------------
// Look & move
// -----------------------------------------------------------------------------
fn flight_camera_move(
    time:        Res<Time>,
    mouse:       Res<ButtonInput<MouseButton>>,
    mut motion:  EventReader<MouseMotion>,
    keys:        Res<ButtonInput<KeyCode>>,
    mut q_cam:   Query<(&mut Transform, &mut FreeFlightCamera)>,
) {
    let Ok((mut transform, mut cam)) = q_cam.single_mut() else { return };

    // -------------------------------------------------- Look (only while RMB held)
    if mouse.pressed(MouseButton::Right) {
        let mut delta = Vec2::ZERO;
        for ev in motion.read() {
            delta += ev.delta;
        }
        cam.yaw   -= delta.x * cam.mouse_sens;
        cam.pitch -= delta.y * cam.mouse_sens;
        cam.pitch = cam.pitch.clamp(-1.54, 1.54); // avoid gimbal flip

        transform.rotation =
            Quat::from_rotation_y(cam.yaw) * Quat::from_rotation_x(cam.pitch);
    }

    // -------------------------------------------------- WASD / QE movement
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { dir.z -= 1.0; }
    if keys.pressed(KeyCode::KeyS) { dir.z += 1.0; }
    if keys.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if keys.pressed(KeyCode::KeyE) { dir.y += 1.0; }
    if keys.pressed(KeyCode::KeyQ) { dir.y -= 1.0; }

    if dir != Vec3::ZERO {
        let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            cam.boost_speed
        } else { cam.speed };
        let rotation = transform.rotation;
        transform.translation += rotation * dir.normalize()
            * speed * time.delta_secs();
    }
}