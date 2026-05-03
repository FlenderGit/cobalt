use cobalt_derive::Packet;
use cobalt_protocol::{
    RawBytesArray, ToVec,
    types::{Byte, Double, Float, Integer, Long, Short, varint::VarInt},
};
use cobalt_sdk::{Difficulty, Dimension, Gamemode, WorldType};

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x01)]
// #[packet(0x23)]
pub struct JoinGame {
    entity_id: Integer,
    gamemode: Gamemode,
    dimension: Dimension,
    difficulty: Difficulty,
    max_players: Byte,
    level_type: WorldType,
    reduced_debug_info: bool,
}

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x02)]
pub struct LoginSuccess {
    uuid: String,
    username: String,
}

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x03)]
pub struct SetCompression {
    threshold: VarInt,
}

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x01)]
pub struct EncryptionRequest {
    server_id: String,
    public_key: Vec<u8>,
    verify_token: Vec<u8>,
}

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x08)]
pub struct PlayerPositionAndLook {
    x: Double,
    y: Double,
    z: Double,
    yaw: Float,
    pitch: Float,
    flags: Byte,
    // teleport_id: VarInt,
}

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x00)]
pub struct KeepAlive {
    keep_alive_id: VarInt,
}

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x02)]
pub struct ChatMessage {
    json_data: String,
    position: Byte,
}

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x21)]
pub struct ChunkData {
    chunk_x: Integer,
    chunk_z: Integer,
    ground_up_continuous: bool,
    primary_bitmask: Short,
    size: VarInt,
    data: RawBytesArray,
}

// #[derive(Debug, Packet, derive_new::new)]
// #[packet(0x21)]
// pub struct ChuckData {
//     chunk_x: Integer,
//     chunk_z: Integer,
//     ground_up_continuous: bool,
//     bitmask: Short,
//     // data: Vec<Byte>,
//     num_block_entities: VarInt,
//     // block_entities: Vec<Nbt>,
// }

// pub trait PackedId {
//     const PACKET_ID: u8;
// }

// impl PackedId for JoinGame {
//     const PACKET_ID: u8 = 0x01;
// }
