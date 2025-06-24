use std::time::Duration;

use avian3d::prelude::Collider;
use bevy::{color::palettes, prelude::*, time::common_conditions::on_timer};
use bevy_trenchbroom::prelude::*;
use vleue_navigator::prelude::*;

use super::{character::CHARACTER_GROUND_MARGIN, shared::GameState};

/// The radius of the agent used to inflate obstacles in the navmesh.
const AGENT_RADIUS: f32 = 0.5;

/// Max size in bevy units (meters). Used for things like generating navmeshes.
const NAVMESH_MAX_WIDTH: f32 = 1024.0;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Worldspawn>();

        app.init_asset::<NavMesh>();
        app.add_plugins(VleueNavigatorPlugin);
        app.add_plugins(NavmeshUpdaterPlugin::<Collider, Worldspawn>::default());

        app.add_plugins(TrenchBroomPlugins(
            TrenchBroomConfig::new("brutal_grounds").assets_path("../assets"),
        ));

        //app.add_plugins(PhysicsDebugPlugin::default());
        app.add_systems(Startup, spawn_test_map);
        app.add_systems(
            Update,
            view_navmesh_system
                .run_if(on_timer(Duration::from_secs_f32(1.0)))
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn spawn_test_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // spawn a trenchbroom map
    commands.spawn(SceneRoot(asset_server.load("maps/unnamed.map#Scene")));

    // spawn directional light pointing south east
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::default()
            .with_translation(Vec3::new(-10.0, 10.0, -10.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // spawn a navmesh
    commands.spawn((
        ManagedNavMesh::from_id(0),
        NavMeshSettings {
            fixed: Triangulation::from_outer_edges(&[
                Vec2::new(-NAVMESH_MAX_WIDTH * 0.5, -NAVMESH_MAX_WIDTH * 0.5),
                Vec2::new(NAVMESH_MAX_WIDTH * 0.5, -NAVMESH_MAX_WIDTH * 0.5),
                Vec2::new(NAVMESH_MAX_WIDTH * 0.5, NAVMESH_MAX_WIDTH * 0.5),
                Vec2::new(-NAVMESH_MAX_WIDTH * 0.5, NAVMESH_MAX_WIDTH * 0.5),
            ]),
            build_timeout: Some(10.0),
            simplify: 0.005,
            merge_steps: 0,
            agent_radius: AGENT_RADIUS,
            ..default()
        },
        NavMeshUpdateMode::Direct,
        Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
            .with_translation(Vec3::new(0.0, CHARACTER_GROUND_MARGIN, 0.0)),
    ));
}

fn view_navmesh_system(
    mut commands: Commands,
    navmeshes: Query<Entity, With<ManagedNavMesh>>,
    mut current: Local<usize>,
) {
    for (i, entity) in navmeshes.iter().sort::<Entity>().enumerate() {
        commands.entity(entity).remove::<NavMeshDebug>();
        if i == *current {
            commands
                .entity(entity)
                .insert(NavMeshDebug(palettes::tailwind::RED_800.into()));
        }
    }
    *current = (*current + 1) % navmeshes.iter().len();
}

#[derive(SolidClass, Component, Reflect)]
#[reflect(QuakeClass, Component)]
#[spawn_hooks(SpawnHooks::new().smooth_by_default_angle().convex_collider())]
pub struct Worldspawn;
