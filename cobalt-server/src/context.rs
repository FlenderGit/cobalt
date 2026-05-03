use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, atomic::AtomicU32},
    time::Instant,
};

use cobalt_net::packet::server::KeepAlive;
use cobalt_protocol::{
    Encode, PacketId,
    codex::MinecraftCodex,
    crypto::{CryptoConfig, SessionCrypto},
    packet::{Packet, PacketError, RawPacket},
    types::{serialize::read_varint, varint::VarInt},
};
use cobalt_sdk::{Difficulty, Dimension, Gamemode};
use futures_util::stream::StreamExt;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, tcp::OwnedReadHalf},
    sync::RwLock,
};
use tokio_util::codec::FramedRead;
use tracing::info;

pub struct ConnContext<W: AsyncWriteExt> {
    pub pending_keepalive: Option<(i32, Instant)>,

    pub compression_threshold: Option<u32>,
    pub session_crypto: Option<SessionCrypto>,
    pub tx: W,
    pub server_state: Arc<ServerState>,
    // pub framed: FramedRead<OwnedReadHalf, MinecraftCodex>,
}

impl<W: AsyncWriteExt + Unpin> ConnContext<W> {
    pub async fn send_keepalive(&mut self) -> Result<(), PacketError> {
        let keepalive_id = VarInt::new(23);
        info!("Sending keepalive");
        let packet = KeepAlive::new(keepalive_id).to_packet()?;
        self.send_packet(packet).await?;
        self.pending_keepalive = Some((keepalive_id.val(), Instant::now()));
        Ok(())
    }

    pub async fn send_packet(&mut self, packet: Packet) -> Result<(), PacketError> {
        let raw = if let Some(compression_threshold) = self.compression_threshold {
            if packet.len() >= compression_threshold as usize {
                packet.compress(compression_threshold)?
            } else {
                let plain_raw = packet.to_raw();
                let data_length_zero = VarInt::new(0);
                let mut buf =
                    Vec::with_capacity(data_length_zero.len() as usize + plain_raw.data.len());
                data_length_zero.encode(&mut buf).expect("encode failed");
                buf.extend_from_slice(&plain_raw.data);
                RawPacket { data: buf }
            }
        } else {
            packet.to_raw()
        };

        let varint = VarInt::new(raw.len() as i32).to_bytes();

        if let Some(session_crypto) = &mut self.session_crypto {
            // Tout est chiffré, y compris le Packet Length
            let mut full = Vec::with_capacity(varint.len() + raw.data.len());
            full.extend_from_slice(&varint);
            full.extend_from_slice(&raw.data);
            let encrypted = session_crypto.encryptor.encrypt(&full)?;
            self.tx.write_all(&encrypted).await?;
        } else {
            self.tx.write_all(&varint).await?;
            self.tx.write_all(&raw.data).await?;
        }

        self.tx.flush().await?;
        Ok(())
    }

    // pub async fn read_packet(&mut self) -> Result<Option<RawPacket>, io::Error> {
    //     let Some(frame) = self.framed.next().await.transpose()? else {
    //         return Ok(None);
    //     };

    //     let decrypted = if let Some(session_crypto) = &mut self.session_crypto {
    //         session_crypto.decryptor.decrypt(frame)
    //     } else {
    //         frame
    //     }

    //     let mut payload = if let Some(treshold) = self.compression_threshold {
    //         Some(_) = todo!(),
    //         None => decrypted,
    //     };

    //     let (id, id_len) = read_varint(&payload)
    //         .ok_or_else(|| anyhow::anyhow!("invalid packet id"))?;
    //     payload.advance(id_len);

    //     Ok(Some(RawPacket { id: id as u8, data: payload }))
    // }
}

#[derive(Debug, Default)]
pub struct ServerState {
    pub config: ServerConfig,
    pub player_count: AtomicU32,
    pub players: RwLock<Vec<u8>>,
}

impl ServerState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct ServerConfig {
    pub addr: SocketAddr,
    pub name: String,
    pub protocol_version: u32,
    pub max_player_count: u32,
    pub description: String,
    pub favicon: Option<String>,
    pub gamemode: Gamemode,
    pub dimension: Dimension,
    pub difficulty: Difficulty,
    pub max_players: u8,
    pub threshold: u32,
    pub crypto: Option<CryptoConfig>,
}

impl ServerConfig {
    pub fn new(addr: SocketAddr, crypto: Option<CryptoConfig>) -> Self {
        Self {
            addr,
            crypto,
            ..Default::default()
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 25565)),
            name: String::new(),
            protocol_version: 47,
            max_player_count: 20,
            description: String::new(),
            favicon: None,
            gamemode: Gamemode {
                mode: cobalt_sdk::GamemodeKind::Creative,
                hardcore: false,
            },
            dimension: Dimension::Overworld,
            difficulty: Difficulty::Easy,
            max_players: 20,
            threshold: 256,
            crypto: None,
        }
    }
}
