use std::io;

use bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;

use crate::types::{serialize::read_varint, varint::VarInt};

pub struct MinecraftCodex;

pub struct RawPacket {
    pub id: u8,
    pub data: BytesMut,
}

impl Decoder for MinecraftCodex {
    type Item = RawPacket;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let (packet_len, varint_len) = match read_varint(src) {
            Some(v) => v,
            None => return Ok(None),
        };

        if src.len() < varint_len + packet_len as usize {
            src.reserve(packet_len as usize);
            return Ok(None);
        }

        src.advance(varint_len);

        let mut packet_data = src.split_to(packet_len as usize);
        let packet_id = packet_data.get_u8();

        Ok(Some(RawPacket {
            id: packet_id,
            data: packet_data,
        }))
    }
}
