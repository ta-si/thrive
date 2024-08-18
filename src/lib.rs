
use bevy::prelude::*;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "Thrive".to_string(),
                        ..default()
                    }
                    .into(),
                    ..default()
                })
        );
    }
}