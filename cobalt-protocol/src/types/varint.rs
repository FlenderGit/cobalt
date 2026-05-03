use std::io::{self, Read, Write};

#[cfg(feature = "async")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct VarInt(i32, u32);

impl VarInt {
    #[inline]
    pub fn val(self) -> i32 {
        self.0
    }

    #[inline]
    pub fn len(self) -> u32 {
        self.1
    }

    pub fn new(value: i32) -> Self {
        Self(value, Self::encoded_len(value))
    }

    pub fn encoded_len(value: i32) -> u32 {
        let mut value = value as u32;
        let mut count = 0;
        loop {
            count += 1;
            if value & !(SEGMENT_BITS as u32) == 0 {
                break;
            }
            value >>= 7;
        }
        count
    }

    pub fn read_sync<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut result: i32 = 0;
        let mut shift: u32 = 0;

        loop {
            let mut buf = [0u8; 1];
            if reader.read(&mut buf)? == 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Unexpected EOF").into());
            }
            let read = buf[0];

            let value = (read & SEGMENT_BITS) as i32;
            result |= value << shift;

            if (read & CONTINUE_BIT) == 0 {
                break;
            }

            shift += 7;
            if shift >= 32 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Overflow"));
            }
        }

        if result == 0 {
            shift = 1;
        }

        // TODO : real shift
        Ok(VarInt(result, shift))
    }

    pub fn write_sync<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut buf = [0u8; 5]; // VarInt max = 5 bytes
        let mut value = self.0 as u32;
        let mut i = 0;

        loop {
            if value & !(SEGMENT_BITS as u32) == 0 {
                buf[i] = value as u8;
                i += 1;
                break;
            }
            buf[i] = ((value as u8) & SEGMENT_BITS) | CONTINUE_BIT;
            i += 1;
            value >>= 7;
        }

        writer.write_all(&buf[..i])?;
        Ok(())
    }

    #[cfg(feature = "async")]
    pub async fn read_async<R: AsyncReadExt + Unpin>(reader: &mut R) -> io::Result<Self> {
        let mut result = 0;
        let mut shift = 0;

        loop {
            let mut buf = [0u8; 1];
            reader.read_exact(&mut buf).await?;
            let byte = buf[0];

            result |= ((byte & SEGMENT_BITS) as i32) << shift;
            shift += 7;

            if byte & CONTINUE_BIT == 0 {
                break;
            }

            if shift >= 32 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Overflow"));
            }
        }

        Ok(Self(result, shift / 7))
    }

    #[cfg(feature = "async")]
    pub async fn write_async<W: AsyncWriteExt + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        let mut buf = [0u8; 5]; // VarInt max = 5 bytes
        let mut value = self.0 as u32;
        let mut i = 0;

        loop {
            if value & !(SEGMENT_BITS as u32) == 0 {
                buf[i] = value as u8;
                i += 1;
                break;
            }
            buf[i] = ((value as u8) & SEGMENT_BITS) | CONTINUE_BIT;
            i += 1;
            value >>= 7;
        }

        writer.write_all(&buf[..i]).await?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut value = self.0 as u32;
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            buf.push(byte);
            if value == 0 {
                break;
            }
        }
        buf
    }
}

impl From<i32> for VarInt {
    fn from(value: i32) -> Self {
        Self(value, Self::encoded_len(value))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct VarLong(i64);
