#![allow(dead_code, unused)]

mod cache;
mod config;
mod context;
mod entity_manager;
mod mojang;
mod player_store;
mod server;
mod states;
mod world;

use std::io;
use tracing::info;

use crate::{
    config::{AccessControlData, ServerConfig},
    server::Server,
    world::world_manager::WorldManager,
};

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let config_file = "./res/config.toml";
    let config = ServerConfig::load(config_file).unwrap_or_else(|e| {
        eprintln!("[ERROR] Invalid configuration :\n  {e}");
        std::process::exit(1);
    });

    let access_control_file = "./res/access_control.toml";
    let access_control = AccessControlData::load(access_control_file).expect("access control");
    println!("Access control: {:?}", access_control);

    // Test
    let basepath = "./res/data";
    let world = WorldManager::new(basepath).expect("Create world manager");

    let server = Server::new(config, world).expect("Server init");
    tokio::select! {
        _ = server.run() => {},
        _ = shutdown_signal() => {
            tracing::info!("Signal received... Shutting down properly.");
        }
    }

    server.shutdown().await;
    info!("Clear shutdown executed. Shutting down server.");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();

        tokio::select! {
            _ = ctrl_c          => tracing::debug!("SIGINT catched"),
            _ = sigterm.recv()  => tracing::debug!("SIGTERM catched"),
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.unwrap();
        tracing::debug!("SIGINT catched");
    }
}
