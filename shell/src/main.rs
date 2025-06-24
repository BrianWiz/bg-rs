mod plugins;

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use core::shared::SharedPlugins;

fn main() {
    // turn on backtraces
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "full");
    }

    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "../assets".to_string(),
            ..default()
        }))
        .add_plugins(SharedPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(plugins::player::PlayerPlugin)
        .run();
}
