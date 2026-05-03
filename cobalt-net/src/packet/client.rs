use std::io::{BufReader, Read};

use cobalt_derive::Packet;
use cobalt_protocol::{
    RawBytesArray,
    packet::{PacketError, RawPacket},
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
    // EntityRelativeMove {
    //     packed_id: u8,
    //     jump_boost: VarInt,
    //     dx: Byte,
    //     dy: Byte,
    //     dz: Byte,
    // },
    // CraftingBookData {
    //     packedid: u8,
    //     recipe_id: Integer,
    //     crafting_book_open: bool,
    //     crafting_filter: bool,
    // },
}

// impl ReadPacket for ClientPlayPacket {
//     fn from_raw<R: Read>(reader: &mut R) -> Result<Self, PacketError> {
//         let packed_id = read_byte(reader)?;

//         // match read_byte(reader)? {
//         //     0x00 => Ok(Self::KeepAlive {
//         //         keep_alive_id: read_int(reader)?,
//         //     }),
//         //     0x03 => Ok(Self::PlayerGround {
//         //         packet_id: 0x03,
//         //         on_ground: read_boolean(reader)?,
//         //     }),
//         //     0x04 => Ok(Self::PlayerPosition {
//         //         packet_id: 0x04,
//         //         x: read_double(reader)?,
//         //         y: read_double(reader)?,
//         //         z: read_double(reader)?,
//         //         on_ground: read_boolean(reader)?,
//         //     }),
//         //     0x05 => Ok(Self::PlayerLook {
//         //         packet_id: 0x05,
//         //         yaw: read_float(reader)?,
//         //         pitch: read_float(reader)?,
//         //         on_ground: read_boolean(reader)?,
//         //     }),
//         //     0x06 => Ok(Self::PlayerPositionAndLook {
//         //         packet_id: 0x06,
//         //         x: read_double(reader)?,
//         //         y: read_double(reader)?,
//         //         z: read_double(reader)?,
//         //         yaw: read_float(reader)?,
//         //         pitch: read_float(reader)?,
//         //         on_ground: read_boolean(reader)?,
//         //     }),
//         //     0x07 => Ok(Self::PlayerDigging {
//         //         packet_id: 0x07,
//         //         status: read_byte(reader)?,
//         //         location: read_double(reader)?,
//         //         face: read_byte(reader)?,
//         //     }),
//         //     0x09 => Ok(Self::HeldItemChange {
//         //         packet_id: 0x09,
//         //         slot: read_byte(reader)?,
//         //     }),
//         //     0x10 => Ok(Self::CreativeInventoryAction {
//         //         packet_id: 0x10,
//         //         slot: read_short(reader)?,
//         //         clicked_item: read_slot(reader)?,
//         //     }),
//         //     0x0d => Ok(Self::CloseWindow {
//         //         packet_id: 0x0d,
//         //         window_id: read_byte(reader)?,
//         //     }),
//         //     0x13 => Ok(Self::PlayerAbility {
//         //         packet_id: 0x13,
//         //         flags: read_byte(reader)?,
//         //         flying_speed: read_float(reader)?,
//         //         walking_speed: read_float(reader)?,
//         //     }),
//         //     0x15 => Ok(Self::ClientSettings {
//         //         packet_id: 0x15,
//         //         locale: read_string(reader)?,
//         //         view_distance: read_byte(reader)?,
//         //         chat_mode: read_byte(reader)?,
//         //         chat_colors: read_boolean(reader)?,
//         //         displayed_skin_parts: read_byte(reader)?,
//         //     }),
//         //     0x16 => Ok(Self::ClientStatus {
//         //         packet_id: 0x16,
//         //         action_id: VarInt::read_sync(reader)?,
//         //     }),
//         //     0x17 => {
//         //         let channel = read_string(reader)?;
//         //         let channel: PluginChannel = PluginChannel::try_from(channel)
//         //             .map_err(|_| PacketError::InvalidData(format!("Channel not found")))?;
//         //         let data = read_bytes(reader, 8)?;
//         //         Ok(Self::PluginMessage {
//         //             packet_id: 0x17,
//         //             channel,
//         //             data,
//         //         })
//         //     }
//         //     0x0b => Ok(Self::PlayerAnimation {
//         //         packet_id: 0x0b,
//         //         entity_id: VarInt::read_sync(reader)?,
//         //         animation: read_byte(reader)?,
//         //     }),

//         //     // 0x17 => Ok(Self::CraftingBookData {
//         //     //     packedid: 0x17,
//         //     //     recipe_id: read_int(reader)?,
//         //     //     crafting_book_open: read_boolean(reader)?,
//         //     //     crafting_filter: read_boolean(reader)?,
//         //     // }),
//         //     id => Err(PacketError::InvalidPacketId(id)),
//         // }
//     }
// }

pub fn parse_packet<P: cobalt_protocol::Decode>(raw_packet: &RawPacket) -> Result<P, PacketError> {
    let mut reader = BufReader::new(raw_packet.data.as_slice());
    P::decode(&mut reader).map_err(|e| PacketError::Io(e))
}
