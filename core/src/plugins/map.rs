use avian3d::prelude::{ColliderConstructor, PhysicsDebugPlugin};
use bevy::prelude::*;
use bevy_trenchbroom::{config::TrenchBroomConfig, prelude::*, TrenchBroomPlugin};

use super::shared::GameState;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        // app.add_plugins(TrenchBroomPlugin(
        //     TrenchBroomConfig::new("brutal_grounds")
        //         .register_class::<Worldspawn>()
        // ));
        app.add_plugins(PhysicsDebugPlugin::default());
        app.add_systems(OnEnter(GameState::Playing), spawn_test_map);
    }
}

fn spawn_test_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    //commands.spawn(SceneRoot(asset_server.load("maps/unnamed.map#Scene")));

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

    // spawn a floor
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(100.0, 1.0, 100.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.1, 0.1))),
        Transform::default()
            .with_translation(Vec3::new(0.0, -0.5, 0.0)),
        ColliderConstructor::ConvexHullFromMesh,
    ));

    // spawn a pillar
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.5, 0.5, 0.5))),
        Transform::default()
            .with_translation(Vec3::new(5.0, 0.5, 5.0)),
        ColliderConstructor::ConvexHullFromMesh,
    ));
}

#[derive(SolidClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().convex_collider())]
pub struct Worldspawn;
