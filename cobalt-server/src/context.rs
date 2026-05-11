use base64::Engine;
use std::{
    fs, io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::Path,
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
    sync::{RwLock, broadcast},
    time::Interval,
};
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::FramedRead,
};
use tracing::info;

use crate::{
    config::{AuthentificationConfig, ServerConfig},
    entity_manager::EntityManager,
    world::world_manager::WorldManager,
};

pub struct ServerContext {}

pub struct ConnContext<W: AsyncWriteExt> {
    pub pending_keepalive: Option<(i32, Instant)>,

    pub compression_threshold: Option<u32>,
    pub session_crypto: Option<SessionCrypto>,
    pub tx: W,
    pub server_state: Arc<ServerState>,
    pub framed: FramedRead<OwnedReadHalf, MinecraftCodex>,
    // pub event_rx: broadcast::Receiver<RawPacket>,
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

        let inner = if let Some(threshold) = self.compression_threshold {
            info!("Compress");
            compress_payload(payload, threshold)?
        } else {
            payload // Pas de compression
        };

        let mut full_frame = BytesMut::with_capacity(5 + inner.len());
        write_varint(&mut full_frame, inner.len() as i32); // ← Seul Packet Length
        full_frame.put(inner);

        if let Some(crypto) = &mut self.session_crypto {
            info!("Encrypt");
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
        let treshhold = self.server_state.config.network.threshold;
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
    pub entity_manager: Arc<EntityManager>,
    pub world: Arc<WorldManager>,

    pub crypto: Option<CryptoConfig>,
    pub favicon: Option<String>,
    // pub event_tx: broadcast::Sender<RawPacket>,
}

impl ServerState {
    pub fn new(config: ServerConfig, world: WorldManager) -> io::Result<Self> {
        // let (tx, _rx) = broadcast::channel(1024);

        let crypto = load_crypto_config(&config.auth)?;
        let favicon = if let Some(ref path) = config.profile.icon {
            load_favicon(path)?
        } else {
            None
        };

        Ok(Self {
            config,
            crypto,
            world: Arc::new(world),
            favicon,
            ..Default::default()
        })
    }
}

pub fn load_favicon(path: impl AsRef<Path>) -> io::Result<Option<String>> {
    let path = path.as_ref();

    // Si pas de chemin configuré → pas de favicon
    if !path.exists() {
        return Ok(None);
    }

    // Lecture du fichier PNG
    let png_bytes = fs::read(path)?;

    // Validation basique : header PNG
    if &png_bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} n'est pas un fichier PNG valide", path.display()),
        ));
    }

    // Encodage base64 (crate `base64`)
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

    // Format requis par le protocole Minecraft
    Ok(Some(format!("data:image/png;base64,{}", b64)))
}

pub fn load_crypto_config(auth: &AuthentificationConfig) -> io::Result<Option<CryptoConfig>> {
    if !auth.enabled {
        return Ok(None);
    }

    let (pub_path, priv_path) = auth
        .public_key
        .as_ref()
        .zip(auth.private_key.as_ref())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Two key don't have value"))?;

    let public_key_der = fs::read(&pub_path)?;
    let private_key_pem = fs::read(&priv_path)?;

    Ok(Some(CryptoConfig::new(&public_key_der, &private_key_pem)?))
}
