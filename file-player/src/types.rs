use std::collections::HashMap;

use remotia::{
    buffers::{BuffersMap, BytesMut, buffers_map},
    traits::{BorrowMutFrameProperties, FrameProperties},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BufferType {
    RGBAFrameBuffer,
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

#[derive(Default, Debug)]
#[buffers_map(buffers)]
pub struct FrameData {
    buffers: BuffersMap<BufferType>,
    statistics: HashMap<Stat, u128>,
}

impl FrameProperties<Stat, u128> for FrameData {
    fn set(&mut self, key: Stat, value: u128) {
        self.statistics.insert(key, value);
    }

    fn get(&self, key: &Stat) -> Option<u128> {
        self.statistics.get(key).copied()
    }
}

impl BorrowMutFrameProperties<BufferType, BytesMut> for FrameData {
    fn get_mut_ref(&mut self, key: &BufferType) -> Option<&mut BytesMut> {
        self.buffers.get_mut(key)
    }
}
