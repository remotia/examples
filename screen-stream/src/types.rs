use std::collections::HashMap;

use remotia::{traits::{PullableFrameProperties, BorrowFrameProperties, BorrowMutFrameProperties, FrameError, FrameProperties}, buffers::BytesMut};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferType {
    CapturedRGBAFrameBuffer,
    EncodedFrameBuffer,
    DecodedRGBAFrameBuffer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stat {
    CaptureTime,
    EncodePushTime,
    TransmissionStartTime,

    EncodeTime,
    TransmissionTime
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    NoFrame,
    CodecError,
}

#[derive(Default, Debug)]
pub struct FrameData {
    statistics: HashMap<Stat, u128>,
    buffers: HashMap<BufferType, BytesMut>,
    error: Option<Error>
}

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

