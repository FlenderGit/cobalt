use std::sync::atomic::Ordering;

use cobalt_net::packet::client::{ServerStatusClient, parse_packet};
use cobalt_protocol::packet::{Packet, PacketError};
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use crate::{
    config::ProfileConfig,
    context::{ConnContext, ServerState},
    states::{State, Transition, encode_string},
};

#[derive(Debug)]
pub struct StatusState;

impl State for StatusState {
    async fn handle<W: AsyncWriteExt + Unpin>(
        &mut self,
        _packet: Packet,
        _ctx: &mut ConnContext<W>,
    ) -> Result<Transition, PacketError> {
        match parse_packet::<ServerStatusClient>(&_packet)? {
            ServerStatusClient::Empty {} => {
                let profile = &_ctx.server_state.config.profile;
                match build_status_json(profile, &_ctx.server_state) {
                    Ok(json) => {
                        let data = encode_string(&json);
                        let packet = Packet::new(0x00, data);
                        _ctx.tx.write_all(&packet.to_bytes()).await?;
                        Ok(Transition::Same)
                    }
                    Err(e) => Ok(Transition::Same),
                }
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

#[derive(Serialize)]
struct StatusResponse<'a> {
    version: VersionInfo,
    players: PlayersInfo,
    description: Description,
    #[serde(skip_serializing_if = "Option::is_none")]
    favicon: Option<&'a str>,
}

#[derive(Serialize)]
struct VersionInfo {
    name: String,
    protocol: u32,
}

#[derive(Serialize)]
struct PlayersInfo {
    max: u32,
    online: u32,
    sample: Vec<()>,
}

#[derive(Serialize)]
struct Description {
    text: String,
}

fn build_status_json(
    profile: &ProfileConfig,
    state: &ServerState,
) -> Result<String, serde_json::Error> {
    let response = StatusResponse {
        version: VersionInfo {
            name: profile.name.clone(),
            protocol: state.config.network.protocol_version,
        },
        players: PlayersInfo {
            max: profile.max_players,
            online: state.player_count.load(Ordering::Relaxed),
            sample: vec![],
        },
        description: Description {
            text: profile.description.clone(),
        },
        favicon: state.favicon.as_deref(), // Option<&str> → sérialisé ou omis
    };

    serde_json::to_string(&response)
}
