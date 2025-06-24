use crate::{
    character::CHARACTER_Y,
    components::{Character, WishDirection},
    net::SERVER_ID,
    plugins::character::spawn_character,
    shared::CommandLineArgs,
};
use bevy::prelude::*;
use bevy_renet2::prelude::*;
use clap::Parser;
use vleue_navigator::prelude::*;

use super::{server::GameServerState, shared::GameState};

const BOT_AVOIDANCE_DISTANCE: f32 = 3.0;
const BOT_REACTION_SPEED: f32 = 22.0;
const BOT_AVOIDANCE_PADDING: f32 = 0.15;
const BOT_AVOIDANCE_RADIUS: f32 = 1.0 + BOT_AVOIDANCE_PADDING; // Assuming character radius is 1.0
const BOT_APPROACH_DISTANCE: f32 = 5.0;

pub struct BotPlugin;

impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            spawn_bots_system, // .run_if(resource_exists::<RenetServer>)
                               // .run_if(resource_exists::<GameServerState>),
        );

        app.add_systems(
            FixedUpdate,
            enemy_move_and_attack_system
                .run_if(resource_exists::<RenetServer>)
                .run_if(resource_exists::<GameServerState>)
                .run_if(resource_exists::<Assets<NavMesh>>)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
pub struct Bot;

fn spawn_bots_system(mut commands: Commands, mut game_server_state: ResMut<GameServerState>) {
    info!("Spawning bots");
    // @todo-brian: using command line args here because the server doesnt exist yet... lol
    let args = CommandLineArgs::parse();
    if args.server {
        let bot_entity = spawn_character(
            &mut commands,
            Vec3::new(0.0, CHARACTER_Y, 0.0),
            false,
            game_server_state.pump_next_net_id(),
            SERVER_ID,
        );
        commands.entity(bot_entity).insert((Bot,));
    }
}

/// Sets the wish direction of the bot to move towards the player.
fn enemy_move_and_attack_system(
    fixed_time: Res<Time<Fixed>>,
    navmeshes: Res<Assets<NavMesh>>,
    navmesh: Query<&ManagedNavMesh>,
    player_characters: Query<&Transform, (With<Character>, Without<Bot>)>,
    mut bots: Query<(&Transform, &mut WishDirection), (With<Character>, With<Bot>)>,
) {
    let all_player_characters = player_characters
        .iter()
        .map(|p| p.translation)
        .collect::<Vec<_>>();

    if let Ok(managed_nav_mesh) = navmesh.single() {
        if let Some(navmesh) = navmeshes.get(managed_nav_mesh.id()) {
            for (bot_transform, mut bot_wish_direction) in bots.iter_mut() {
                if let Some(closest_enemy_translation) = all_player_characters
                    .iter()
                    .min_by_key(|p| bot_transform.translation.distance(**p) as u32)
                {
                    let start = Vec2::new(bot_transform.translation.x, bot_transform.translation.z);
                    let end = Vec2::new(closest_enemy_translation.x, closest_enemy_translation.z);

                    // First try to find a path
                    let new_wish_direction = if let Some(path) = navmesh.path(start, end) {
                        // Path found, move towards next path point
                        if let Some(next_point) = path.path.get(0) {
                            let target_position3d = Vec3::new(next_point.x, 0.0, next_point.y);

                            // If there's only one point in the path, use avoidance logic
                            // This is because it's got a clear line of sight to the enemy
                            if path.path.len() == 1 {
                                let to_player =
                                    closest_enemy_translation - bot_transform.translation;
                                let distance_to_player = to_player.length();

                                if distance_to_player <= BOT_AVOIDANCE_DISTANCE {
                                    // Too close, move away
                                    -to_player.normalize_or_zero()
                                } else if distance_to_player >= BOT_APPROACH_DISTANCE {
                                    // Too far, move towards
                                    to_player.normalize_or_zero()
                                } else {
                                    // In goldilocks zone, don't move
                                    Vec3::ZERO
                                }
                            } else {
                                // Multiple points, just follow the path
                                let direction_to_target =
                                    target_position3d - bot_transform.translation;
                                direction_to_target.normalize_or_zero()
                            }
                        } else {
                            Vec3::ZERO
                        }
                    } else {
                        Vec3::ZERO
                    };

                    // Ensure the new wish direction is on the XZ plane
                    let new_wish_direction =
                        Vec3::new(new_wish_direction.x, 0.0, new_wish_direction.z)
                            .normalize_or_zero();

                    // Interpolate towards the new wish direction
                    bot_wish_direction.0 = bot_wish_direction.0.lerp(
                        new_wish_direction,
                        (BOT_REACTION_SPEED * 0.5) * fixed_time.delta_secs(),
                    );
                }
            }
        }
    }
}
