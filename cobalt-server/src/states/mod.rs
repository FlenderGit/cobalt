use cobalt_protocol::{
    Encode, RawBytesArray,
    chunk::ChunkBuilder,
    crypto::{SessionCrypto, generate_pairs, generate_verify_token, minecraft_hash, rsa_decrypt},
};
use std::{io, sync::atomic::Ordering, time::Duration};

use cobalt_net::packet::{
    client::{
        ClientLoginPacket, ClientPlayPacket, ServerHandshakeClient, ServerStatusClient,
        parse_packet,
    },
    server::{ChatMessage, ChunkData, EncryptionRequest, LoginSuccess, SetCompression},
};
use cobalt_protocol::{
    PacketId,
    packet::{Packet, PacketError, RawPacket},
    types::varint::VarInt,
};
use cobalt_sdk::{Difficulty, Dimension, Gamemode, GamemodeKind, WorldType};
use enum_dispatch::enum_dispatch;
use tokio::io::AsyncWriteExt;
use tracing::{error, info, warn};

use crate::{context::ConnContext, mojang::verify_auth};

const FAVICON_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAIAAAAlC+aJAAADuklEQVR4nO2aT0gUURzHf+85SqcuLZIHKYkOg2WmWGFTh6QQTNAOgyYi4RxEUUEkXZa2TRFEiiAiSgZMzC4iEhVUBCEsYgUWVOyhQ13CAiMUJFsvHXRnnX1vZ957O+M+wd9BfvOb7/7mM+Pvzbx/qK5/Pmr2AgAAIECWU2mMzJlXt0Y2nFPG8FszSOorjKH3ZojUlxuDC2aY1JcakY9mhNSXGOFP5iCpLzZCX8yhFL1qBFEgoMGmIcThyKLHlprHkUiPpaIR0GOpaAT0SrJVSEAjoGcvoe2gEdAzlpCk9ACAtdYReWh49WprHwrkn7WOyc8KSvwhHUn0ziXk/CSk0Dt8yLJAI6BP9xbKDo2AXtnS+8g+jYCeLKFs0gjoMZfabxoBParrn4+OUfr36ZyL11+Bz/bmRj0jj3qlF1v0/M/Gb3PHiI3d4urMJemfD1QnjhGDwyhD58Izqb9zc8TGA8Ag26Z2omRCH8/bQ4uL2N44JT8LD9eQMl3QD2PlUZgLzu57zm9rLxw8AuMBh0gm5pDfiUfRWoai46EMWhUAQHnV+RMXqv+t/Y2vrT0dvb/8e4mTnjQmHrWlRxGit9mhkmMl2hnzWnA9Hj98vKy+o+vhQNhraDpPbPw2BoQAASCgObSglSRRqadr614/frQejwPA1w8Lf379zMnJ4YdH9uSsPArjvTrI8gsLF79/sw6fPLjHTU+7IUYesfGAzTDmf950E2mH1LeQay5bZGnxx/6DRZshhC51dPFhu+V35sFMdb/VSZBaqd69fFHVcFnJzQWAI5XahiPCn9LA2HgU9ntNV0Kf56L7Cgrahm+urqysriw/M0dFbiDthVx4FK15MDoRZlSns9npqdnpKU7cFKPmd+FRm7sxJz312fhk7hixiTuZrw94aLyVjCAxO81L7wM/dWmGgUdgfQAcgx4aE8+OHw+IrQ/4ZCLtkDozx0CPiEiGZmsDHDxYa4qwq+UooaSvNnXi6GRkx9W95cQm7wqsD5CnPDGH/E48DhNb1GDCQV7/N1Ivy8rjwXjAOxNph9SJLddcm5G85Dnk6tg/sJSzZH4WHtnWB0iNS9qUvhArRE3/DHFJ7wxx8GCtMcyu3nZz4VEb21HgQI0VIl4xu/uF/Nfv+P1CjJ05SenBi/FAlvW7+4WyrcdaQwgQcM9Ry6FXG9pQoKjWuhsZ9v/w6nf3C2Vbn25ia2fQg9D6gFx62cYD3Hqs6UGqulLvo/7spN5L1VfoPVR9md5N1ZfqnVT9Ub2dqi/W26j6//Ebk1V7Cs3tAAAAAElFTkSuQmCC";

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

#[derive(Debug)]
pub struct HandshakeState;
#[derive(Debug)]
pub struct StatusState;
#[derive(Debug)]
pub struct LoginState {
    username: Option<String>,
    verify_token: Option<[u8; 4]>,
}
#[derive(Debug)]
pub struct PlayState {
    username: String,
}

impl State for HandshakeState {
    async fn handle<W: AsyncWriteExt>(
        &mut self,
        _packet: Packet,
        _ctx: &mut ConnContext<W>,
    ) -> Result<Transition, PacketError> {
        match parse_packet::<ServerHandshakeClient>(&_packet)? {
            ServerHandshakeClient::PlayerIdentification {
                protocol_version,
                server_address,
                server_port,
                next_state,
            } => {
                info!(
                    "Player identification: protocol_version={}, server_address={}, server_port={}, next_state={:?}",
                    protocol_version, server_address, server_port, next_state
                );
                match next_state.val() {
                    1 => Ok(Transition::Next(StatusState.into())),
                    2 => Ok(Transition::Next(
                        LoginState {
                            verify_token: None,
                            username: None,
                        }
                        .into(),
                    )),
                    id => Err(PacketError::InvalidPacketId(id as u8)),
                }
            }
            _ => Err(PacketError::InvalidPacketId(2)),
        }
    }
}

impl State for StatusState {
    async fn handle<W: AsyncWriteExt + Unpin>(
        &mut self,
        _packet: Packet,
        _ctx: &mut ConnContext<W>,
    ) -> Result<Transition, PacketError> {
        match parse_packet::<ServerStatusClient>(&_packet)? {
            ServerStatusClient::Empty {} => {
                let server_config = &_ctx.server_state.config;
                let json = format!(
                    r#"{{"version":{{"name":"{}","protocol":{}}},"players":{{"max":{},"online":{},"sample":[]}},"description":{{"text":"{}"}},"favicon":"data:image/png;base64,{}"}}"#,
                    server_config.name,
                    server_config.protocol_version,
                    server_config.max_player_count,
                    _ctx.server_state.player_count.load(Ordering::Relaxed),
                    server_config.description,
                    FAVICON_B64,
                );
                let data = encode_string(&json);
                let packet = Packet::new(0x00, data);
                _ctx.tx.write_all(&packet.to_bytes()).await?;
                Ok(Transition::Same)
            }
            ServerStatusClient::Ping { payload } => {
                let mut data = Vec::new();
                data.extend_from_slice(&payload.to_be_bytes());
                let packet = Packet::new(0x01, data);
                _ctx.tx.write_all(&packet.to_bytes()).await?;
                Ok(Transition::Exit)
            }
            _ => Err(PacketError::InvalidPacketId(1)),
        }
    }
}

pub fn encode_string(s: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&VarInt::from(s.len() as i32).to_bytes());
    buf.extend_from_slice(s.as_bytes());
    buf
}

impl State for LoginState {
    async fn handle<W: AsyncWriteExt + Unpin>(
        &mut self,
        _packet: Packet,
        _ctx: &mut ConnContext<W>,
    ) -> Result<Transition, PacketError> {
        match parse_packet::<ClientLoginPacket>(&_packet)? {
            ClientLoginPacket::LoginStart { username } => {
                info!("LoginStart: {}", username);
                self.username = Some(username.clone());

                if let Some(crypto) = &_ctx.server_state.config.crypto {
                    info!("starting encryption");
                    info!("Public key size: {}", crypto.public_key_der.len());
                    let verify_token: [u8; 4] = generate_verify_token();
                    let packet = EncryptionRequest::new(
                        "".to_string(),
                        crypto.public_key_der.clone(),
                        verify_token.to_vec(),
                    );
                    _ctx.send_packet(packet).await?;

                    // Print size of public key

                    self.verify_token = Some(verify_token);
                    info!("fgeyz");
                    return Ok(Transition::Same);
                }

                _ctx.activate_compression().await?;

                // mode offline
                let uuid = format!(
                    "00000000-0000-3000-{:04x}-{:012x}",
                    username.len(),
                    username.bytes().fold(0u64, |a, b| a ^ b as u64)
                );

                let packet = LoginSuccess::new(uuid, username.clone());
                _ctx.send_packet(packet).await?;

                Ok(Transition::Next(PlayState { username }.into()))
            }
            ClientLoginPacket::EncryptionResponse {
                shared_secret,
                verify_token,
            } => {
                info!("EncryptionResponse: {:?}", shared_secret);

                let Some(session_verify_token) = self.verify_token else {
                    warn!("EncryptionResponse received but no verify token found");
                    return Ok(Transition::Same);
                };

                let Some(crypto) = &_ctx.server_state.config.crypto else {
                    warn!("EncryptionResponse received but no crypto context found");
                    return Ok(Transition::Same);
                };

                let Some(username) = &self.username else {
                    warn!("EncryptionResponse received but no username found");
                    return Ok(Transition::Same);
                };

                let decrypted_secret = rsa_decrypt(&crypto.private_key, &shared_secret)?;
                let decrypted_token = rsa_decrypt(&crypto.private_key, &verify_token)?;

                if decrypted_token != session_verify_token {
                    warn!("Verify token mismatch, kicking player");
                    return Ok(Transition::Same);
                }

                let key: [u8; 16] = decrypted_secret.try_into().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Invalid key length")
                })?;

                let hash = minecraft_hash(&key, &crypto.public_key_der);
                let Some(profile) = verify_auth(username, &hash).await else {
                    warn!("Auth failed, kicking player");
                    return Ok(Transition::Same);
                };

                pub fn format_uuid(uuid_no_dashes: &str) -> String {
                    if uuid_no_dashes.len() != 32 {
                        panic!("UUID must be 32 characters");
                    }

                    format!(
                        "{}-{}-{}-{}-{}",
                        &uuid_no_dashes[0..8],
                        &uuid_no_dashes[8..12],
                        &uuid_no_dashes[12..16],
                        &uuid_no_dashes[16..20],
                        &uuid_no_dashes[20..32]
                    )
                }

                let uuid = format_uuid(&profile.id);

                info!("Auth OK: {} ({})", profile.name, uuid);
                let (encryptor, decryptor) = generate_pairs(&key)?;
                _ctx.framed.decoder_mut().decryptor = Some(decryptor);
                _ctx.session_crypto = Some(SessionCrypto { encryptor });

                // Before chiffrement

                // Auth mojang
                // calculate hash from shared_token  and public key
                // send to mojang
                // if success, continue
                _ctx.activate_compression().await?;
                let packet = LoginSuccess::new(uuid, profile.name);
                _ctx.send_packet(packet).await?;

                _ctx.tx.flush().await?;

                Ok(Transition::Next(
                    PlayState {
                        username: username.clone(),
                    }
                    .into(),
                ))
            }
        }
    }
}

impl State for PlayState {
    async fn on_enter<W: AsyncWriteExt + Unpin>(
        &mut self,
        _ctx: &mut ConnContext<W>,
    ) -> Result<(), PacketError> {
        info!("OnEnter play");
        let packet = cobalt_net::packet::server::JoinGame::new(
            21,
            Gamemode::new(GamemodeKind::Creative),
            Dimension::Overworld,
            Difficulty::Normal,
            20,
            WorldType::Default,
            false,
        );

        _ctx.send_packet(packet).await?;

        // Player Position And Look
        let packet = cobalt_net::packet::server::PlayerPositionAndLook::new(
            0f64, 64f64, 0f64, 0f32, 0f32, 0,
            // VarInt::new(0),
        );
        _ctx.send_packet(packet).await?;

        // Send chunk test
        let chunk = ChunkBuilder {
            chunk_x: 0,
            chunk_z: 0,
            primary_bitmask: 0b0000_0000_0001_1111,
            include_sky_light: true,
        };
        let data = chunk.build_flat_world();

        let packet = ChunkData::new(
            0,
            0,
            true,
            0b0000_0000_0001_1111,
            VarInt::new(data.len() as i32),
            RawBytesArray(data),
        );
        // info!(
        //     "🧱 Sending ChunkData: size={}, first_bytes={:02x}{:02x}{:02x}{:02x}",
        //     raw.data.len(),
        //     raw.data[0],
        //     raw.data[1],
        //     raw.data[2],
        //     raw.data[3]
        // );
        // let raw = packet.to_packet()?;

        _ctx.send_packet(packet).await?;
        _ctx.tx.flush().await?;

        _ctx.server_state
            .player_count
            .fetch_add(1, Ordering::Relaxed);

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
                // info!(
                //     "Player position and look: x: {}, y: {}, z: {}, yaw: {}, pitch: {}, on_ground: {}",
                //     x, y, z, yaw, pitch, on_ground
                // );
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
                // info!(
                //     "Player position: x: {}, y: {}, z: {}, on_ground: {}",
                //     x, y, z, on_ground
                // );

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

pub enum Transition {
    Next(AState),
    Same,
    Exit,
}
