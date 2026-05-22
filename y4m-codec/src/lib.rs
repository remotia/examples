pub mod processors;

use std::collections::HashMap;

use remotia::buffers::{BuffersMap, BytesMut, buffers_map};
use remotia::traits::{BorrowFrameProperties, BorrowMutFrameProperties, FrameError, FrameProperties};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferType {
    RgbaFrame,
    EncodedPacket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stat {
    FrameId,
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    CodecError,
    FlushError,
    DrainError,
}

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
