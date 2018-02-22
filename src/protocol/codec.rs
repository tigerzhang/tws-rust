/*
 * BytesCodec from tokio_io 0.1.4 has a buffer-overflowing bug.
 * Temporary fix: just pull in the code from 0.1.5
 * FIXME: Remove this file after upgrading to the reformed new tokio
 */
use bytes::{Bytes, BufMut, BytesMut};
use tokio_io::codec::{Encoder, Decoder};
use std::io;

/// A simple `Codec` implementation that just ships bytes around.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BytesCodec(());

impl BytesCodec {
    /// Creates a new `BytesCodec` for shipping around raw bytes.
    pub fn new() -> BytesCodec { BytesCodec(())  }
}

impl Decoder for BytesCodec {
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<BytesMut>, io::Error> {
        if buf.len() > 0 {
            let len = buf.len();
            Ok(Some(buf.split_to(len)))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for BytesCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn encode(&mut self, data: Bytes, buf: &mut BytesMut) -> Result<(), io::Error> {
        buf.reserve(data.len());
        buf.put(data);
        Ok(())
    }
}