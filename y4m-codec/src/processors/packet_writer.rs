use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use remotia::traits::FrameProcessor;
use remotia_ffmpeg_codecs::FFMpegCodec;

use crate::FrameData;

/// Writes encoded packet data from frame data into an output file.
///
/// Intended as the terminal processor in an encoding pipeline. On each invocation it
/// extracts the packet payload via [`FFMpegCodec::get_packet_data_buffer`] and appends
/// it to the shared file.
pub struct PacketWriter {
    file: Arc<Mutex<File>>,
}

impl PacketWriter {
    /// Creates a new writer that appends packet data to `file`.
    pub fn new(file: Arc<Mutex<File>>) -> Self {
        Self { file }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for PacketWriter {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        let packet_data = frame_data.get_packet_data_buffer();
        if !packet_data.is_empty() {
            let mut file = self.file.lock().unwrap();
            file.write_all(packet_data).expect("Unable to write packet data");
        }

        Some(frame_data)
    }
}
