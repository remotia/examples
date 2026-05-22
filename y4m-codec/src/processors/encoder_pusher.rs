use std::sync::Arc;

use async_trait::async_trait;
use remotia::buffers::BytesMut;
use remotia::traits::{BorrowFrameProperties, FrameError, FrameProcessor, FrameProperties, PullableFrameProperties};
use rsmpeg::avcodec::AVCodecContext;
use rsmpeg::avutil::AVFrame;
use rsmpeg::error::RsmpegError;
use rsmpeg::swscale::SwsContext;
use tokio::sync::Mutex;

use crate::{BufferType, Error, FrameData, Stat};

pub struct EncoderPusher {
    encode_context: Arc<Mutex<AVCodecContext>>,
    scaler: SwsContext,
    input_avframe: AVFrame,
    scaled_avframe: AVFrame,
}

impl EncoderPusher {
    pub fn new(
        encode_context: Arc<Mutex<AVCodecContext>>,
        scaler: SwsContext,
        input_avframe: AVFrame,
        scaled_avframe: AVFrame,
    ) -> Self {
        Self {
            encode_context,
            scaler,
            input_avframe,
            scaled_avframe,
        }
    }
}

fn drain_packets(encode_context: &mut AVCodecContext, packet_buf: &mut BytesMut) {
    loop {
        match encode_context.receive_packet() {
            Ok(packet) => {
                let data =
                    unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };
                packet_buf.extend_from_slice(data);
            }
            Err(RsmpegError::EncoderDrainError) => break,
            Err(RsmpegError::EncoderFlushedError) => break,
            Err(e) => {
                log::warn!("Encoder receive_packet error: {:?}", e);
                break;
            }
        }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for EncoderPusher {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let is_eof = frame_data.get(&Stat::Eof).unwrap_or(0) == 1;

        if is_eof {
            log::info!("EncoderPusher: EOF, flushing encoder");
            let mut packet_buf = BytesMut::new();
            let mut encode_context = self.encode_context.lock().await;
            encode_context.send_frame(None).ok();
            drain_packets(&mut encode_context, &mut packet_buf);
            drop(encode_context);
            frame_data.push(BufferType::EncodedPacket, packet_buf);
            return Some(frame_data);
        }

        let frame_id = frame_data.get(&Stat::FrameId).unwrap_or(0) as i64;

        {
            let rgba_buffer = frame_data.get_ref(&BufferType::RgbaFrame).unwrap();
            let linesize = self.input_avframe.linesize[0] as usize;
            let height = self.input_avframe.height as usize;
            let data =
                unsafe { std::slice::from_raw_parts_mut(self.input_avframe.data[0], height * linesize) };
            data.copy_from_slice(rgba_buffer);
        }

        self.scaler
            .scale_frame(
                &self.input_avframe,
                0,
                self.input_avframe.height,
                &mut self.scaled_avframe,
            )
            .expect("Scaling failed");

        self.scaled_avframe.set_pts(frame_id);

        let mut packet_buf = BytesMut::new();
        let mut encode_context = self.encode_context.lock().await;

        let mut sent = false;
        while !sent {
            match encode_context.send_frame(Some(&self.scaled_avframe)) {
                Ok(()) => {
                    sent = true;
                }
                Err(RsmpegError::SendFrameAgainError) => {
                    drain_packets(&mut encode_context, &mut packet_buf);
                }
                Err(e) => {
                    log::warn!("EncoderPusher: send_frame error: {:?}", e);
                    frame_data.report_error(Error::CodecError);
                    sent = true;
                }
            }
        }

        drain_packets(&mut encode_context, &mut packet_buf);
        drop(encode_context);

        frame_data.push(BufferType::EncodedPacket, packet_buf);

        Some(frame_data)
    }
}
