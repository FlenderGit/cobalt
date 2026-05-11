use std::{
    io,
    io::Read,
    pin::Pin,
    task::{Context, Poll, ready},
};

use bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;
use tracing::info;

use crate::{crypto::Cfb8Decryptor, types::serialize::read_varint};

pub struct MinecraftCodex {
    pub decryptor: Option<Cfb8Decryptor>,
    pub threshold: Option<u32>,
    pub decrypted_offset: usize,
}

#[derive(Debug)]
pub struct RawPacket {
    pub id: u8,
    pub data: BytesMut,
}

impl Default for MinecraftCodex {
    fn default() -> Self {
        Self {
            decryptor: None,
            threshold: None,
            decrypted_offset: 0,
        }
    }
}

impl Decoder for MinecraftCodex {
    type Item = RawPacket;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        // info!("{:?} - {:?}", self.decryptor.is_some(), self.threshold);

        if let Some(decryptor) = &mut self.decryptor {
            let new_bytes = &src[self.decrypted_offset..];
            let decrypted = decryptor.decrypt_bytes(new_bytes)?;
            src[self.decrypted_offset..].copy_from_slice(&decrypted);
            self.decrypted_offset = src.len();
        }

        let (packet_len, len_bytes) = match read_varint(src) {
            Some(v) => v,
            None => return Ok(None),
        };

        if src.len() < len_bytes + packet_len as usize {
            return Ok(None);
        }

        src.advance(len_bytes);
        let mut packet_data = src.split_to(packet_len as usize);

        if self.decryptor.is_some() {
            self.decrypted_offset -= len_bytes + packet_len as usize;
        }

        if self.threshold.is_some() {
            let (data_len, dl_bytes) = read_varint(&packet_data).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Invalid Data Length VarInt")
            })?;
            packet_data.advance(dl_bytes);

            if data_len > 0 {
                let mut decoder = flate2::read::ZlibDecoder::new(&packet_data[..]);
                let mut decompressed = Vec::with_capacity(data_len as usize);
                decoder.read_to_end(&mut decompressed).map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("Zlib: {}", e))
                })?;

                if decompressed.len() != data_len as usize {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Decompressed size mismatch: {} != {}",
                            decompressed.len(),
                            data_len
                        ),
                    ));
                }
                packet_data = BytesMut::from(decompressed.as_slice());
            }
        }

        let (packet_id, id_bytes) = read_varint(&packet_data)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing Packet ID"))?;
        packet_data.advance(id_bytes);

        Ok(Some(RawPacket {
            id: packet_id as u8,
            data: packet_data,
        }))
    }
}
use tokio::io::{AsyncRead, ReadBuf};

pub struct DecryptingReader<R> {
    inner: R,
    pub decryptor: Option<Cfb8Decryptor>,
}

impl<R> DecryptingReader<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            decryptor: None,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for DecryptingReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        ready!(Pin::new(&mut self.inner).poll_read(cx, buf))?;
        let after = buf.filled().len();

        if let Some(ref mut dec) = self.decryptor {
            let filled = &mut buf.filled_mut()[before..after];
            let tmp = filled.to_vec();
            let mut out = vec![0u8; tmp.len() + 16];
            let n = dec.update(&tmp, &mut out).unwrap();
            filled.copy_from_slice(&out[..n]);

            info!("Decrypted {:?}", filled);
        }

        Poll::Ready(Ok(()))
    }
}
