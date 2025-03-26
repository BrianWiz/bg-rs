use avian3d::PhysicsPlugins;
use bevy::prelude::*;
use clap::Parser;

use super::{
    bot::BotPlugin,
    character::CharacterPlugin,
    client::{ClientPlugin, ConnectToServerEvent},
    game_mode::GameModePlugin,
    map::MapPlugin,
    server::{HostServerEvent, ServerPlugin},
};

pub struct SharedPlugins;

pub const FIXED_TIME_STEP_HZ: f64 = 128.0;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, name = "Brutal Grounds", author = "Riverside Games")]
pub struct CommandLineArgs {
    #[arg(long)]
    pub server: bool,

    #[arg(long, default_value = "127.0.0.1")]
    pub server_ip: String,

    #[arg(long, default_value_t = 5000)]
    pub port: u16,
}

impl Plugin for SharedPlugins {
    fn build(&self, app: &mut App) {
        #[cfg(not(feature = "headless"))]
        app.add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "../assets".to_string(),
            ..default()
        }));

        #[cfg(feature = "headless")]
        app.add_plugins(MinimalPlugins);

        app.insert_resource(Time::<Fixed>::from_hz(FIXED_TIME_STEP_HZ));
        app.insert_state(GameState::Loading);

        app.add_plugins(ServerPlugin);
        app.add_plugins(ClientPlugin);

        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins(MapPlugin);

        app.add_plugins(CharacterPlugin);
        app.add_plugins(BotPlugin);

        app.add_plugins(GameModePlugin);
        app.add_systems(Startup, setup_shared_system);
    }
}

fn setup_shared_system(
    mut next_state: ResMut<NextState<GameState>>,
    mut host_server_events: EventWriter<HostServerEvent>,
    mut connect_to_server_events: EventWriter<ConnectToServerEvent>,
) {
    let args = CommandLineArgs::parse();

    if args.server {
        host_server_events.send(HostServerEvent { port: args.port });
    } else {
        connect_to_server_events.send(ConnectToServerEvent {
            server_ip: args.server_ip,
            server_port: args.port,
        });
    }

    next_state.set(GameState::Playing);
}

/// The state of the game. Duh.
#[derive(States, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Playing,
    MainMenu,
}
