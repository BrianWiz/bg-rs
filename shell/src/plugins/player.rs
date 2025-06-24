use core::{
    client::ClientDebugDiagnostics,
    components::{Character, LocallyControlled, Velocity, Visuals},
    shared::GameState,
};

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::PrimaryWindow,
};

const CAMERA_Y_OFFSET: f32 = 6.0;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player_system);
        app.add_systems(FixedPreUpdate, spawn_visuals_system);
        app.add_systems(
            Update,
            (camera_follow_system, update_visuals_system).run_if(in_state(GameState::Playing)),
        );
        app.add_systems(
            Update,
            hud_metrics_system
                .run_if(resource_exists::<ClientDebugDiagnostics>)
                .run_if(resource_exists::<DiagnosticsStore>),
        );
    }
}

#[derive(Component)]
struct HUDMetricsText;

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

    commands
        .spawn(Node {
            position_type: PositionType::Relative,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        })
        .with_children(|parent| {
            // Prediction metrics text
            parent.spawn((
                HUDMetricsText,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.0),
                    right: Val::Px(10.0),
                    ..default()
                },
                Text::new("Prediction metrics..."),
            ));
        });
}

fn hud_metrics_system(
    diagnostics: Res<DiagnosticsStore>,
    prediction_metrics: Option<Res<ClientDebugDiagnostics>>,
    mut text_query: Query<&mut Text, With<HUDMetricsText>>,
) {
    if let Some(prediction_metrics) = prediction_metrics {
        if let Ok(mut text) = text_query.single_mut() {
            if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
                text.0 = format!(
                    "FPS: {}\nRollbacks: {}\nRollback Ticks: {}",
                    fps.smoothed().unwrap_or(0.0).round(),
                    prediction_metrics.rollback_count,
                    prediction_metrics.rollback_ticks
                );
            } else {
                text.0 = format!(
                    "Rollbacks: {}\nRollback Ticks: {}",
                    prediction_metrics.rollback_count, prediction_metrics.rollback_ticks
                );
            }
        }
    }
}

fn spawn_visuals_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_characters: Query<(Entity, &Transform), Added<Character>>,
) {
    for (entity, transform) in new_characters.iter() {
        commands.spawn((
            Visuals {
                target: entity,
                target_world_position: transform.translation,
                last_world_position: transform.translation,
            },
            Mesh3d::from(meshes.add(Sphere::new(0.5))),
            MeshMaterial3d(materials.add(Color::srgb(0.8, 0.1, 0.1))),
        ));
    }
}

fn update_visuals_system(
    time: Res<Time>,
    fixed_time: Res<Time<Fixed>>,
    characters_query: Query<(&Transform, &Velocity), With<Character>>,
    mut visuals_query: Query<(&mut Visuals, &mut Transform), Without<Character>>,
) {
    for (visuals, mut visuals_transform) in visuals_query.iter_mut() {
        if let Ok((character_transform, character_velocity)) = characters_query.get(visuals.target)
        {
            let target_position_extrapolated = character_transform.translation.lerp(
                character_transform.translation + character_velocity.0 * fixed_time.delta_secs(),
                fixed_time.overstep_fraction(),
            );

            visuals_transform.translation = visuals_transform
                .translation
                .lerp(target_position_extrapolated, 22.0 * time.delta_secs());
        }
    }
}

fn camera_follow_system(
    time: Res<Time>,
    mut camera: Query<(&mut Transform, &GlobalTransform, &Camera), With<Camera3d>>,
    window: Query<&Window, With<PrimaryWindow>>,
    character: Query<
        &GlobalTransform,
        (With<Character>, With<LocallyControlled>, Without<Camera3d>),
    >,
) {
    if let (Ok(window), Ok((mut camera_transform, camera_global_transform, camera))) =
        (window.single(), camera.single_mut())
    {
        if let Some(mouse_position) = window.cursor_position() {
            if let Ok(mouse_ray) =
                camera.viewport_to_world(&camera_global_transform, mouse_position)
            {
                if let Some(distance) =
                    mouse_ray.intersect_plane(Vec3::ZERO, InfinitePlane3d { normal: Dir3::Y })
                {
                    let mouse_world_position = mouse_ray.origin + (mouse_ray.direction * distance);

                    if let Ok(character_transform) = character.single() {
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

                        camera_transform.translation = camera_transform.translation.lerp(
                            limited_pos + (Vec3::Y * CAMERA_Y_OFFSET),
                            8.0 * time.delta_secs(),
                        );
                    }
                }
            }
        }
    }
}
