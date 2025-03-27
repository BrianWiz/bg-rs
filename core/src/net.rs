use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, SystemTime},
};

use bevy::prelude::*;
use bevy_renet2::netcode::{
    ClientAuthentication, NativeSocket, NetcodeClientPlugin, NetcodeClientTransport,
    NetcodeServerPlugin, NetcodeServerTransport, NetcodeTransportError, ServerAuthentication,
    ServerSetupConfig,
};
use bevy_renet2::prelude::*;

use serde::{Deserialize, Serialize};

use crate::{is_nearly_equal_f32, is_nearly_equal_vec3};

pub type SnapshotId = u32;
pub type InputId = u32;
pub type EntityNetId = u32;

pub const SERVER_ID: ClientId = 0;

#[derive(Serialize, Deserialize, Clone)]
pub struct WorldSnapshot {
    pub id: SnapshotId,
    pub acking_input_id: Option<InputId>,
    pub character_entities: Vec<EntitySnapshot>,
}

impl WorldSnapshot {
    pub fn diff(&self, other: &WorldSnapshot) -> WorldSnapshot {
        WorldSnapshot {
            id: self.id,
            acking_input_id: self.acking_input_id,
            character_entities: {
                let mut entities = Vec::new();
                for entity in self.character_entities.iter() {
                    if let Some(other_entity) =
                        other.character_entities.iter().find(|e| e.id == entity.id)
                    {
                        let diff = entity.diff(other_entity);
                        if !diff.is_empty() {
                            entities.push(diff);
                        }
                    } else {
                        entities.push(entity.clone());
                    }
                }
                entities
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntitySnapshot {
    pub id: EntityNetId,
    pub position: Option<Vec3>,
    pub velocity: Option<Vec3>,
    pub yaw: Option<f32>,
}

impl EntitySnapshot {
    pub fn diff(&self, other: &EntitySnapshot) -> EntitySnapshot {
        EntitySnapshot {
            id: self.id,
            position: if is_nearly_equal_vec3(self.position, other.position) {
                None
            } else {
                self.position
            },
            velocity: if is_nearly_equal_vec3(self.velocity, other.velocity) {
                None
            } else {
                self.velocity
            },
            yaw: if is_nearly_equal_f32(self.yaw, other.yaw) {
                None
            } else {
                self.yaw
            },
        }
    }

    fn is_empty(&self) -> bool {
        self.position.is_none() && self.velocity.is_none() && self.yaw.is_none()
    }
}

#[derive(Serialize, Deserialize, Event, Clone)]
pub struct SpawnCharacterEvent {
    pub net_id: EntityNetId,
    pub client_id: ClientId,
    pub position: Vec3,
    pub is_local: bool,
}

#[derive(Serialize, Deserialize, Event, Clone)]
pub struct DespawnCharacterEvent {
    pub net_id: EntityNetId,
}

#[derive(Serialize, Deserialize, Event, Clone)]
pub struct CharacterInput {
    //////////////////////////////////////////////
    // networked
    //////////////////////////////////////////////
    pub wish_direction: Vec3,
    pub aim_yaw: f32,
    pub predicted_ability: bool,
    pub weapon_wish_fire: bool,

    //////////////////////////////////////////////
    // non-networked
    //////////////////////////////////////////////
    #[serde(skip)]
    pub final_position: Option<Vec3>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerInput {
    pub id: InputId,
    pub acking_snapshot_id: Option<SnapshotId>,
    pub character_input: Option<CharacterInput>,
    pub sends: u8,
}

pub fn start_server(commands: &mut Commands, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
    info!("Attempting to bind server socket to: {}", socket_addr);

    let socket = UdpSocket::bind(socket_addr)?;
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;

    // Get the actual local address
    let local_addr = socket.local_addr()?;
    info!("Server socket successfully bound to: {}", local_addr);

    let server_config = ServerSetupConfig {
        current_time,
        max_clients: 64,
        protocol_id: 0,
        authentication: ServerAuthentication::Unsecure,
        socket_addresses: vec![vec![local_addr]],
    };

    let transport = NetcodeServerTransport::new(server_config, NativeSocket::new(socket)?)?;
    info!("Server transport created successfully");

    commands.insert_resource(transport);
    commands.insert_resource(RenetServer::new(connection_config()));
    info!("Server resources inserted successfully");
    Ok(())
}

pub fn connect_to_server(
    commands: &mut Commands,
    server_ip: &String,
    server_port: &u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let server_addr = SocketAddr::new(server_ip.parse()?, *server_port);
    let authentication = ClientAuthentication::Unsecure {
        server_addr,
        user_data: None,
        protocol_id: 0,
        socket_id: 0,
        client_id: current_time.as_millis() as u64,
    };

    let socket = UdpSocket::bind("127.0.0.1:0")?;
    let transport =
        NetcodeClientTransport::new(current_time, authentication, NativeSocket::new(socket)?)?;
    let client = RenetClient::new(connection_config(), false);

    commands.insert_resource(transport);
    commands.insert_resource(client);
    Ok(())
}

fn connection_config() -> ConnectionConfig {
    ConnectionConfig {
        server_channels_config: ServerChannel::config(),
        client_channels_config: ClientChannel::config(),
        available_bytes_per_tick: 64 * 1024,
    }
}

pub enum ClientChannel {
    Input,
}

impl From<ClientChannel> for u8 {
    fn from(channel: ClientChannel) -> Self {
        match channel {
            ClientChannel::Input => 0,
        }
    }
}

impl ClientChannel {
    pub fn config() -> Vec<ChannelConfig> {
        vec![ChannelConfig {
            channel_id: 0,
            max_memory_usage_bytes: 5 * 1024 * 1024,
            // send_type: SendType::ReliableOrdered {
            //     resend_time: Duration::from_millis(2),
            // },
            send_type: SendType::Unreliable,
        }]
    }
}

pub enum ServerChannel {
    SpawnCharacter,
    DespawnCharacter,
    WorldSnapshot,
}

impl From<ServerChannel> for u8 {
    fn from(channel: ServerChannel) -> Self {
        match channel {
            ServerChannel::SpawnCharacter => 0,
            ServerChannel::DespawnCharacter => 1,
            ServerChannel::WorldSnapshot => 2,
        }
    }
}

impl ServerChannel {
    pub fn config() -> Vec<ChannelConfig> {
        vec![
            // SpawnCharacter
            ChannelConfig {
                channel_id: 0,
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(10),
                },
            },
            // DespawnCharacter
            ChannelConfig {
                channel_id: 1,
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(10),
                },
            },
            // WorldSnapshot
            ChannelConfig {
                channel_id: 2,
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
        ]
    }
}
