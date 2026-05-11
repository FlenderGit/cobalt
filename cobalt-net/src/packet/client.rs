use cobalt_derive::Packet;
use cobalt_protocol::{
    RawBytesArray,
    packet::{Packet, PacketError},
    types::{Byte, Long, Short, varint::VarInt},
};
use cobalt_sdk::{PluginChannel, Slot};

const BUFFER_SIZE: usize = 1024 * 8;

#[derive(Debug, Packet)]
pub enum ServerHandshakeClient {
    /// 0x00
    /// Sent by a player joining a server with relevant information.
    /// Current protocol version is 0x07.
    #[packet(0x00)]
    PlayerIdentification {
        protocol_version: Byte,
        server_address: String,
        server_port: Short,
        next_state: VarInt,
    },
}

#[derive(Debug, Packet)]
pub enum ServerStatusClient {
    /// 0x00
    /// Sent by a player joining a server with relevant information.
    /// Current protocol version is 0x07.
    #[packet(0x00)]
    Empty {},
    #[packet(0x01)]
    Ping { payload: Long },
}

#[derive(Debug, Packet)]
pub enum ClientLoginPacket {
    #[packet(0x00)]
    LoginStart { username: String },
    #[packet(0x01)]
    // Special inner struct because need use length of shared_secret and verify_token
    EncryptionResponse {
        shared_secret: Vec<u8>,
        verify_token: Vec<u8>,
    },
}

#[derive(Debug, Packet)]
pub enum ClientPlayPacket {
    #[packet(0x00)]
    KeepAlive { keep_alive_id: VarInt },
    #[packet(0x01)]
    ChatMessage { message: String },
    #[packet(0x03)]
    PlayerGround { on_ground: bool },
    #[packet(0x04)]
    PlayerPosition {
        x: f64,
        y: f64,
        z: f64,
        on_ground: bool,
    },
    #[packet(0x05)]
    PlayerLook {
        yaw: f32,
        pitch: f32,
        on_ground: bool,
    },
    #[packet(0x06)]
    PlayerPositionAndLook {
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
        on_ground: bool,
    },
    #[packet(0x07)]
    PlayerDigging {
        status: Byte,
        // location: Position,
        location: f64,
        face: Byte,
    },
    #[packet(0x09)]
    HeldItemChange { slot: Byte },
    #[packet(0x0b)]
    PlayerAnimation { entity_id: VarInt, animation: Byte },
    #[packet(0x0d)]
    CloseWindow { window_id: Byte },
    #[packet(0x10)]
    CreativeInventoryAction { slot: Short, clicked_item: Slot },
    #[packet(0x13)]
    PlayerAbility {
        flags: Byte,
        flying_speed: f32,
        walking_speed: f32,
    },
    #[packet(0x15)]
    ClientSettings {
        locale: String,
        view_distance: Byte,
        chat_mode: Byte,
        chat_colors: bool,
        displayed_skin_parts: u8,
    },
    #[packet(0x16)]
    ClientStatus { action_id: VarInt },
    #[packet(0x17)]
    PluginMessage {
        channel: PluginChannel,
        data: RawBytesArray,
    },
}

pub fn parse_packet<P: cobalt_protocol::DecodeWithId>(packet: &Packet) -> Result<P, PacketError> {
    let mut reader = packet.data.as_ref();
    P::decode_with_id(packet.id, &mut reader).map_err(PacketError::Io)
}
