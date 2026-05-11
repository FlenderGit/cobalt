use std::io::{self, Read, Write};

use cobalt_derive::{DecodeTrait, EncodeTrait, Packet};
use cobalt_protocol::{
    Decode, Encode, RawBytesArray,
    types::{Byte, Double, Float, Integer, UShort, varint::VarInt},
};
use cobalt_sdk::{Difficulty, Dimension, GamemodeState, WorldType};
use uuid::Uuid;

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x01)]
// #[packet(0x23)]
pub struct JoinGame {
    entity_id: Integer,
    gamemode: GamemodeState,
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
    primary_bitmask: UShort,
    size: VarInt,
    data: RawBytesArray,
}

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x38)]
pub struct PlayerListItem {
    action: VarInt,
    players: Vec<PlayerListItemData>,
}

#[derive(Debug, EncodeTrait, DecodeTrait, derive_new::new)]
pub struct PlayerListItemData {
    uuid: Uuid,
    name: String,
    properties: Vec<Property>,
    gamemode: GamemodeState,
    ping: VarInt,
    display_name: Option<String>,
}

#[derive(Debug, EncodeTrait, DecodeTrait, derive_new::new, serde::Deserialize)]
pub struct Property {
    name: String,
    value: String,
    signature: Option<String>,
}

#[derive(Debug, Packet, derive_new::new)]
#[packet(0x1C)]
pub struct EntityMetadata {
    pub entity_id: VarInt,
    pub metadata: MetadataEntries,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetadataEntries(pub Vec<MetadataEntry>);

impl Encode for MetadataEntries {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for entry in &self.0 {
            entry.encode(writer)?;
        }
        writer.write_all(&[0x7F])?;
        Ok(())
    }
}

impl Decode for MetadataEntries {
    fn decode<R: Read + Unpin>(reader: &mut R) -> io::Result<Self> {
        let mut entries = Vec::new();
        while let Some(entry) = MetadataEntry::decode(reader)? {
            entries.push(entry);
        }
        Ok(MetadataEntries(entries))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetadataEntry {
    Byte(u8, u8),
    Short(u8, i16),
    Int(u8, i32),
    Float(u8, f32),
    Text(u8, String),
}

impl Encode for MetadataEntry {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            MetadataEntry::Byte(idx, val) => {
                // Type 0 << 5 | index
                writer.write_all(&[(0 << 5) | idx])?;
                writer.write_all(&[*val])?;
            }
            MetadataEntry::Short(idx, val) => {
                writer.write_all(&[(1 << 5) | idx])?;
                writer.write_all(&val.to_be_bytes())?;
            }
            MetadataEntry::Int(idx, val) => {
                writer.write_all(&[(2 << 5) | idx])?;
                writer.write_all(&val.to_be_bytes())?;
            }
            MetadataEntry::Float(idx, val) => {
                writer.write_all(&[(3 << 5) | idx])?;
                writer.write_all(&val.to_be_bytes())?;
            }
            MetadataEntry::Text(idx, val) => {
                writer.write_all(&[(4 << 5) | idx])?;
                val.encode(writer)?;
            }
        }
        Ok(())
    }
}

impl MetadataEntry {
    pub fn decode<R: Read + Unpin>(reader: &mut R) -> io::Result<Option<Self>> {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        let header = buf[0];
        if header == 0x7F {
            return Ok(None);
        }

        let entry_type = header >> 5;
        let index = header & 0x1F;

        match entry_type {
            0 => {
                let mut val = [0u8; 1];
                reader.read_exact(&mut val)?;
                Ok(Some(MetadataEntry::Byte(index, val[0])))
            }
            1 => {
                let mut val = [0u8; 2];
                reader.read_exact(&mut val)?;
                Ok(Some(MetadataEntry::Short(index, i16::from_be_bytes(val))))
            }
            2 => {
                let mut val = [0u8; 4];
                reader.read_exact(&mut val)?;
                Ok(Some(MetadataEntry::Int(index, i32::from_be_bytes(val))))
            }
            3 => {
                let mut val = [0u8; 4];
                reader.read_exact(&mut val)?;
                Ok(Some(MetadataEntry::Float(index, f32::from_be_bytes(val))))
            }
            4 => {
                let s = String::decode(reader)?;
                Ok(Some(MetadataEntry::Text(index, s)))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown metadata type",
            )),
        }
    }
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
