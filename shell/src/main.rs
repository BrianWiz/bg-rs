mod plugins;

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_trenchbroom::prelude::*;
use core::shared::SharedPlugins;

fn main() {
    let mut tb_config = TrenchBroomConfig::new("brutal_grounds");
    tb_config.assets_path = "../assets".into();

    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "../assets".to_string(),
            ..default()
        }))
        .add_plugins(TrenchBroomPlugin(tb_config))
        .add_plugins(SharedPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(plugins::player::PlayerPlugin)
        .run();
}
