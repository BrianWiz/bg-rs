use bevy::prelude::*;

use crate::components::Character;

use super::{character::spawn_character, shared::GameState};

pub struct GameModePlugin;

impl Plugin for GameModePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing), 
            setup_game_mode
        );
        app.add_systems(
            OnExit(GameState::Playing),
            cleanup_game_mode
        );
    }
}

fn setup_game_mode(mut commands: Commands) {
    spawn_character(
        &mut commands,
        Vec3::new(0.0, 0.5 + 0.001, 0.0),
        true,
    );
}

fn cleanup_game_mode(
    mut commands: Commands,
    query: Query<Entity, With<Character>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
