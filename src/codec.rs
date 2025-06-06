use bytes::Bytes;
use tonic::{
    Status,
    codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder},
};

#[derive(Clone)]
pub(crate) struct BytesCodec;

impl Codec for BytesCodec {
    type Encode = Bytes;
    type Decode = Bytes;
    type Encoder = Self;
    type Decoder = Self;

    fn encoder(&mut self) -> Self::Encoder {
        Self
    }

    fn decoder(&mut self) -> Self::Decoder {
        Self
    }
}

impl Encoder for BytesCodec {
    type Item = Bytes;
    type Error = Status;

    fn encode(
        &mut self,
        item: Self::Item,
        dst: &mut EncodeBuf<'_>,
    ) -> std::result::Result<(), Self::Error> {
        use bytes::BufMut;
        dst.put(item);
        Ok(())
    }
}

impl Decoder for BytesCodec {
    type Item = Bytes;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        use bytes::{Buf, Bytes};
        if src.remaining() == 0 {
            return Ok(None);
        }
        let chunk = src.chunk();
        let bytes = Bytes::copy_from_slice(chunk);
        src.advance(chunk.len());
        Ok(Some(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_codec_creation() {
        let mut codec = BytesCodec;

        // Test that we can create encoder and decoder without panicking
        let _encoder = codec.encoder();
        let _decoder = codec.decoder();

        // Test that the codec implements Clone
        let _cloned_codec = codec.clone();
    }
}
