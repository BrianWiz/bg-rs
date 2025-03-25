use bevy::prelude::*;

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct LocallyControlled;

#[derive(Component)]
pub struct RemoteControlled;

#[derive(Component)]
pub struct Character;

#[derive(Component)]
pub struct WishDirection(pub Vec3);