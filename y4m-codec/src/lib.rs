//! Frame data types and buffer/stat keys for the Y4M codec pipeline.

pub mod processors;

use std::collections::HashMap;

use remotia::buffers::{BufMut, BuffersMap, BytesMut, buffers_map};
use remotia::traits::{BorrowFrameProperties, BorrowMutFrameProperties, FrameError, FrameProperties};
use remotia_ffmpeg_codecs::FFMpegCodec;

/// Buffer slot keys used by the encoding/decoding pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferType {
    /// RGBA pixel data produced by Y4M capture or decoder output.
    RgbaFrame,
    /// Encoded bitstream data (input to decoder, output from encoder).
    EncodedPacket,
    /// Decoded RGBA frame produced by the decoder puller.
    DecodedRGBAFrame,
}

/// Stat keys carried alongside frame data through the pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stat {
    /// Monotonically increasing frame counter.
    FrameId,
    /// Set to `1` on the final frame to signal end-of-stream.
    Eof,
}

/// Codec-specific error kinds reported through frame data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    CodecError,
    FlushError,
    DrainError,
}

/// Frame data that flows through the Y4M codec pipeline.
///
/// Carries typed buffers ([`BufferType`]) and stat metadata ([`Stat`]) as well as
/// an optional error slot. The `#[buffers_map]` macro generates [`PullableFrameProperties`]
/// impl for the `buffers` field.
#[derive(Default, Debug)]
#[buffers_map(buffers)]
pub struct FrameData {
    buffers: BuffersMap<BufferType>,
    stats: HashMap<Stat, u128>,
    error: Option<Error>,
}

impl FrameProperties<Stat, u128> for FrameData {
    fn set(&mut self, key: Stat, value: u128) {
        self.stats.insert(key, value);
    }

    fn get(&self, key: &Stat) -> Option<u128> {
        self.stats.get(key).copied()
    }
}

impl BorrowFrameProperties<BufferType, BytesMut> for FrameData {
    fn get_ref(&self, key: &BufferType) -> Option<&BytesMut> {
        self.buffers.get(key)
    }
}

impl BorrowMutFrameProperties<BufferType, BytesMut> for FrameData {
    fn get_mut_ref(&mut self, key: &BufferType) -> Option<&mut BytesMut> {
        self.buffers.get_mut(key)
    }
}

impl FrameError<Error> for FrameData {
    fn report_error(&mut self, error: Error) {
        self.error = Some(error);
    }

    fn get_error(&self) -> Option<Error> {
        self.error
    }
}

impl FFMpegCodec for FrameData {
    fn write_packet_data(&mut self, packet_data: &[u8]) {
        if let Some(buf) = self.buffers.get_mut(&BufferType::EncodedPacket) {
            buf.put(packet_data);
        } else {
            let mut buf = BytesMut::with_capacity(packet_data.len());
            buf.put(packet_data);
            self.buffers.insert(BufferType::EncodedPacket, buf);
        }
    }

    fn get_packet_data_buffer(&self) -> &[u8] {
        self.buffers
            .get(&BufferType::EncodedPacket)
            .map(|b| b.as_ref())
            .unwrap_or(&[])
    }

    fn write_decoded_buffer(&mut self, data: &[u8]) {
        if let Some(buf) = self.buffers.get_mut(&BufferType::DecodedRGBAFrame) {
            buf.put(data);
        } else {
            let mut buf = BytesMut::with_capacity(data.len());
            buf.put(data);
            self.buffers.insert(BufferType::DecodedRGBAFrame, buf);
        }
    }

    fn report_flush_error(&mut self) {
        self.error = Some(Error::FlushError);
    }

    fn report_codec_error(&mut self) {
        self.error = Some(Error::CodecError);
    }

    fn report_decoder_drain_error(&mut self) {
        self.error = Some(Error::DrainError);
    }

    fn set_frame_id(&mut self, frame_id: i64) {
        self.stats.insert(Stat::FrameId, frame_id as u128);
    }

    fn get_frame_id(&self) -> i64 {
        self.stats
            .get(&Stat::FrameId)
            .copied()
            .unwrap_or(0) as i64
    }

    fn is_eof(&self) -> bool {
        self.stats.get(&Stat::Eof).copied() == Some(1)
    }
}
