use core::{components::{Character, LocallyControlled}};

use bevy::{prelude::*, window::PrimaryWindow};

const CAMERA_Y_OFFSET: f32 = 6.0;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player_system);
        app.add_systems(FixedPreUpdate, spawn_visuals_system);
        app.add_systems(Update, camera_follow_system);
    }
}

fn setup_player_system(mut commands: Commands) {
    // spawn camera looking down
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 90.0f32.to_radians(),
            ..default()
        }),
        Transform::default()
            .with_translation(Vec3::new(0.0, CAMERA_Y_OFFSET, 0.0))
            // look down
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::PI * 0.5)),
    ));
}

fn spawn_visuals_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_characters: Query<Entity, Added<Character>>,
) {
    for entity in new_characters.iter() {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Mesh3d::from(meshes.add(Sphere::new(0.5))),
                MeshMaterial3d(materials.add(Color::srgb(0.8, 0.1, 0.1))),
            ));
        });
    }
}

fn camera_follow_system(
    time: Res<Time>,
    mut camera: Query<(&mut Transform, &GlobalTransform, &Camera), With<Camera3d>>,
    window: Query<&Window, With<PrimaryWindow>>,
    character: Query<&GlobalTransform, (With<Character>, With<LocallyControlled>, Without<Camera3d>)>,
) {
    if let (Ok(window), Ok((mut camera_transform, camera_global_transform, camera))) = (window.get_single(), camera.get_single_mut()) {
        if let Some(mouse_position) = window.cursor_position() {
            if let Ok(mouse_ray) = camera.viewport_to_world(&camera_global_transform, mouse_position) {
                if let Some(distance) = mouse_ray.intersect_plane(Vec3::ZERO, InfinitePlane3d { normal: Dir3::Y }) {
                    let mouse_world_position = mouse_ray.origin + (mouse_ray.direction * distance);

                    if let Ok(character_transform) = character.get_single() {
                        let char_pos = character_transform.translation();
                        
                        // First calculate the midpoint between character and mouse
                        let midpoint = (char_pos + mouse_world_position) * 0.5;
                        
                        // Then limit this midpoint's distance from character if needed
                        let to_midpoint = midpoint - char_pos;
                        let max_distance = CAMERA_Y_OFFSET;
                        
                        let limited_pos = if to_midpoint.length() > max_distance {
                            char_pos + to_midpoint.normalize() * max_distance
                        } else {
                            midpoint
                        };

                        camera_transform.translation = camera_transform.translation.lerp(limited_pos + (Vec3::Y * CAMERA_Y_OFFSET),  8.0 * time.delta_secs());
                    }
                }
            }
        }
    }
}
