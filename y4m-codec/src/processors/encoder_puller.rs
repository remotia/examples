use std::sync::Arc;

use async_trait::async_trait;
use remotia::traits::{BorrowMutFrameProperties, FrameError, FrameProcessor, PullableFrameProperties};
use remotia::buffers::BytesMut;
use rsmpeg::avcodec::AVCodecContext;
use rsmpeg::error::RsmpegError;
use tokio::sync::Mutex;

use crate::{BufferType, Error, FrameData};

pub struct EncoderPuller {
    encode_context: Arc<Mutex<AVCodecContext>>,
}

impl EncoderPuller {
    pub fn new(encode_context: Arc<Mutex<AVCodecContext>>) -> Self {
        Self { encode_context }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for EncoderPuller {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        if frame_data.get_mut_ref(&BufferType::EncodedPacket).is_none() {
            frame_data.push(BufferType::EncodedPacket, BytesMut::new());
        }

        let packet_buf = frame_data.get_mut_ref(&BufferType::EncodedPacket).unwrap();

        let mut encode_context = self.encode_context.lock().await;

        loop {
            match encode_context.receive_packet() {
                Ok(packet) => {
                    let data =
                        unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };
                    packet_buf.extend_from_slice(data);
                }
                Err(RsmpegError::EncoderDrainError) => break,
                Err(RsmpegError::EncoderFlushedError) => {
                    frame_data.report_error(Error::FlushError);
                    break;
                }
                Err(e) => panic!("Encoder receive_packet error: {:?}", e),
            }
        }

        Some(frame_data)
    }
}
