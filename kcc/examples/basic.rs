// use bevy::prelude::*;

// fn main() {
//     App::new()
//         .add_plugins(DefaultPlugins)
//         .add_systems(Startup, setup)
//         .run();
// }

// fn setup(mut commands: Commands) {
//     commands.spawn(Camera3d::default());
//     commands.spawn((DirectionalLight { ..default() }, Transform::default()));
// }

// fn spawn_cube(mut commands: Commands) {
//     commands.spawn((
//         Mesh3d::from(CuboidMesh::default()),
//         Transform3d::default(),
//         MaterialMesh3dBundle {
//             mesh: meshes.add(CuboidMesh::default()),
//             ..default()
//         },
//         Transform::default(),
//     ));
// }
