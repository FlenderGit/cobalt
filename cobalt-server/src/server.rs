use std::{
    io::{self},
    sync::Arc,
    time::Duration,
};

use cobalt_protocol::{
    codex::{DecryptingReader, MinecraftCodex},
    packet::{PacketError, RawPacket},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::FramedRead;
use tracing::{error, info, warn};

use crate::{
    config::ServerConfig,
    context::{ConnContext, ServerState},
    states::{AState, HandshakeState, State, Transition},
    world::world_manager::WorldManager,
};

pub struct Server {
    pub state: Arc<ServerState>,
}

impl Server {
    pub fn new(config: ServerConfig, world: WorldManager) -> io::Result<Self> {
        let state = ServerState::new(config, world)?;
        Ok(Self {
            state: Arc::new(state),
        })
    }

    pub async fn run(&self) -> Result<(), io::Error> {
        let listener = TcpListener::bind(self.state.config.network.addr).await?;

        println!("{}", self.state.config);

        info!("listening on {}", self.state.config.network.addr);

        // Thread thread that emit each 20/s
        // let state = self.state.clone();
        // tokio::spawn(async move {
        //     let mut interval = interval(Duration::from_secs_f64(1.0 / 20.0));
        //     loop {
        //         interval.tick().await;

        //         // Detecter les joueurs qui ont bougés

        //         // Obtenir la liste des joueurs qui voient le joueur

        //         // Envoyer les données aux joueurs

        //         // println!("tick");
        //     }
        // });

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            info!("new connection from {}", peer_addr);
            let state = self.state.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, state).await {
                    println!("{:?}", e);
                    error!("Client {peer_addr} error: {e}");
                }
                info!("Client {peer_addr} disconnected");
            });
        }
    }

    pub async fn shutdown(&self) {}
}

async fn handle_client(stream: TcpStream, state: Arc<ServerState>) -> Result<(), PacketError> {
    let (reader, writer) = stream.into_split();

    let framed = FramedRead::new(reader, MinecraftCodex::default());
    let mut ctx = ConnContext {
        tx: writer,
        server_state: state,
        compression_threshold: None,
        pending_keepalive: None,
        session_crypto: None,
        framed,
    };
    let mut state: AState = HandshakeState.into();

    let mut keepalive_interval: Option<tokio::time::Interval> = None;

    loop {
        tokio::select! {
            result = ctx.read_packet() => {
                match result? {
                    Some(packet) => {
                        match state.handle(packet, &mut ctx).await {
                            Ok(transition) => match transition {
                                Transition::Same => {}
                                Transition::Next(mut next) => {
                                    info!("Transition from {state:?} to {next:?}");
                                    next.on_enter(&mut ctx).await?;

                                    if next.is_play() {
                                        info!("Starting keepalive");
                                        keepalive_interval = Some(tokio::time::interval(Duration::from_secs(10)));
                                    }

                                    state = next;
                                }
                                Transition::Exit => break,
                            },
                            Err(e) => {
                                warn!("Error handling packet: {}", e);
                            }
                        }
                    }
                    None => {
                        info!("Connection closed");
                        break;
                    }
                }
            }
            _ = async {
                if let Some(ref mut i) = keepalive_interval {
                    i.tick().await;
                } else {
                    std::future::pending::<()>().await
                }
            } => {
                if let Err(e) = ctx.send_keepalive().await {
                    warn!("Failed to send keepalive: {}", e);
                    break;
                }
            }
        }
    }

    info!("Client disconnected, state: {state:?}");

    if state.is_play() {
        ctx.server_state
            .player_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    Ok(())
}
