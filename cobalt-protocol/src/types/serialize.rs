use bytes::BufMut;
use bytes::BytesMut;
use std::io::{self, Read, Result as IoResult};
use tokio::io::AsyncReadExt;

use crate::types::varint::VarInt;

pub fn read_byte<R: Read>(r: &mut R) -> IoResult<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    let u = buf[0];
    Ok(u)
}

pub async fn read_byte_async<R: AsyncReadExt + Unpin>(r: &mut R) -> IoResult<u8> {
    let u = r.read_u8().await;
    u
}

pub async fn read_sbyte<R: AsyncReadExt + Unpin>(r: &mut R) -> IoResult<i8> {
    r.read_i8().await
}

pub fn read_short<R: Read>(r: &mut R) -> IoResult<i16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    let u = i16::from_be_bytes(buf);
    Ok(u)
}

pub async fn read_short_async<R: AsyncReadExt + Unpin>(r: &mut R) -> IoResult<i16> {
    let u = r.read_i16().await;
    u
}

pub fn read_double<R: Read>(r: &mut R) -> IoResult<f64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    let u = f64::from_be_bytes(buf);
    Ok(u)
}
pub fn read_float<R: Read>(r: &mut R) -> IoResult<f32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    let u = f32::from_be_bytes(buf);
    Ok(u)
}

pub fn read_long<R: Read>(r: &mut R) -> IoResult<i64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    let u = i64::from_be_bytes(buf);
    Ok(u)
}

pub async fn read_long_async<R: AsyncReadExt + Unpin>(r: &mut R) -> IoResult<i64> {
    let u = r.read_i64().await;
    u
}

pub fn read_string<R: Read>(r: &mut R) -> IoResult<String> {
    let len = VarInt::read_sync(r).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let len = len.val() as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;

    let s = String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
    s
}

pub async fn read_string_async<R: AsyncReadExt + Unpin>(r: &mut R) -> IoResult<String> {
    let len = VarInt::read_async(r)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let len = len.val() as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;

    let s = String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
    s
}

pub fn read_boolean<R: Read>(r: &mut R) -> IoResult<bool> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    let b = buf[0] != 0;
    Ok(b)
}

pub fn read_int<R: Read>(r: &mut R) -> IoResult<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    let u = i32::from_be_bytes(buf);
    Ok(u)
}

pub fn read_bytes<R: Read>(r: &mut R, len: usize) -> IoResult<Vec<u8>> {
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn read_varint(src: &BytesMut) -> Option<(i32, usize)> {
    let mut result = 0i32;
    let mut shift = 0u32;

    for (i, &byte) in src.iter().enumerate() {
        result |= ((byte & 0x7F) as i32) << shift;
        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift += 7;
        if shift >= 32 {
            return None;
        }
    }
    None
}

pub fn write_varint<B: BufMut>(dst: &mut B, mut value: i32) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        dst.put_u8(byte);
        if value == 0 {
            break;
        }
    }
}
