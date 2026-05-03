mod context;
mod mojang;
mod server;
mod states;

use std::io;

use cobalt_protocol::{crypto::CryptoConfig, types::varint::VarInt};

use crate::{context::ServerConfig, server::Server};

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:25565".parse().unwrap();

    let public_key = include_bytes!("../res/public_key.der");
    let private_key = include_bytes!("../res/server_key.pem");

    let crypto = CryptoConfig::new(public_key, private_key).expect("Failed to load crypto config");
    let crypto = Some(crypto);
    // let crypto = None;

    let mut config = ServerConfig::new(addr, crypto);
    config.description = "A Cobalt server".to_string();

    Server::new(config).run().await
}
