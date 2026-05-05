use std::{
    io::{self, BufReader},
    sync::Arc,
    time::Duration,
};

use cobalt_protocol::{
    Decode,
    codex::{DecryptingReader, MinecraftCodex},
    packet::{PacketError, RawPacket},
    types::varint::VarInt,
};
use tokio::{
    net::{TcpListener, TcpStream},
    time::interval,
};
use tokio_util::codec::FramedRead;
use tracing::{error, info, warn};

use crate::{
    context::{ConnContext, ServerConfig, ServerState},
    states::{AState, HandshakeState, State, Transition},
};

pub struct Server {
    pub state: Arc<ServerState>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            state: Arc::new(ServerState::new(config)),
        }
    }

    pub async fn run(&self) -> Result<(), io::Error> {
        let listener = TcpListener::bind(self.state.config.addr).await?;
        info!("listening on {}", self.state.config.addr);

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
            // _ = async {
            //     if let Some(ref mut i) = keepalive_interval {
            //         i.tick().await;
            //     } else {
            //         std::future::pending::<()>().await
            //     }
            // } => {
            //     if let Err(e) = ctx.send_keepalive().await {
            //         warn!("Failed to send keepalive: {}", e);
            //         break;
            //     }
            // }
        }
    }

    // loop {
    //     tokio::select! {
    //         result = RawPacket::read_async(&mut reader) => {
    //             let mut packet = match result {
    //                 Err(PacketError::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof => break,
    //                 Err(e) => {
    //                     error!("Error reading packet: {e}");
    //                     continue;
    //                 },
    //                 Ok(packet) => packet,
    //             };

    //             if let Some(threshold) = ctx.compression_threshold {
    //                 let mut bufreader = BufReader::new(&packet.data[..]);
    //                 let varint = VarInt::decode(&mut bufreader)?;
    //                 let varint_size = varint.len() as usize;
    //                 let payload = &packet.data[varint_size..];

    //                 // info!("Compression threshold: {threshold}, varint: {varint:?}, size: {varint_size}");
    //                 if varint.val() == 0 {
    //                     packet = RawPacket { data: payload.to_vec() };
    //                 } else {
    //                     packet = RawPacket { data: vec![] };
    //                 }

    //             }

    //             info!("Packet: {:?}", packet);
    //             if let Some(session_crypto) = ctx.session_crypto.as_mut() {
    //                 session_crypto.decryptor.decrypt(&mut packet.data)?;
    //                 info!("Decrypted packet: {:?}", packet);

    //             }

    //             match state.handle(packet, &mut ctx).await {
    //                 Ok(transition) => match transition {
    //                     Transition::Same => {}
    //                     Transition::Next(mut next) => {
    //                         info!("Transition from {state:?} to {next:?}");
    //                         next.on_enter(&mut ctx).await.expect("3");
    //                         state = next;
    //                     }
    //                     Transition::Exit => break,
    //                 },
    //                 Err(e) => {
    //                     warn!("Error handling packet: {}", e);
    //                 }
    //             }
    //         }
    //         // _ = keepalive_interval.tick() => {
    //         //     if let Err(e) = ctx.send_keepalive().await {
    //         //         warn!("Failed to send keepalive: {}", e);
    //         //         break; // Client probablement déconnecté
    //         //     }
    //         // }
    //         else => break,
    //     }
    // }

    info!("Client disconnected, state: {state:?}");

    if state.is_play() {
        ctx.server_state
            .player_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    Ok(())
}
