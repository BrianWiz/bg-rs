use avian3d::prelude::SpatialQuery;
use bevy::prelude::*;
use bevy_renet::{
    RenetClientPlugin,
    netcode::{NetcodeClientPlugin, NetcodeClientTransport},
    renet::RenetClient,
};

use crate::{
    character::{move_character, update_character_velocity},
    components::{
        Ability, Character, LocallyControlled, ReplicatedEntity, UseAbility, Velocity,
        WishDirection,
    },
    net::{
        CharacterInput, ClientChannel, DespawnCharacterEvent, EntitySnapshot, InputId, PlayerInput,
        ServerChannel, SnapshotId, SpawnCharacterEvent, WorldSnapshot, connect_to_server,
    },
};

use super::shared::FIXED_TIME_STEP_HZ;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RenetClientPlugin, NetcodeClientPlugin));
        app.add_event::<ConnectToServerEvent>();
        app.insert_resource(GameClientState {
            input_history: Vec::new(),
            next_input_id: 0,
            last_world_snapshot_processed_id: None,
            is_connecting: false,
        });
        app.insert_resource(ClientDebugDiagnostics {
            rollback_count: 0,
            rollback_ticks: 0,
        });
        app.add_systems(
            FixedUpdate,
            (
                connect_to_server_system,
                handle_server_messages_system.run_if(resource_exists::<RenetClient>),
            ),
        );
        app.add_systems(
            FixedPreUpdate,
            weapons_abilities_movement_system.run_if(resource_exists::<RenetClient>),
        );
        app.add_systems(
            FixedPostUpdate,
            send_input_system.run_if(resource_exists::<RenetClient>),
        );
    }
}

#[derive(Resource)]
pub struct ClientDebugDiagnostics {
    pub rollback_count: u32,
    pub rollback_ticks: u32,
}

#[derive(Resource)]
struct GameClientState {
    input_history: Vec<PlayerInput>,
    next_input_id: InputId,
    last_world_snapshot_processed_id: Option<SnapshotId>,
    is_connecting: bool,
}

#[derive(Event)]
pub struct ConnectToServerEvent {
    pub server_ip: String,
    pub server_port: u16,
}

fn connect_to_server_system(
    mut commands: Commands,
    mut connect_to_server_event_reader: EventReader<ConnectToServerEvent>,
    mut game_client_state: ResMut<GameClientState>,
) {
    for event in connect_to_server_event_reader.read() {
        if game_client_state.is_connecting {
            info!("Already attempting to connect to server");
            continue;
        }

        game_client_state.is_connecting = true;
        match connect_to_server(&mut commands, &event.server_ip, &event.server_port) {
            Ok(_) => {
                info!(
                    "Connecting to server at {}:{}",
                    event.server_ip, event.server_port
                );
            }
            Err(e) => {
                error!("Failed to connect to server: {}", e);
                game_client_state.is_connecting = false;
            }
        }
    }
}

fn handle_server_messages_system(
    fixed_time: Res<Time<Fixed>>,
    client_transport: Res<NetcodeClientTransport>,
    mut client_debug_diagnostics: ResMut<ClientDebugDiagnostics>,
    mut game_client_state: ResMut<GameClientState>,
    mut renet_client: ResMut<RenetClient>,
    mut character_spawn_events: EventWriter<SpawnCharacterEvent>,
    mut character_despawn_events: EventWriter<DespawnCharacterEvent>,
    mut characters: Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            &mut WishDirection,
            &mut UseAbility,
            &mut Ability,
            &ReplicatedEntity,
        ),
        With<Character>,
    >,
    spatial_query: SpatialQuery,
) {
    while let Some(message) = renet_client.receive_message(ServerChannel::SpawnCharacter) {
        match bitcode::deserialize::<SpawnCharacterEvent>(&message) {
            Ok(SpawnCharacterEvent {
                net_id,
                client_id,
                position,
                is_local,
            }) => {
                info!("Received spawn character event: {}", net_id);
                character_spawn_events.send(SpawnCharacterEvent {
                    net_id,
                    client_id,
                    position,
                    is_local,
                });
            }
            Err(e) => {
                error!("Failed to deserialize message: {}", e);
            }
        }
    }

    while let Some(message) = renet_client.receive_message(ServerChannel::DespawnCharacter) {
        match bitcode::deserialize::<DespawnCharacterEvent>(&message) {
            Ok(DespawnCharacterEvent { net_id }) => {
                character_despawn_events.send(DespawnCharacterEvent { net_id });
            }
            Err(e) => {
                error!("Failed to deserialize message: {}", e);
            }
        }
    }

    while let Some(message) = renet_client.receive_message(ServerChannel::WorldSnapshot) {
        match bitcode::deserialize::<WorldSnapshot>(&message) {
            Ok(world_snapshot) => {
                try_apply_world_snapshot(
                    &fixed_time,
                    &mut client_debug_diagnostics,
                    &client_transport,
                    &mut game_client_state,
                    &world_snapshot,
                    &spatial_query,
                    &mut characters,
                );
            }
            Err(e) => {
                error!("Failed to deserialize message: {}", e);
            }
        }
    }
}

/// Handles weapons, abilities and movement.
/// We do this all in one system because weapons and movement go hand in hand.
fn weapons_abilities_movement_system(
    mut game_client_state: ResMut<GameClientState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut WishDirection, &mut UseAbility, &mut Ability), With<LocallyControlled>>,
) {
    for (mut wish_direction, mut use_ability, mut ability) in query.iter_mut() {
        wish_direction.0 = Vec3::ZERO;
        if keyboard_input.pressed(KeyCode::KeyW) {
            wish_direction.0 += Vec3::NEG_Z;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            wish_direction.0 += Vec3::Z;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            wish_direction.0 += Vec3::NEG_X;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            wish_direction.0 += Vec3::X;
        }

        wish_direction.0 = wish_direction.0.normalize_or_zero();

        use_ability.0 = false;

        // try fire ability
        if ability.ticks_until_ready == 0 && keyboard_input.pressed(KeyCode::Space) {
            use_ability.0 = true;
            ability.ticks_until_ready = FIXED_TIME_STEP_HZ as u32; // 1 second
        }

        // decrement the ticks until ability ready
        ability.ticks_until_ready = ability.ticks_until_ready.saturating_sub(1);

        let id = game_client_state.next_input_id;
        let acking_snapshot_id = game_client_state.last_world_snapshot_processed_id;

        game_client_state.input_history.push(PlayerInput {
            id,
            acking_snapshot_id,
            character_input: Some(CharacterInput {
                wish_direction: wish_direction.0,
                wish_yaw: 0.0,
                predicted_ability: use_ability.0,
                final_position: None,
            }),
            sends: 0,
        });
        game_client_state.next_input_id += 1;
    }

    // retain 1 second of inputs, we're running at 128hz (or whatever is configured, see FIXED_TIME_STEP_HZ)
    if game_client_state.input_history.len() > FIXED_TIME_STEP_HZ as usize {
        game_client_state.input_history.remove(0);
    }
}

fn send_input_system(
    mut renet_client: ResMut<RenetClient>,
    mut game_client_state: ResMut<GameClientState>,
    locally_controlled_characters: Query<&mut Transform, With<LocallyControlled>>,
) {
    let mut final_position = None;
    for transform in locally_controlled_characters.iter() {
        final_position = Some(transform.translation);
    }

    for input in game_client_state.input_history.iter_mut() {
        if input.sends < 1 {
            // for only the first send, set the final position
            if input.sends == 0 {
                if let Some(character_input) = input.character_input.as_mut() {
                    character_input.final_position = final_position;
                }
            }

            match bitcode::serialize(&input) {
                Ok(message) => {
                    renet_client.send_message(ClientChannel::Input, message);
                    input.sends += 1;
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                }
            }
        }
    }
}

fn try_apply_world_snapshot(
    fixed_time: &Time<Fixed>,
    client_debug_diagnostics: &mut ClientDebugDiagnostics,
    client_transport: &NetcodeClientTransport,
    game_client_state: &mut GameClientState,
    world_snapshot: &WorldSnapshot,
    spatial_query: &SpatialQuery,
    characters: &mut Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            &mut WishDirection,
            &mut UseAbility,
            &mut Ability,
            &ReplicatedEntity,
        ),
        With<Character>,
    >,
) {
    if let Some(last_world_snapshot_processed_id) =
        game_client_state.last_world_snapshot_processed_id
    {
        if world_snapshot.id <= last_world_snapshot_processed_id {
            debug!(
                "Skipping world snapshot {}, already processed newer snapshot {}",
                world_snapshot.id, last_world_snapshot_processed_id
            );
            return;
        }
    }

    // delete acked inputs, except the one just acked.
    if let Some(acked_input_id) = world_snapshot.acking_input_id {
        game_client_state
            .input_history
            .retain(|input| input.id >= acked_input_id);
    }

    // first query all the characters
    let mut all_characters = characters.iter_mut().collect::<Vec<_>>();

    for character_entity_snapshot in world_snapshot.character_entities.iter() {
        // find the character entity
        if let Some((
            entity,
            transform,
            velocity,
            wish_direction,
            use_ability,
            ability,
            replicated_entity,
        )) = all_characters
            .iter_mut()
            .find(|(_, _, _, _, _, _, net_id)| net_id.net_id == character_entity_snapshot.id)
        {
            let is_local = replicated_entity.owner_client_id == client_transport.client_id();

            if is_local {
                if let Some(new_position) = character_entity_snapshot.position {
                    if let Some(acked_input_id) = world_snapshot.acking_input_id {
                        let acked_input = game_client_state
                            .input_history
                            .iter()
                            .find(|input| input.id == acked_input_id);

                        if let Some(acked_input) = acked_input {
                            if let Some(character_input) = acked_input.character_input.as_ref() {
                                if let Some(final_position) = character_input.final_position {
                                    let correction_distance = final_position.distance(new_position);
                                    if correction_distance > 0.1 {
                                        debug!(
                                            "Rolling back, correction distance: {}",
                                            correction_distance
                                        );
                                        client_debug_diagnostics.rollback_count += 1;
                                        transform.translation = new_position;

                                        if let Some(new_velocity) =
                                            character_entity_snapshot.velocity
                                        {
                                            velocity.0 = new_velocity;
                                        }

                                        let use_ability_before = use_ability.0;

                                        for input in game_client_state.input_history.iter_mut() {
                                            if input.id > acked_input_id {
                                                client_debug_diagnostics.rollback_ticks += 1;
                                                if let Some(character_input) =
                                                    input.character_input.as_mut()
                                                {
                                                    wish_direction.0 =
                                                        character_input.wish_direction;
                                                    use_ability.0 =
                                                        character_input.predicted_ability;
                                                    update_character_velocity(
                                                        fixed_time,
                                                        velocity,
                                                        wish_direction,
                                                        if use_ability.0 {
                                                            ability.recoil
                                                        } else {
                                                            None
                                                        },
                                                    );
                                                    move_character(
                                                        fixed_time,
                                                        entity,
                                                        transform,
                                                        velocity,
                                                        spatial_query,
                                                    );
                                                    character_input.final_position =
                                                        Some(transform.translation);
                                                }
                                            }
                                        }

                                        use_ability.0 = use_ability_before;
                                    } else {
                                        debug!("no correction needed");
                                    }
                                }
                            }
                        }
                    } else {
                        apply_entity_snapshot(transform, velocity, character_entity_snapshot);
                    }
                }
            } else {
                apply_entity_snapshot(transform, velocity, character_entity_snapshot);
            }
        }
    }

    game_client_state.last_world_snapshot_processed_id = Some(world_snapshot.id);
}

fn apply_entity_snapshot(
    transform: &mut Transform,
    velocity: &mut Velocity,
    entity_snapshot: &EntitySnapshot,
) {
    if let Some(new_position) = entity_snapshot.position {
        transform.translation = new_position;
    }

    if let Some(new_velocity) = entity_snapshot.velocity {
        velocity.0 = new_velocity;
    }
}
