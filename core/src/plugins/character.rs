use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::components::{Character, LocallyControlled, RemoteControlled, Velocity, WishDirection};

use super::shared::GameState;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, 
            (
                update_velocity_system,
                move_character_system
            )
            .chain()
            .run_if(in_state(GameState::Playing))
        );
    }
}

fn update_velocity_system(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(&mut Velocity, &WishDirection), With<Character>>,
) {
    for (mut velocity, wish_direction) in query.iter_mut() {
        velocity.0 = apply_friction(
            velocity.0, 
            velocity.0.length(), 
            10.0, 
            fixed_time.delta_secs()
        );

        let current_speed = velocity.0.length();
        velocity.0 += accelerate(
            wish_direction.0, 
            20.0, 
            current_speed, 2.0, 
            fixed_time.delta_secs()
        );
    }
}

fn move_character_system(
    fixed_time: Res<Time<Fixed>>,
    spatial_query: SpatialQuery,
    mut query: Query<(Entity,&mut Transform, &mut Velocity), With<Character>>,
) {
    for (entity, mut transform, mut velocity) in query.iter_mut() {
        move_character(&fixed_time, entity, &mut transform, &mut velocity, &spatial_query);
    }
}

////////////////////////////////////////////////////////
/// Moves the character based on the velocity.
/// Will collide with walls, and slide along them.
pub fn move_character(
    fixed_time: &Time<Fixed>,
    entity: Entity,
    transform: &mut Transform,
    velocity: &mut Velocity,
    spatial_query: &SpatialQuery,
) {
    const EPSILON: f32 = 0.001;

    let collider = Collider::sphere(0.5);

    let mut remaining_motion = velocity.0 * fixed_time.delta_secs();

    for _ in 0..4 {

        if let Some(hit) = spatial_query.cast_shape(
            &collider,
            transform.translation,
            Quat::default(),
            Dir3::new(remaining_motion.normalize_or_zero()).unwrap_or(Dir3::X),
            &ShapeCastConfig::from_max_distance(remaining_motion.length()),
            &SpatialQueryFilter::default().with_excluded_entities([entity]),
        ) {
            // Move to just before the collision point
            transform.translation += remaining_motion.normalize_or_zero() * hit.distance;

            // Prevents sticking
            transform.translation += hit.normal1 * EPSILON;

            // Deflect velocity along the surface
            velocity.0 -= hit.normal1 * velocity.0.dot(hit.normal1);
            remaining_motion -= hit.normal1 * remaining_motion.dot(hit.normal1);
        } else {
            // No collision, move the full distance
            transform.translation += remaining_motion;
            break;
        }
    }
}

////////////////////////////////////////////////////////
/// Spawns a character at the provided position.
/// If is_local is true, the character will be marked as locally controlled.
pub fn spawn_character(
    commands: &mut Commands,
    position: Vec3,
    is_local: bool,
) {
    // visuals are spawned in the shell app
    let new_entity = commands.spawn((
        Character,
        Velocity(Vec3::ZERO),
        WishDirection(Vec3::ZERO),
        Transform::default()
            .with_translation(position),
    )).id();

    if is_local {
        commands.entity(new_entity).insert(LocallyControlled);
    } else {
        commands.entity(new_entity).insert(RemoteControlled);
    }
}

////////////////////////////////////////////////////////
/// Applies friction to the velocity.
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