use std::sync::Arc;

use async_trait::async_trait;
use remotia::traits::{BorrowFrameProperties, FrameProcessor, FrameProperties};
use rsmpeg::avcodec::{AVCodecContext, AVCodecParserContext, AVPacket};
use rsmpeg::avutil::AVFrame;
use rsmpeg::error::RsmpegError;
use rsmpeg::swscale::SwsContext;
use tokio::sync::Mutex;

use crate::processors::png_frame_writer::PNGWriter;
use crate::{BufferType, FrameData, Stat};

pub struct DecoderProcessor {
    decode_context: Arc<Mutex<AVCodecContext>>,
    parser_context: AVCodecParserContext,
    scaler: SwsContext,
    scaled_avframe: AVFrame,
    png_writer: Arc<std::sync::Mutex<PNGWriter>>,
}

impl DecoderProcessor {
    pub fn new(
        decode_context: Arc<Mutex<AVCodecContext>>,
        parser_context: AVCodecParserContext,
        scaler: SwsContext,
        scaled_avframe: AVFrame,
        png_writer: Arc<std::sync::Mutex<PNGWriter>>,
    ) -> Self {
        Self {
            decode_context,
            parser_context,
            scaler,
            scaled_avframe,
            png_writer,
        }
    }
}

fn drain_frames(
    decode_context: &mut AVCodecContext,
    scaler: &mut SwsContext,
    scaled_avframe: &mut AVFrame,
    png_writer: &Arc<std::sync::Mutex<PNGWriter>>,
) {
    loop {
        match decode_context.receive_frame() {
            Ok(codec_avframe) => {
                scaler
                    .scale_frame(
                        &codec_avframe,
                        0,
                        codec_avframe.height,
                        scaled_avframe,
                    )
                    .expect("Scaling failed");

                let linesize = scaled_avframe.linesize[0] as usize;
                let height = scaled_avframe.height as usize;
                let data = unsafe {
                    std::slice::from_raw_parts(scaled_avframe.data[0], height * linesize)
                };

                let mut png_writer = png_writer.lock().unwrap();
                png_writer.write_frame(data);
            }
            Err(RsmpegError::DecoderDrainError) => break,
            Err(RsmpegError::DecoderFlushedError) => break,
            Err(_) => break,
        }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for DecoderProcessor {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        let is_eof = frame_data.get(&Stat::Eof).unwrap_or(0) == 1;

        let mut decode_context = self.decode_context.lock().await;

        if is_eof {
            log::info!("DecoderProcessor: EOF, flushing decoder");
            decode_context.send_packet(None).ok();
            drain_frames(
                &mut decode_context,
                &mut self.scaler,
                &mut self.scaled_avframe,
                &self.png_writer,
            );
            return Some(frame_data);
        }

        let packet_data = frame_data.get_ref(&BufferType::EncodedPacket).unwrap();
        let mut packet = AVPacket::new();

        let mut offset = 0usize;
        while offset < packet_data.len() {
            match self
                .parser_context
                .parse_packet(&mut decode_context, &mut packet, &packet_data[offset..])
            {
                Ok((packet_ready, consumed)) => {
                    offset += consumed;
                    if packet_ready {
                        let mut sent = false;
                        while !sent {
                            match decode_context.send_packet(Some(&packet)) {
                                Ok(()) => {
                                    sent = true;
                                }
                                Err(RsmpegError::DecoderFullError) => {
                                    drain_frames(
                                        &mut decode_context,
                                        &mut self.scaler,
                                        &mut self.scaled_avframe,
                                        &self.png_writer,
                                    );
                                }
                                Err(e) => {
                                    log::warn!("DecoderProcessor: send_packet error: {:?}", e);
                                    sent = true;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("DecoderProcessor: parser error: {:?}", e);
                    break;
                }
            }
        }

        drain_frames(
            &mut decode_context,
            &mut self.scaler,
            &mut self.scaled_avframe,
            &self.png_writer,
        );

        Some(frame_data)
    }
}
