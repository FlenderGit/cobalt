use std::io::{self, Read, Write};

use crate::{
    packet::{Packet, RawPacket},
    types::varint::VarInt,
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

pub trait PacketId: Encode {
    const ID: i32;

    fn to_packet(&self) -> io::Result<Packet> {
        let mut buf = Vec::with_capacity(32);
        self.encode(&mut buf)?;
        Ok(Packet::new(Self::ID, buf))
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

impl Encode for Vec<u8> {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Can be without prefix length, like channel
        let len = VarInt::new(self.len() as i32);
        println!("len packet: {:?} -- {:?}", len.to_bytes(), len.val());
        len.encode(writer)?;
        writer.write_all(self)
    }
}

impl Decode for Vec<u8> {
    fn decode<R: Read + Unpin>(reader: &mut R) -> io::Result<Self> {
        let var_int = VarInt::decode(reader)?;
        let len = var_int.val() as usize;
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;
        Ok(buf)
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
