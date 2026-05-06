use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, atomic::AtomicU32},
    time::Instant,
};

use cobalt_net::packet::server::{KeepAlive, SetCompression};
use cobalt_protocol::{
    Encode, PacketId,
    codex::{DecryptingReader, MinecraftCodex},
    crypto::{CryptoConfig, SessionCrypto},
    packet::{
        Packet, PacketError, RawPacket, compress_payload, compress_zlib_bytes, decompress_packet,
    },
    types::{
        serialize::{read_varint, write_varint},
        varint::VarInt,
    },
};
use cobalt_sdk::{Difficulty, Dimension, Gamemode};
use futures_util::stream::StreamExt;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, tcp::OwnedReadHalf},
    sync::RwLock,
    time::Interval,
};
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::FramedRead,
};
use tracing::info;

pub struct ConnContext<W: AsyncWriteExt> {
    pub pending_keepalive: Option<(i32, Instant)>,

    pub compression_threshold: Option<u32>,
    pub session_crypto: Option<SessionCrypto>,
    pub tx: W,
    pub server_state: Arc<ServerState>,
    pub framed: FramedRead<OwnedReadHalf, MinecraftCodex>,
}

impl<W: AsyncWriteExt + Unpin> ConnContext<W> {
    pub async fn send_keepalive(&mut self) -> Result<(), PacketError> {
        let keepalive_id = VarInt::new(23);
        info!("Sending keepalive");
        let packet = KeepAlive::new(keepalive_id);
        self.send_packet(packet).await?;
        self.pending_keepalive = Some((keepalive_id.val(), Instant::now()));
        Ok(())
    }

    pub async fn send_packet(&mut self, packet: impl PacketId) -> io::Result<()> {
        let payload = packet.to_bytes()?;

        let mut inner = if let Some(threshold) = self.compression_threshold {
            compress_payload(payload, threshold)?
        } else {
            payload // Pas de compression
        };

        let mut full_frame = BytesMut::with_capacity(5 + inner.len());
        write_varint(&mut full_frame, inner.len() as i32); // ← Seul Packet Length
        full_frame.put(inner);

        if let Some(crypto) = &mut self.session_crypto {
            full_frame = crypto.encryptor.encrypt_bytes(&full_frame)?;
        }

        info!("Sending frame: {full_frame:?}");

        self.tx.write_all(&full_frame).await?;
        self.tx.flush().await?;
        Ok(())
    }

    pub async fn read_packet(&mut self) -> Result<Option<Packet>, io::Error> {
        let Some(frame) = self.framed.next().await.transpose()? else {
            return Ok(None);
        };

        // info!("Received frame: {frame:?}");

        Ok(Some(Packet::new_with_bytes(frame.id, frame.data)))
    }

    pub async fn activate_compression(&mut self) -> io::Result<()> {
        let treshhold = self.server_state.config.threshold;
        if treshhold == 0 {
            info!("Compression disabled");
            return Ok(());
        }
        let packet = SetCompression::new(VarInt::new(treshhold as i32));
        self.send_packet(packet).await?;
        self.compression_threshold = Some(treshhold);
        self.framed.decoder_mut().threshold = Some(treshhold);
        Ok(())
    }
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
