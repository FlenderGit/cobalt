use std::{sync::atomic::Ordering, time::Duration};

use cobalt_net::packet::{
    client::{ClientPlayPacket, parse_packet},
    server::{ChatMessage, ChunkData, EntityMetadata, MetadataEntries, MetadataEntry},
};
use cobalt_protocol::{
    RawBytesArray,
    packet::{Packet, PacketError},
    types::varint::VarInt,
};
use cobalt_sdk::{Difficulty, Dimension, Gamemode, GamemodeState, WorldType};
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use crate::{
    context::ConnContext,
    states::{State, Transition},
    world::world_manager::ChunkKey,
};

#[derive(Debug, derive_new::new)]
pub struct PlayState {
    username: String,
}

impl State for PlayState {
    async fn on_enter<W: AsyncWriteExt + Unpin>(
        &mut self,
        _ctx: &mut ConnContext<W>,
    ) -> Result<(), PacketError> {
        info!("OnEnter play");

        let eid = _ctx.server_state.entity_manager.next_id();
        _ctx.server_state
            .player_count
            .fetch_add(1, Ordering::Relaxed);

        let profile = &_ctx.server_state.config.profile;

        let packet = cobalt_net::packet::server::JoinGame::new(
            eid,
            GamemodeState::new(profile.gamemode),
            profile.dimension,
            profile.difficulty,
            profile.max_players.min(u8::MAX as u32) as u8,
            WorldType::Default,
            false,
        );

        _ctx.send_packet(packet).await?;

        // Player Position And Look
        let packet = cobalt_net::packet::server::PlayerPositionAndLook::new(
            0f64, 100f64, 0f64, 0f32, 0f32, 0,
            // VarInt::new(0),
        );
        _ctx.send_packet(packet).await?;

        let packet = EntityMetadata::new(
            VarInt::new(eid),
            MetadataEntries(vec![MetadataEntry::Byte(10, 127)]),
        );
        _ctx.send_packet(packet).await?;

        for i in 1..10 {
            for j in 1..10 {
                let chunk = _ctx
                    .server_state
                    .world
                    .get_chunk(ChunkKey::new(i, j))
                    .expect("Loading chunk");
                info!("Test After");

                let ret = _ctx
                    .server_state
                    .world
                    .modify_chunk(&ChunkKey::new(i, j), |chunk| {
                        chunk.set_block(10, 90, 0, 5, 0);
                    });

                let (bitmask, data) = chunk.serialize_payload(true, true);
                let len = data.len();
                let packet = ChunkData::new(
                    chunk.x,
                    chunk.z,
                    true,
                    bitmask,
                    VarInt::new(len as i32),
                    RawBytesArray(data),
                );

                _ctx.send_packet(packet).await?;
            }
        }

        // info!(
        //     "🧱 Sending ChunkData: size={}, first_bytes={:02x}{:02x}{:02x}{:02x}",
        //     raw.data.len(),
        //     raw.data[0],
        //     raw.data[1],
        //     raw.data[2],
        //     raw.data[3]
        // );
        // let raw = packet.to_packet()?;

        _ctx.tx.flush().await?;

        Ok(())
    }

    async fn handle<W: AsyncWriteExt + Unpin>(
        &mut self,
        _packet: Packet,
        _ctx: &mut ConnContext<W>,
    ) -> Result<Transition, PacketError> {
        match parse_packet::<ClientPlayPacket>(&_packet)? {
            ClientPlayPacket::ChatMessage { message } => {
                info!("ChatMessage: {}", message);

                let json = format!(
                    r#"{{"text":"<{}> {}","color":"white"}}"#,
                    self.username, message
                );
                let packet = ChatMessage::new(json, 0);
                _ctx.send_packet(packet).await?;

                Ok(Transition::Same)
            }
            ClientPlayPacket::KeepAlive { keep_alive_id } => {
                info!("KeepAlive: {}", keep_alive_id.val());
                if let Some((expected_id, send_at)) = _ctx.pending_keepalive {
                    if keep_alive_id.val() != expected_id {
                        warn!(
                            "KeepAlive ID mismatch, expected {}, got {}",
                            expected_id,
                            keep_alive_id.val()
                        );
                        return Ok(Transition::Same);
                    }
                    if send_at.elapsed() > Duration::from_secs(30) {
                        warn!("KeepAlive took too long to respond");
                        return Ok(Transition::Same);
                    }
                    _ctx.pending_keepalive = None;
                    info!("KeepAlive took {}ms", send_at.elapsed().as_millis());
                }
                Ok(Transition::Same)
            }
            ClientPlayPacket::ClientSettings {
                locale,
                view_distance,
                chat_mode,
                chat_colors,
                displayed_skin_parts,
            } => {
                info!(
                    "Client settings: locale: {}, view_distance: {}, chat_mode: {:?}, chat_colors: {:?}, displayed_skin_parts: {:?}",
                    locale, view_distance, chat_mode, chat_colors, displayed_skin_parts
                );
                Ok(Transition::Same)
            }
            ClientPlayPacket::PluginMessage { channel, data } => {
                info!("Plugin message: channel: {}, data: {:?}", channel, data);
                Ok(Transition::Same)
            }
            ClientPlayPacket::PlayerPositionAndLook {
                x,
                y,
                z,
                yaw,
                pitch,
                on_ground,
            } => {
                info!(
                    "Player position and look: x: {}, y: {}, z: {}, yaw: {}, pitch: {}, on_ground: {}",
                    x, y, z, yaw, pitch, on_ground
                );
                Ok(Transition::Same)
            }
            ClientPlayPacket::PlayerGround { on_ground } => {
                // info!("Player ground: on_ground: {}", on_ground);

                Ok(Transition::Same)
            }
            ClientPlayPacket::PlayerLook {
                yaw,
                pitch,
                on_ground,
            } => {
                // info!(
                //     "Player look: yaw: {}, pitch: {}, on_ground: {}",
                //     yaw, pitch, on_ground
                // );
                Ok(Transition::Same)
            }
            ClientPlayPacket::PlayerPosition { x, y, z, on_ground } => {
                info!(
                    "Player position: x: {}, y: {}, z: {}, on_ground: {}",
                    x, y, z, on_ground
                );

                Ok(Transition::Same)
            }
            ClientPlayPacket::PlayerDigging {
                status,
                location,
                face,
            } => {
                info!(
                    "Player digging: status: {:?}, location: {:?}, face: {:?}",
                    status, location, face
                );
                Ok(Transition::Same)
            }
            _ => {
                warn!("Unhandled packet: {:?}", _packet);
                Ok(Transition::Same)
            }
        }
    }
}
