mod plugins;

use core::shared::SharedPlugins;
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};

fn main() {
    App::new()
        .add_plugins(SharedPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(plugins::player::PlayerPlugin)
        .run();
}

