use avian3d::prelude::*;
use bevy::prelude::*;

pub struct Impact<'a> {
    pub surface_normal: Vec3,
    pub point: Vec3,
    pub distance: f32,
    pub fraction: f32,
    pub remaining_time: f32,
    pub velocity_before_impact: Vec3,
    pub velocity: &'a mut Vec3,
    pub transform: &'a mut Transform,
    pub hit_entity: Entity,
    pub hit_point: Vec3,
}

pub struct MoveAndSlideConfig {
    pub max_iterations: usize,
    pub epsilon: f32,
    pub collider: Collider,
    pub hooks: MoveAndSlideHooks,
    pub excluded_entities: Vec<Entity>,
}

impl Default for MoveAndSlideConfig {
    fn default() -> Self {
        Self {
            max_iterations: 4,
            epsilon: 0.001,
            collider: Collider::capsule(0.25, 1.75),
            hooks: MoveAndSlideHooks::default(),
            excluded_entities: Vec::new(),
        }
    }
}

pub struct MoveAndSlideHooks {
    pub on_impact: Option<fn(&mut Impact)>,
}

impl Default for MoveAndSlideHooks {
    fn default() -> Self {
        Self { on_impact: None }
    }
}

pub fn move_and_slide(
    config: MoveAndSlideConfig,
    delta_time: f32,
    entity: &Entity,
    transform: &mut Transform,
    velocity: &mut Vec3,
    spatial_query: &SpatialQuery,
) {
    let mut remaining_time = delta_time;
    let mut excluded_entities = config.excluded_entities;
    excluded_entities.push(*entity);

    for _ in 0..config.max_iterations {
        let wish_motion = *velocity * remaining_time;

        if let Some(hit) = spatial_query.cast_shape(
            &config.collider,
            transform.translation,
            transform.rotation,
            Dir3::new(wish_motion.normalize_or_zero()).unwrap_or(Dir3::X),
            &ShapeCastConfig::from_max_distance(wish_motion.length()),
            &SpatialQueryFilter::default()
                .with_excluded_entities(excluded_entities.iter().copied()),
        ) {
            let fraction = hit.distance / wish_motion.length();
            let velocity_before_impact = *velocity;

            // Move to just before the collision point
            transform.translation += wish_motion.normalize_or_zero() * hit.distance;

            // Prevents sticking
            transform.translation += hit.normal1 * config.epsilon;

            // Project velocity onto the surface plane
            *velocity = *velocity - (hit.normal1 * velocity.dot(hit.normal1));

            // User hook to compute the impact outcome
            if let Some(on_impact) = config.hooks.on_impact {
                on_impact(&mut Impact {
                    surface_normal: hit.normal1,
                    point: hit.point1,
                    distance: hit.distance,
                    fraction,
                    remaining_time,
                    velocity_before_impact,
                    velocity,
                    transform,
                    hit_entity: hit.entity,
                    hit_point: hit.point1,
                });
            }

            // Scale remaining time
            remaining_time *= 1.0 - fraction;
        } else {
            // No collision, move the full remaining distance
            transform.translation += wish_motion;
            break;
        }
    }
}
