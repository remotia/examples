use std::collections::HashMap;

use bitcode::{Decode, Encode};
use remotia::{
    buffers::BufMut,
    traits::{FrameError, FrameProperties},
};
use remotia_srt::SRTTransmission;

use crate::types::{BufferType, Error, FrameData, Stat};

#[derive(Encode, Decode)]
pub struct SerializedFrameData {
    frame_id: u128,
    mapped_buffers: HashMap<BufferType, Vec<u8>>,
}

impl SRTTransmission for FrameData {
    fn report_receive_error(&mut self, _: std::io::Error) {
        self.report_error(Error::NetworkError);
    }

    fn report_reception_delay(&mut self, value: u128) {
        self.set(Stat::ReceptionDelay, value);
    }

    fn deserialize_packet(&mut self, data: &remotia::buffers::Bytes) {
        let serialized = bitcode::decode::<SerializedFrameData>(data).unwrap();

        self.set(Stat::CaptureTime, serialized.frame_id);
        for (key, vector) in serialized.mapped_buffers.iter() {
            log::debug!("Deserializing buffer {:?} ({} bytes)...", key, vector.len());
            self.buffers.get_mut(key).unwrap().put(vector.as_slice());
        }
    }

    fn serialize_packet(&self) -> remotia::buffers::Bytes {
        let mapped_buffers: HashMap<BufferType, Vec<u8>> = self
            .buffers
            .iter()
            .map(|(key, value)| {
                log::debug!("Serializing buffer {:?} ({} bytes)...", key, value.len());
                (key.clone(), value.to_vec())
            })
            .collect();

        let serialized = SerializedFrameData {
            frame_id: self.get(&Stat::CaptureTime).unwrap(),
            mapped_buffers,
        };

        bitcode::encode(&serialized).into()
    }
}
