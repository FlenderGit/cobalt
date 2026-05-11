use cobalt_protocol::{
    Encode, RawBytesArray,
    chunk::ChunkBuilder,
    crypto::{SessionCrypto, generate_pairs, generate_verify_token, minecraft_hash, rsa_decrypt},
};
use serde::Serialize;
use std::{io, sync::atomic::Ordering, time::Duration};
use uuid::{Uuid, Variant};

use cobalt_net::packet::{
    client::{
        ClientLoginPacket, ClientPlayPacket, ServerHandshakeClient, ServerStatusClient,
        parse_packet,
    },
    server::{
        ChatMessage, ChunkData, EncryptionRequest, EntityMetadata, LoginSuccess, MetadataEntries,
        MetadataEntry, PlayerListItem, PlayerListItemData, SetCompression,
    },
};
use cobalt_protocol::{
    packet::{Packet, PacketError},
    types::varint::VarInt,
};
use cobalt_sdk::{Difficulty, Dimension, Gamemode, GamemodeState, WorldType};
use enum_dispatch::enum_dispatch;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use crate::{
    config::ProfileConfig,
    context::{ConnContext, ServerState},
    mojang::verify_auth,
    states::{login::LoginState, play::PlayState, status::StatusState},
    world::world_manager::ChunkKey,
};

mod handshake;
mod login;
mod play;
mod status;

pub use handshake::HandshakeState;

#[derive(Debug)]
#[enum_dispatch]
pub enum AState {
    Handshake(HandshakeState),
    Status(StatusState),
    Login(LoginState),
    Play(PlayState),
}

impl AState {
    pub fn is_play(&self) -> bool {
        matches!(self, AState::Play(_))
    }
}

#[enum_dispatch(AState)]
pub trait State {
    async fn handle<W: AsyncWriteExt + Unpin>(
        &mut self,
        packet: Packet,
        ctx: &mut ConnContext<W>,
    ) -> Result<Transition, PacketError>;
    async fn on_enter<W: AsyncWriteExt + Unpin>(
        &mut self,
        _ctx: &mut ConnContext<W>,
    ) -> Result<(), PacketError> {
        Ok(())
    }
}

pub enum Transition {
    Next(AState),
    Same,
    Exit,
}

pub fn encode_string(s: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&VarInt::from(s.len() as i32).to_bytes());
    buf.extend_from_slice(s.as_bytes());
    buf
}
