use crate::{
    Encode,
    types::serialize::{read_varint, write_varint},
};
use std::io::{self, Read, Write};

use bytes::{Buf, BufMut, BytesMut};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
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
#[derive(Debug)]
pub struct Packet {
    pub id: u8,
    pub data: BytesMut,
}

pub fn compress_zlib(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}
pub fn compress_zlib_bytes(data: &[u8]) -> io::Result<BytesMut> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    Ok(BytesMut::from(compressed.as_slice()))
}

pub fn compress_payload(payload: BytesMut, threshold: u32) -> io::Result<BytesMut> {
    if payload.len() < threshold as usize {
        let mut inner = BytesMut::with_capacity(1 + payload.len());
        write_varint(&mut inner, 0);
        inner.put(payload);
        Ok(inner)
    } else {
        let data_length = payload.len() as i32;
        let compressed = compress_zlib_bytes(&payload)?;

        let mut inner = BytesMut::with_capacity(5 + compressed.len());
        write_varint(&mut inner, data_length);
        inner.put(compressed);
        Ok(inner)
    }
}

pub fn decompress_packet(mut src: BytesMut) -> io::Result<BytesMut> {
    let (data_length, varint_len) = read_varint(&src)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid data length varint"))?;
    src.advance(varint_len);

    if data_length == 0 {
        return Ok(src);
    }

    let mut decoder = ZlibDecoder::new(&src[..]);
    let mut decompressed = Vec::with_capacity(data_length as usize);
    decoder.read_to_end(&mut decompressed)?;

    Ok(BytesMut::from(decompressed.as_slice()))
}

// pub fn compress_packet(data: BytesMut, threshold: u32) -> io::Result<BytesMut> {
//     let mut uncompressed = BytesMut::with_capacity(5 + data.len());
//     write_varint(&mut uncompressed, self.id as i32);
//     uncompressed.put(data);

//     if uncompressed.len() < threshold as usize {
//         let mut buf = BytesMut::with_capacity(1 + uncompressed.len());
//         write_varint(&mut buf, 0); // data_length = 0
//         buf.put(uncompressed);
//         return Ok(buf);
//     }

//     // 2. Compresser
//     let data_length = uncompressed.len() as i32;
//     let compressed = compress_zlib_bytes(&uncompressed)?;

//     // 3. Construire : VarInt(data_length) + compressed
//     let mut buf = BytesMut::with_capacity(5 + compressed.len());
//     write_varint(&mut buf, data_length);
//     buf.put(compressed);

//     Ok(buf)
// }

impl Packet {
    pub fn new(id: u8, data: Vec<u8>) -> Self {
        Self {
            id,
            data: BytesMut::from(data.as_slice()),
        }
    }

    pub fn new_with_bytes(id: u8, data: BytesMut) -> Self {
        Self { id, data }
    }

    pub fn len(&self) -> usize {
        VarInt::new(self.id as i32).len() as usize + self.data.len()
    }

    pub fn to_raw(self) -> RawPacket {
        let packet_id_varint = VarInt::new(self.id as i32);

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

    pub fn compress(self, _threshold: u32) -> io::Result<RawPacket> {
        let packet_id_varint = VarInt::new(self.id as i32);
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
        VarInt::new(self.id as i32).encode(&mut buf)?;
        buf.extend_from_slice(&self.data);
        Ok(RawPacket { data: buf })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Encoder le packet_id en VarInt
        let packet_id_bytes = VarInt::from(self.id as i32).to_bytes();

        // La longueur = taille(packet_id encodé) + taille(data)
        let length = VarInt::from((packet_id_bytes.len() + self.data.len()) as i32);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&length.to_bytes());
        bytes.extend_from_slice(&packet_id_bytes);
        bytes.extend_from_slice(&self.data);
        bytes
    }
}
