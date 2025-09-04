use std::collections::HashMap;

use bitcode::{Decode, Encode};
use remotia::{
    buffers::{BufMut, BytesMut},
    traits::{
        BorrowFrameProperties, BorrowMutFrameProperties, FrameError, FrameProperties,
        PullableFrameProperties,
    },
};
use remotia_ffmpeg_codecs::FFMpegCodec;

mod transmission;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub enum BufferType {
    CapturedRGBAFrameBuffer,
    EncodedPacketBuffer,

    DecodedRGBAFrameBuffer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stat {
    CaptureTime,
    EncodePushTime,
    TransmissionStartTime,
    DecodePushTime,

    EncodeTime,
    TransmissionTime,
    DecodeTime,

    FrameDelay,
    ReceptionDelay,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    NoFrame,
    CodecError,
    NetworkError,
}

#[derive(Default, Debug)]
pub struct FrameData {
    statistics: HashMap<Stat, u128>,
    buffers: HashMap<BufferType, BytesMut>,
    error: Option<Error>,
}

/*
impl Encode for FrameData {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        Encode::encode(&self.statistics, encoder)?;
        Encode::encode(&self.error, encoder)?;
        let mapped_buffers: HashMap<&BufferType, Vec<u8>> = self
            .buffers
            .iter()
            .map(|(key, value)| (key, value.to_vec()))
            .collect();

        Encode::encode(&mapped_buffers, encoder)?;

        Ok(())
    }
}

impl Decode for FrameData {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let statistics: HashMap<Stat, u128> = Decode::decode(decoder)?;
        let error: Option<Error> = Decode::decode(decoder)?;
        let mapped_buffers: HashMap<BufferType, Vec<u8>> = Decode::decode(decoder)?;
        let buffers: HashMap<BufferType, BytesMut> = mapped_buffers
            .iter()
            .map(|(key, vector)| (*key, BytesMut::from(&vector[..])))
            .collect();

        Ok(Self {
            statistics,
            buffers,
            error,
        })
    }
}
*/

impl FrameProperties<Stat, u128> for FrameData {
    fn set(&mut self, key: Stat, value: u128) {
        self.statistics.insert(key, value);
    }

    fn get(&self, key: &Stat) -> Option<u128> {
        self.statistics.get(key).copied()
    }
}

impl PullableFrameProperties<BufferType, BytesMut> for FrameData {
    fn push(&mut self, key: BufferType, value: BytesMut) {
        self.buffers.insert(key, value);
    }

    fn pull(&mut self, key: &BufferType) -> Option<BytesMut> {
        self.buffers.remove(key)
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
        self.buffers
            .get_mut(&BufferType::EncodedPacketBuffer)
            .unwrap()
            .put(packet_data);
    }

    fn get_packet_data_buffer(&self) -> &[u8] {
        self.buffers.get(&BufferType::EncodedPacketBuffer).unwrap()
    }

    fn write_decoded_buffer(&mut self, data: &[u8]) {
        self.buffers
            .get_mut(&BufferType::DecodedRGBAFrameBuffer)
            .unwrap()
            .put(data);
    }

    fn report_flush_error(&mut self) {
        self.report_error(Error::CodecError);
    }

    fn report_codec_error(&mut self) {
        self.report_error(Error::CodecError);
    }

    fn report_decoder_drain_error(&mut self) {
        self.report_error(Error::CodecError);
    }

    fn set_frame_id(&mut self, frame_id: i64) {
        self.set(Stat::CaptureTime, frame_id as u128);
    }

    fn get_frame_id(&self) -> i64 {
        self.get(&Stat::CaptureTime).unwrap() as i64
    }
}
