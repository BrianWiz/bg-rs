use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy_renet2::prelude::*;

use crate::{
    components::{
        AimYaw, Character, LocallyControlled, RemoteControlled, ReplicatedEntity, Velocity, Weapon,
        WeaponState, WeaponWishFire, WishDirection,
    },
    net::{DespawnCharacterEvent, EntityNetId, SpawnCharacterEvent},
};

pub const CHARACTER_GROUND_MARGIN: f32 = 0.01;
pub const CHARACTER_PLANE: f32 = 0.5;
pub const CHARACTER_Y: f32 = CHARACTER_PLANE + CHARACTER_GROUND_MARGIN;
use super::shared::GameState;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnCharacterEvent>();
        app.add_event::<DespawnCharacterEvent>();
        app.add_systems(
            FixedUpdate,
            (
                spawn_character_system,
                despawn_character_system,
                update_velocity_system,
                move_character_system,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn spawn_character_system(
    mut commands: Commands,
    mut spawn_character_event_reader: EventReader<SpawnCharacterEvent>,
) {
    for event in spawn_character_event_reader.read() {
        spawn_character(
            &mut commands,
            event.position,
            event.is_local,
            event.net_id,
            event.client_id,
        );
    }
}

fn despawn_character_system(
    mut commands: Commands,
    mut despawn_character_event_reader: EventReader<DespawnCharacterEvent>,
    query: Query<(Entity, &ReplicatedEntity), With<Character>>,
) {
    for event in despawn_character_event_reader.read() {
        for (entity, net_id) in query.iter() {
            if net_id.net_id == event.net_id {
                despawn_character(&mut commands, entity);
            }
        }
    }
}

fn update_velocity_system(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(&mut Velocity, &WishDirection), With<Character>>,
) {
    for (mut velocity, wish_direction) in query.iter_mut() {
        update_character_velocity(&fixed_time, &mut velocity, wish_direction, None);
    }
}

fn move_character_system(
    fixed_time: Res<Time<Fixed>>,
    spatial_query: SpatialQuery,
    mut query: Query<(Entity, &mut Transform, &mut Velocity), With<Character>>,
) {
    for (entity, mut transform, mut velocity) in query.iter_mut() {
        move_character(
            &fixed_time,
            &entity,
            &mut transform,
            &mut velocity,
            &spatial_query,
        );
    }
}

////////////////////////////////////////////////////////
/// Updates the velocity based on the wish direction.
////////////////////////////////////////////////////////
pub fn update_character_velocity(
    fixed_time: &Time<Fixed>,
    velocity: &mut Velocity,
    wish_direction: &WishDirection,
    recoil: Option<f32>,
) {
    velocity.0 = apply_friction(
        velocity.0,
        velocity.0.length(),
        5.0,
        fixed_time.delta_secs(),
    );

    let current_speed = velocity.0.length();
    velocity.0 += accelerate(
        wish_direction.0,
        20.0,
        current_speed,
        2.0,
        fixed_time.delta_secs(),
    );

    if let Some(recoil) = recoil {
        velocity.0 = wish_direction.0 * recoil;
    }
}

////////////////////////////////////////////////////////
/// Moves the character based on the velocity.
/// Will collide with walls, and slide along them.
////////////////////////////////////////////////////////
pub fn move_character(
    fixed_time: &Time<Fixed>,
    entity: &Entity,
    transform: &mut Transform,
    velocity: &mut Velocity,
    spatial_query: &SpatialQuery,
) {
    const EPSILON: f32 = 0.001;
    const MAX_ITERATIONS: usize = 4;

    let collider = Collider::sphere(0.5);
    let mut remaining_time = fixed_time.delta_secs();

    for _ in 0..MAX_ITERATIONS {
        let wish_motion = velocity.0 * remaining_time;

        if let Some(hit) = spatial_query.cast_shape(
            &collider,
            transform.translation,
            Quat::default(),
            Dir3::new(wish_motion.normalize_or_zero()).unwrap_or(Dir3::X),
            &ShapeCastConfig::from_max_distance(wish_motion.length()),
            &SpatialQueryFilter::default().with_excluded_entities([*entity]),
        ) {
            // Move to just before the collision point
            transform.translation += wish_motion.normalize_or_zero() * hit.distance;

            // Prevents sticking
            transform.translation += hit.normal1 * EPSILON;

            // Project velocity onto the wall plane
            velocity.0 = velocity.0 - (hit.normal1 * velocity.0.dot(hit.normal1));

            // Scale remaining time based on collision fraction
            remaining_time *= 1.0 - hit.distance / wish_motion.length();
        } else {
            // No collision, move the full distance
            transform.translation += wish_motion;
            break;
        }
    }
}

////////////////////////////////////////////////////////
/// Spawns a character at the provided position.
/// If is_local is true, the character will be marked as locally controlled.
////////////////////////////////////////////////////////
pub fn spawn_character(
    commands: &mut Commands,
    position: Vec3,
    is_local: bool,
    net_id: EntityNetId,
    owner_client_id: ClientId,
) -> Entity {
    info!("Spawning character at: {:?}", position);

    // visuals are spawned in the shell
    let new_entity = commands
        .spawn((
            ReplicatedEntity {
                net_id,
                owner_client_id,
            },
            Character,
            Velocity(Vec3::ZERO),
            WishDirection(Vec3::ZERO),
            Weapon {
                fire_rate_ticks: 10,
                recoil: Some(1.0),
            },
            WeaponState::default(),
            WeaponWishFire(false),
            AimYaw(0.0),
            Transform::default().with_translation(position),
        ))
        .id();

    if is_local {
        commands.entity(new_entity).insert(LocallyControlled);
    } else {
        commands.entity(new_entity).insert(RemoteControlled);
    }

    new_entity
}

fn despawn_character(commands: &mut Commands, id: Entity) {
    commands.entity(id).despawn_recursive();
}

////////////////////////////////////////////////////////
/// Applies friction to the velocity.
////////////////////////////////////////////////////////
fn apply_friction(velocity: Vec3, current_speed: f32, drag: f32, delta_seconds: f32) -> Vec3 {
    let mut new_speed;
    let mut drop = 0.0;

    drop += current_speed * drag * delta_seconds;

    new_speed = current_speed - drop;
    if new_speed < 0.0 {
        new_speed = 0.0;
    }

    if new_speed != 0.0 {
        new_speed /= current_speed;
    }

    velocity * new_speed
}

////////////////////////////////////////////////////////
/// Accelerates the character towards the wish speed.
////////////////////////////////////////////////////////
fn accelerate(
    wish_direction: Vec3,
    wish_speed: f32,
    current_speed: f32,
    accel: f32,
    delta_seconds: f32,
) -> Vec3 {
    let add_speed = wish_speed - current_speed;

    if add_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let mut accel_speed = accel * delta_seconds * wish_speed;
    if accel_speed > add_speed {
        accel_speed = add_speed;
    }

    wish_direction * accel_speed
}
