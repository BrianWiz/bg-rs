use bevy::prelude::*;

use crate::components::Character;

use super::shared::GameState;

pub struct GameModePlugin;

impl Plugin for GameModePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_game_mode);
        app.add_systems(OnExit(GameState::Playing), cleanup_game_mode);
    }
}

fn setup_game_mode(mut _commands: Commands) {}

fn cleanup_game_mode(mut commands: Commands, query: Query<Entity, With<Character>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
