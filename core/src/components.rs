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
pub struct ReplicatedEntity {
    pub net_id: EntityNetId,
    pub owner_client_id: ClientId,
}

#[derive(Component)]
pub struct WishDirection(pub Vec3);