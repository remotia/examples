use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use remotia::traits::{BorrowFrameProperties, FrameProcessor};

use crate::{BufferType, FrameData};

pub struct H264PacketWriter {
    file: Arc<Mutex<File>>,
}

impl H264PacketWriter {
    pub fn new(file: Arc<Mutex<File>>) -> Self {
        Self { file }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for H264PacketWriter {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        if let Some(packet_buf) = frame_data.get_ref(&BufferType::EncodedPacket) {
            if !packet_buf.is_empty() {
                let mut file = self.file.lock().unwrap();
                file.write_all(packet_buf).expect("Unable to write H264 packet data");
            }
        }

        Some(frame_data)
    }
}
