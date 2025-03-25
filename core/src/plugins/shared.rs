use avian3d::PhysicsPlugins;
use bevy::{pbr::PbrPlugin, prelude::*};

use super::{character::CharacterPlugin, game_mode::GameModePlugin, map::MapPlugin};

pub struct SharedPlugins;

const FIXED_TIME_STEP_HZ: f64 = 128.0;

impl Plugin for SharedPlugins {
    fn build(&self, app: &mut App) {
        #[cfg(not(feature = "headless"))]
        app.add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "../assets".to_string(),
            ..default()
        }));

        #[cfg(feature = "headless")]
        app.add_plugins(MinimalPlugins);

        app.insert_state(GameState::Loading);
        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins(CharacterPlugin);
        app.add_plugins(GameModePlugin);
        app.add_plugins(MapPlugin);
        app.add_systems(Startup, setup_shared_system);
        app.insert_resource(Time::<Fixed>::from_hz(FIXED_TIME_STEP_HZ));
    }
}

fn setup_shared_system(
    mut next_state: ResMut<NextState<GameState>>,
) {
    next_state.set(GameState::Playing);
}

/// The state of the game. Duh.
#[derive(States, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Playing,
    MainMenu,
}

