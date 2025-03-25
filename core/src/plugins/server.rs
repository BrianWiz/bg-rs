use bevy::{ecs::world, prelude::*, utils::HashMap};
use bevy_renet::{netcode::NetcodeServerPlugin, renet::{ClientId, RenetServer, ServerEvent}, RenetServerPlugin};

use crate::{components::{Character, ReplicatedEntity, Velocity, WishDirection}, net::{start_server, ClientChannel, DespawnCharacterEvent, EntityNetId, EntitySnapshot, InputId, PlayerInput, ServerChannel, SnapshotId, SpawnCharacterEvent, WorldSnapshot}};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(IdTracker {
            next_world_snapshot_id: 0,
            next_entity_net_id: 0,
        });
        app.insert_resource(GameServerState {
            players: HashMap::new(),
        });
        app.add_plugins((
            RenetServerPlugin,
            NetcodeServerPlugin,
        ));
        app.add_systems(FixedUpdate, start_server_system);
        app.add_systems(FixedUpdate, 
            (
                handle_client_input_system,
                handle_connection_system,
            )
            .chain()
            .run_if(resource_exists::<RenetServer>)
        );
        app.add_systems(FixedPostUpdate, send_world_snapshot_system.run_if(resource_exists::<RenetServer>));
        app.add_event::<HostServerEvent>();
    }
}

struct Player {
    last_acked_world_snapshot_id: Option<SnapshotId>,
    last_processed_input_id: Option<InputId>,
}

#[derive(Resource)]
struct GameServerState {
    players: HashMap<ClientId, Player>,
}


#[derive(Resource)]
struct IdTracker {
    next_world_snapshot_id: SnapshotId,
    next_entity_net_id: EntityNetId,
}

#[derive(Event)]
pub struct HostServerEvent {
    pub port: u16,
}

fn start_server_system(
    mut commands: Commands,
    mut host_server_event_reader: EventReader<HostServerEvent>,
) {
    for event in host_server_event_reader.read() {
        match start_server(
            &mut commands,
            event.port,
        ) {
            Ok(_) => {
                info!("Server started successfully on port {}", event.port);
            }
            Err(e) => {
                error!("Failed to start server: {}", e);
            }
        }
    }
}

fn handle_client_input_system(
    mut game_server_state: ResMut<GameServerState>,
    mut renet_server: ResMut<RenetServer>,
    mut characters: Query<(&mut WishDirection, &ReplicatedEntity), With<Character>>,
) {
    for client_id in renet_server.clients_id() {
        if let Some(message) = renet_server.receive_message(client_id, ClientChannel::Input) {
            match bitcode::deserialize::<PlayerInput>(&message) {
                Ok(input) => {
                    for (mut wish_direction, replicated_entity) in characters.iter_mut() {
                        if replicated_entity.owner_client_id == client_id {
                            if let Some(character_input) = &input.character_input {
                                wish_direction.0 = character_input.wish_direction;

                                if let Some(player) = game_server_state.players.get_mut(&client_id) {
                                    player.last_processed_input_id = Some(input.id);
                                    player.last_acked_world_snapshot_id = input.acking_snapshot_id;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error deserializing message: {}", e);
                }
            }
        }
    }
}

fn handle_connection_system(
    mut game_server_state: ResMut<GameServerState>,
    mut id_tracker: ResMut<IdTracker>,
    mut renet_server: ResMut<RenetServer>,
    mut server_events: EventReader<ServerEvent>,
    mut character_spawn_events: EventWriter<SpawnCharacterEvent>,
    mut character_despawn_events: EventWriter<DespawnCharacterEvent>,
    characters: Query<(Entity, &Transform, &ReplicatedEntity), With<Character>>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                info!("Player {} connected", client_id);

                game_server_state.players.insert(*client_id, Player {
                    last_acked_world_snapshot_id: None,
                    last_processed_input_id: None,
                });

                // get every character and tell the new client to spawn them
                for (_, transform, net_id) in characters.iter() {
                    let message = SpawnCharacterEvent {
                        net_id: net_id.net_id,
                        client_id: net_id.owner_client_id,
                        position: transform.translation,
                        is_local: false,
                    };

                    match bitcode::serialize(&message) {
                        Ok(serialized) => {
                            renet_server.send_message(*client_id, ServerChannel::SpawnCharacter, serialized);
                        }
                        Err(e) => {
                            error!("Error serializing message: {}", e);
                        }
                    }
                }

                // spawn their character
                let character_spawn_event = SpawnCharacterEvent {
                    net_id: id_tracker.next_entity_net_id,
                    client_id: *client_id,
                    position: Vec3::new(0.0, 0.5, 0.0),
                    is_local: false,
                };
                character_spawn_events.send(character_spawn_event.clone());
                id_tracker.next_entity_net_id += 1;

                // tell every client about the new character
                for cid in renet_server.clients_id() {
                    let mut message = character_spawn_event.clone();

                    // We need to make sure that when we're sending to this new client,
                    // that we tell them that this is their character to control.
                    if *client_id == cid {
                        message.is_local = true;
                    }

                    match bitcode::serialize(&message) {
                        Ok(serialized) => {
                            renet_server.send_message(cid, ServerChannel::SpawnCharacter, serialized);
                        }
                        Err(e) => {
                            error!("Error serializing message: {}", e);
                        }
                    }
                }
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!("Player {} disconnected: {:?}", client_id, reason);

                game_server_state.players.remove(client_id);

                for (_, _, net_id) in characters.iter() {

                    if net_id.owner_client_id != *client_id {
                        continue;
                    }

                    let message = DespawnCharacterEvent {
                        net_id: net_id.net_id,
                    };

                    // tell ourselves to despawn their character
                    character_despawn_events.send(message.clone());

                    // tell everyone to despawn their character
                    for cid in renet_server.clients_id() {
                        match bitcode::serialize(&message) {
                            Ok(serialized) => {
                                renet_server.send_message(cid, ServerChannel::DespawnCharacter, serialized);
                        }
                        Err(e) => {
                                error!("Error serializing message: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn send_world_snapshot_system(
    mut id_tracker: ResMut<IdTracker>,
    mut renet_server: ResMut<RenetServer>,
    game_server_state: Res<GameServerState>,
    characters: Query<(&Transform, &Velocity, &ReplicatedEntity), With<Character>>,
) {
    let mut world_snapshot = WorldSnapshot {
        id: id_tracker.next_world_snapshot_id,
        acking_input_id: None,
        character_entities: Vec::new(),
    };

    id_tracker.next_world_snapshot_id += 1;

    for (transform, velocity, replicated_entity) in characters.iter() {
        world_snapshot.character_entities.push(EntitySnapshot {
            id: replicated_entity.net_id,
            position: Some(transform.translation),
            velocity: Some(velocity.0),
            yaw: Some(0.0),
        });
    }

    for cid in renet_server.clients_id() {

        let mut world_snapshot = world_snapshot.clone();

        if let Some(player) = game_server_state.players.get(&cid) {
            world_snapshot.acking_input_id = player.last_processed_input_id;
        }

        match bitcode::serialize(&world_snapshot) {
            Ok(serialized) => {
                renet_server.send_message(cid, ServerChannel::WorldSnapshot, serialized);
            }
            Err(e) => {
                error!("Error serializing message: {}", e);
            }
        }
    }
}
