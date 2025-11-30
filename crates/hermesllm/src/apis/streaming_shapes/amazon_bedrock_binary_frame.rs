use aws_smithy_eventstream::frame::DecodedFrame;
use aws_smithy_eventstream::frame::MessageFrameDecoder;
use bytes::Buf;
use std::collections::HashSet;

/// AWS Event Stream frame decoder wrapper
pub struct BedrockBinaryFrameDecoder<B>
where
    B: Buf,
{
    decoder: MessageFrameDecoder,
    buffer: B,
    content_block_start_indices: HashSet<i32>,
}

impl BedrockBinaryFrameDecoder<bytes::BytesMut> {
    /// This is a convenience constructor that creates a BytesMut buffer internally
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let buffer = bytes::BytesMut::from(bytes);
        Self {
            decoder: MessageFrameDecoder::new(),
            buffer,
            content_block_start_indices: std::collections::HashSet::new(),
        }
    }
}

impl<B> BedrockBinaryFrameDecoder<B>
where
    B: Buf,
{
    pub fn new(buffer: B) -> Self {
        Self {
            decoder: MessageFrameDecoder::new(),
            buffer,
            content_block_start_indices: HashSet::new(),
        }
    }

    pub fn decode_frame(&mut self) -> Option<DecodedFrame> {
        match self.decoder.decode_frame(&mut self.buffer) {
            Ok(frame) => Some(frame),
            Err(_e) => None, // Fatal decode error
        }
    }

    pub fn buffer_mut(&mut self) -> &mut B {
        &mut self.buffer
    }

    /// Check if there are any bytes remaining in the buffer
    pub fn has_remaining(&self) -> bool {
        self.buffer.has_remaining()
    }

    /// Check if a content_block_start event has been sent for the given index
    pub fn has_content_block_start_been_sent(&self, index: i32) -> bool {
        self.content_block_start_indices.contains(&index)
    }

    /// Mark that a content_block_start event has been sent for the given index
    pub fn set_content_block_start_sent(&mut self, index: i32) {
        self.content_block_start_indices.insert(index);
    }
}
