mod plugins;

use core::shared::SharedPlugins;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(SharedPlugins)
        .add_plugins(plugins::player::PlayerPlugin)
        .run();
}
