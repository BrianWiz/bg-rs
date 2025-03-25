use bevy::prelude::*;
use bevy_renet::{renet::{RenetServer, ServerEvent}, RenetServerPlugin};

use crate::{components::{Character, ReplicatedEntity, WishDirection}, net::{start_server, ClientChannel, DespawnCharacterEvent, EntityNetId, PlayerInput, ServerChannel, SpawnCharacterEvent}};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(IdTracker {
            next_entity_net_id: 0,
        });
        app.add_plugins(RenetServerPlugin);
        app.add_systems(FixedUpdate, start_server_system);
        app.add_systems(FixedUpdate, 
            (
                handle_client_input_system,
                handle_connection_system,
            )
            .chain()
            .run_if(resource_exists::<RenetServer>)
        );
        app.add_event::<HostServerEvent>();
    }
}

#[derive(Resource)]
struct IdTracker {
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
                    position: Vec3::new(0.0, 2.0, 0.0),
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

