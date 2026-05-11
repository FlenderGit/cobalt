use cobalt_net::packet::client::{ServerHandshakeClient, parse_packet};
use cobalt_protocol::packet::{Packet, PacketError};
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::{
    context::ConnContext,
    states::{State, Transition, login::LoginState, status::StatusState},
};

#[derive(Debug)]
pub struct HandshakeState;

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
                    2 => Ok(Transition::Next(LoginState::new(None, None).into())),
                    id => Err(PacketError::InvalidPacketId(id as u8)),
                }
            }
            _ => Err(PacketError::InvalidPacketId(2)),
        }
    }
}
