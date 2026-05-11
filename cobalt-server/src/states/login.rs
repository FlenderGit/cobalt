use std::io;

use cobalt_net::packet::{
    client::{ClientLoginPacket, parse_packet},
    server::{EncryptionRequest, LoginSuccess, PlayerListItem, PlayerListItemData},
};
use cobalt_protocol::{
    crypto::{SessionCrypto, generate_pairs, generate_verify_token, minecraft_hash, rsa_decrypt},
    packet::{Packet, PacketError},
    types::varint::VarInt,
};
use cobalt_sdk::{Gamemode, GamemodeState};
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    context::ConnContext,
    mojang::verify_auth,
    states::{State, Transition, play::PlayState},
};

#[derive(Debug, derive_new::new)]
pub struct LoginState {
    username: Option<String>,
    verify_token: Option<[u8; 4]>,
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

                // test _ctx.server_state.crtpo not config
                if let Some(crypto) = &_ctx.server_state.crypto {
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
                let uuid = Uuid::new_v3(&Uuid::NAMESPACE_OID, username.as_bytes());

                let packet = LoginSuccess::new(uuid.to_string(), username.clone());
                _ctx.send_packet(packet).await?;

                Ok(Transition::Next(PlayState::new(username).into()))
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

                let Some(crypto) = &_ctx.server_state.crypto else {
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
                    warn!("Auth failed, kicking playepublic_key_derr");
                    return Ok(Transition::Same);
                };

                // let uuid = format_uuid(&profile.id);
                let uuid = profile.id;

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
                let packet = LoginSuccess::new(uuid.to_string(), profile.name);
                _ctx.send_packet(packet).await?;

                // In login phase, send the PlayerInfo
                let packet = PlayerListItem::new(
                    VarInt::new(0),
                    vec![PlayerListItemData::new(
                        profile.id,
                        username.to_string(),
                        profile.properties,
                        GamemodeState::new(Gamemode::Survival),
                        VarInt::new(12),
                        None,
                    )],
                );
                _ctx.send_packet(packet).await?;

                _ctx.tx.flush().await?;

                Ok(Transition::Next(PlayState::new(username.clone()).into()))
            }
        }
    }
}
