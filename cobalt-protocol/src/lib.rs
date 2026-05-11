use std::io::{self, Read, Write};

use bytes::{BufMut, BytesMut};
use uuid::Uuid;

use crate::{
    packet::RawPacket,
    types::{serialize::write_varint, varint::VarInt},
};

pub mod chunk;
pub mod codex;
pub mod crypto;
pub mod packet;
pub mod types;

pub trait Encode {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>;
    fn instanciate(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::with_capacity(32);
        self.encode(&mut buf)?;
        Ok(RawPacket { data: buf })
    }
}

pub trait Decode: Sized {
    fn decode<R: Read + Unpin>(reader: &mut R) -> io::Result<Self>;
}

pub trait DecodeWithId: Sized {
    fn decode_with_id<R: Read + Unpin>(id: u8, reader: &mut R) -> io::Result<Self>;
}

pub trait PacketId: Encode {
    const ID: u8;

    // fn to_bytes(&self) -> io::Result<BytesMut> {
    //     let mut payload = BytesMut::with_capacity(64);
    //     write_varint(&mut payload, Self::ID as i32);
    //     let mut writer = payload.writer();
    //     self.encode(&mut writer)?;
    //     let payload = writer.into_inner();

    //     let mut frame = BytesMut::with_capacity(5 + payload.len());
    //     write_varint(&mut frame, payload.len() as i32);
    //     frame.put(payload);

    //     Ok(frame)
    // }

    fn to_bytes(&self) -> io::Result<BytesMut> {
        let mut buf = BytesMut::with_capacity(64);
        write_varint(&mut buf, Self::ID as i32); // id seulement
        let mut writer = buf.writer();
        self.encode(&mut writer)?;
        Ok(writer.into_inner()) // retourne [ VarInt(id) | data ] sans length
    }
}

// impl<T: Decode> ReadPacket for T {
//     fn from_raw<R: Read + Unpin>(reader: &mut R) -> Result<Self, PacketError> {
//         // Convertit io::Error → PacketError automatiquement
//         T::decode(reader).map_err(|e| PacketError::Io(e))
//     }
// }

pub trait ToVec: Encode {
    fn instanciate(&self) -> io::Result<RawPacket> {
        let mut buf = Vec::with_capacity(32);
        self.encode(&mut buf)?;
        Ok(RawPacket { data: buf })
    }
}

macro_rules! impl_primitive {
    ($t:ty) => {
        impl Encode for $t {
            #[inline]
            fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                writer.write_all(&self.to_be_bytes())?;
                Ok(())
            }
        }

        impl Decode for $t {
            #[inline]
            fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
                let mut buf = [0; std::mem::size_of::<$t>()];
                reader.read_exact(&mut buf)?;
                Ok(Self::from_be_bytes(buf))
            }
        }
    };
}

impl_primitive!(u8);
impl_primitive!(i8);
impl_primitive!(i16);
impl_primitive!(i32);
impl_primitive!(i64);
impl_primitive!(f64);
impl_primitive!(f32);
impl_primitive!(u16);

pub const MAX_STRING_LENGTH: usize = 32767;
pub fn serialize_string_with_max<W: Write>(
    s: &str,
    writer: &mut W,
    max_utf16: usize,
) -> io::Result<()> {
    let utf16_len = s.encode_utf16().count();
    if utf16_len > max_utf16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "String too long",
        ));
    }
    VarInt::new(s.len() as i32).write_sync(writer)?;
    writer.write_all(s.as_bytes())?;
    Ok(())
}

impl Encode for String {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        serialize_string_with_max(self, writer, MAX_STRING_LENGTH)
    }
}

pub fn deserialize_string_with_max<R: Read + Unpin>(
    reader: &mut R,
    max_utf16: usize,
) -> io::Result<String> {
    let byte_len = VarInt::read_sync(reader)?.val() as usize;
    let mut buf = vec![0u8; byte_len];
    reader.read_exact(&mut buf)?;
    let s = String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let utf16_len = s.encode_utf16().count();
    if utf16_len > max_utf16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "String too long",
        ));
    }
    Ok(s)
}

impl Decode for String {
    fn decode<R: Read + Unpin>(reader: &mut R) -> io::Result<Self> {
        deserialize_string_with_max(reader, MAX_STRING_LENGTH)
    }
}

impl Encode for str {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        serialize_string_with_max(self, writer, MAX_STRING_LENGTH)
    }
}

impl Encode for bool {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer
            .write_all(&[if *self { 0x01 } else { 0x00 }])
            .map_err(Into::into)
    }
}

impl Decode for bool {
    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = [0; 1];
        reader.read_exact(&mut buf)?;
        Ok(buf[0] != 0)
    }
}

impl Encode for VarInt {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.write_sync(writer).map_err(Into::into)
    }
}

impl Decode for VarInt {
    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        Self::read_sync(reader)
    }
}

#[derive(Debug)]
pub struct RawBytesArray(pub Vec<u8>);

impl Encode for RawBytesArray {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.0)
    }
}

impl Decode for RawBytesArray {
    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(Self(buf))
    }
}

pub struct Chunk {}

impl Encode for Uuid {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.as_bytes())
    }
}

impl Decode for Uuid {
    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut bytes = [0u8; 16];
        reader.read_exact(&mut bytes)?;
        Ok(Uuid::from_bytes(bytes))
    }
}

impl<T: Encode> Encode for Option<T> {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            Some(v) => {
                bool::encode(&true, writer)?;
                v.encode(writer)?;
                Ok(())
            }
            None => {
                bool::encode(&false, writer)?;
                Ok(())
            }
        }
    }
}

impl<D: Decode> Decode for Option<D> {
    fn decode<R: Read + Unpin>(reader: &mut R) -> io::Result<Self> {
        let present = u8::decode(reader)?;
        if present != 0 {
            Ok(Some(D::decode(reader)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let len = VarInt::new(self.len() as i32);
        len.encode(writer)?;
        for item in self {
            item.encode(writer)?;
        }
        Ok(())
    }
}

impl<D: Decode> Decode for Vec<D> {
    fn decode<R: Read + Unpin>(reader: &mut R) -> io::Result<Self> {
        let len = VarInt::decode(reader)?;
        let mut vec = Vec::with_capacity(len.val() as usize);
        for _ in 0..len.val() {
            vec.push(D::decode(reader)?);
        }
        Ok(vec)
    }
}
