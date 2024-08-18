use bevy::prelude::*;
use thrive::AppPlugin;

fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}
