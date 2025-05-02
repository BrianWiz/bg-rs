use bevy::prelude::*;
use bevy_renet2::prelude::*;

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

#[derive(Component)]
pub struct Weapon {
    /// The number of ticks between each shot.
    pub fire_rate_ticks: u32,
    /// The force of the recoil, if any. Applied to the entity's velocity by pushing them back.
    pub recoil: Option<f32>,
}

#[derive(Component, Default)]
pub struct WeaponState {
    /// The next tick that the weapon can fire. Must be at this tick, or later for it to fire.
    pub next_fire_tick: u32,
}

/// Whether the entity wishes to fire their weapon or not.
#[derive(Component)]
pub struct WeaponWishFire(pub bool);

/// The yaw of the entity's aim. The direction they'd be shooting in.
#[derive(Component)]
pub struct AimYaw(pub f32);
