use bevy::prelude::*;
use bevy_renet::renet::ClientId;

use crate::net::EntityNetId;

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct LocallyControlled;

#[derive(Component)]
pub struct RemoteControlled;

#[derive(Component)]
pub struct Character;

#[derive(Component)]
pub struct Visuals {
    pub target: Entity,
    pub target_world_position: Vec3,
    pub last_world_position: Vec3,
}

/// A wish direction is a direction that the entity wants to move in.
#[derive(Component)]
pub struct WishDirection(pub Vec3);

/// A replicated entity. All entities that are replicated over the network need this component.
#[derive(Component)]
pub struct ReplicatedEntity {
    /// A net id is a unique identifier for an entity over the network.
    pub net_id: EntityNetId,
    /// The renet client id of the owner of the entity.
    pub owner_client_id: ClientId,
}
