use crate::Encode;
use std::io::{self, Read, Write};

use flate2::{Compression, write::ZlibEncoder};
#[cfg(feature = "async")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::types::varint::VarInt;

#[derive(Debug, thiserror::Error)]
pub enum PacketError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Packet length cannot be negative: {0}")]
    NegativeLength(i64),

    #[error("Invalid packet id: {0}")]
    InvalidPacketId(u8),

    #[error("Invalid data: {0}")]
    InvalidData(String),
}

// enum ClientPacket {
//     Test,
// }

// impl ReadPacket for ClientPacket {
//     fn from_reader<R: Read>(reader: &mut R) -> Result<Self, PacketError> {
//         Ok(Self::Test)
//     }

//     async fn from_reader_async<R: AsyncReadExt + Unpin>(
//         reader: &mut R,
//     ) -> Result<Self, PacketError> {
//         todo!()
//     }
// }

pub trait ReadPacket: Sized {
    #[cfg(feature = "sync")]
    fn from_reader<R: Read>(reader: &mut R) -> Result<Self, PacketError>;

    // #[cfg(feature = "async")]
    // async fn from_reader_async<R: AsyncReadExt + Unpin>(
    //     reader: &mut R,
    // ) -> Result<Self, PacketError>;

    fn from_raw<R: Read>(reader: &mut R) -> Result<Self, PacketError>;
}

trait ToPacket {
    fn to_packet(&self) -> Packet;
}

#[derive(Debug)]
pub struct RawPacket {
    pub data: Vec<u8>,
}

impl RawPacket {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn uncompress(&self, threshold: u32) -> Self {
        todo!()
    }

    #[cfg(feature = "sync")]
    pub fn read_sync<R: Read, P: ReadPacket>(reader: &mut R) -> Result<P, PacketError> {
        let varint = VarInt::read_sync(reader)?;
        if varint.val() < 0 {
            return Err(PacketError::NegativeLength(varint.val() as i64));
        }
        let mut limited_reader = reader.take(varint.val() as u64);
        Ok(P::from_reader(&mut limited_reader)?)
    }

    #[cfg(feature = "sync")]
    pub fn write_sync<W: Write>(&self, writer: &mut W) -> Result<(), PacketError> {
        VarInt::from(self.data.len() as i32).write_sync(writer)?;
        writer.write_all(&self.data)?;
        Ok(())
    }

    #[cfg(feature = "async")]
    pub async fn read_async<R: AsyncReadExt + Unpin>(
        reader: &mut R,
    ) -> Result<RawPacket, PacketError> {
        let varint = VarInt::read_async(reader).await?;
        if varint.val() < 0 {
            return Err(PacketError::NegativeLength(varint.val() as i64));
        }
        let length = varint.val() as u64;
        let mut buf = Vec::with_capacity(length as usize);

        reader
            .take(length)
            .read_to_end(&mut buf)
            .await
            .map_err(PacketError::Io)?;
        Ok(RawPacket { data: buf })
    }

    #[cfg(feature = "async")]
    pub async fn write_async<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
    ) -> Result<(), PacketError> {
        VarInt::from(self.data.len() as i32)
            .write_async(writer)
            .await?;
        writer.write_all(&self.data).await?;
        Ok(())
    }
}

// Create and send packets to the client/server
pub struct Packet {
    packet_id: i32,
    data: Vec<u8>,
}

pub fn compress_zlib(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

impl Packet {
    pub fn new(packet_id: i32, data: Vec<u8>) -> Self {
        Self { packet_id, data }
    }

    pub fn len(&self) -> usize {
        VarInt::new(self.packet_id).len() as usize + self.data.len()
    }

    pub fn to_raw(self) -> RawPacket {
        let packet_id_varint = VarInt::new(self.packet_id);

        let payload_length = self.data.len();
        let total_length = packet_id_varint.len() as usize + payload_length;

        let mut buf = Vec::with_capacity(total_length);

        // data_length
        //     .encode(&mut buf)
        //     .expect("frame length encode failed");
        packet_id_varint
            .encode(&mut buf)
            .expect("packet_id encode failed");
        buf.extend_from_slice(&self.data);

        RawPacket { data: buf }
    }

    pub fn compress(self, threshold: u32) -> io::Result<RawPacket> {
        let packet_id_varint = VarInt::new(self.packet_id);
        let mut uncompressed =
            Vec::with_capacity(packet_id_varint.len() as usize + self.data.len());
        packet_id_varint.encode(&mut uncompressed)?;
        uncompressed.extend_from_slice(&self.data);

        let compressed = compress_zlib(&uncompressed)?;

        let data_length = VarInt::new(uncompressed.len() as i32);

        let mut buf = Vec::with_capacity(data_length.len() as usize + compressed.len());
        data_length.encode(&mut buf)?;
        buf.extend_from_slice(&compressed);

        Ok(RawPacket { data: buf })
    }

    pub async fn into_raw(self) -> Result<RawPacket, PacketError> {
        let mut buf = Vec::with_capacity(5 + self.data.len());
        VarInt::new(self.packet_id).encode(&mut buf)?;
        buf.extend_from_slice(&self.data);
        Ok(RawPacket { data: buf })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Encoder le packet_id en VarInt
        let packet_id_bytes = VarInt::from(self.packet_id).to_bytes();

        // La longueur = taille(packet_id encodé) + taille(data)
        let length = VarInt::from((packet_id_bytes.len() + self.data.len()) as i32);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&length.to_bytes());
        bytes.extend_from_slice(&packet_id_bytes);
        bytes.extend_from_slice(&self.data);
        bytes
    }
}
