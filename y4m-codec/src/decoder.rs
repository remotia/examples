use std::io::Read;
use std::sync::Arc;

use async_trait::async_trait;
use clap::Parser;
use cstr::cstr;
use remotia::buffers::BytesMut;
use remotia::pipeline::{component::Component, Pipeline};
use remotia::traits::{BorrowFrameProperties, FrameProperties, FrameProcessor, PullableFrameProperties};
use remotia_ffmpeg_codecs::ffi;
use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVCodecParserContext};
use rsmpeg::error::RsmpegError;
use rsmpeg::swscale::SwsContext;
use tokio::sync::Mutex;

use y4m_codec::processors::png_frame_writer::PNGWriter;
use y4m_codec::{BufferType, Error, FrameData, Stat};

const READ_CHUNK_SIZE: usize = 65536;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    output_dir: String,

    #[arg(short = 'W', long)]
    width: usize,

    #[arg(short = 'H', long)]
    height: usize,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = Args::parse();
    log::info!(
        "Decoding {} -> {} ({}x{})",
        args.input,
        args.output_dir,
        args.width,
        args.height
    );

    let decoder = AVCodec::find_decoder_by_name(cstr!("h264")).expect("h264 decoder not found");
    let mut decode_context = AVCodecContext::new(&decoder);
    decode_context.open(None).expect("Unable to open decoder");
    let decode_context = Arc::new(Mutex::new(decode_context));

    let parser_context = AVCodecParserContext::init(decoder.id).expect("h264 parser not found");

    let scaler = SwsContext::get_context(
        args.width as i32,
        args.height as i32,
        ffi::AV_PIX_FMT_YUV420P,
        args.width as i32,
        args.height as i32,
        ffi::AV_PIX_FMT_RGBA,
        ffi::SWS_BILINEAR,
        None,
        None,
        None,
    )
    .expect("Failed to create SwsContext");

    let png_writer = Arc::new(std::sync::Mutex::new(PNGWriter::new(
        args.output_dir.into(),
        args.width as u32,
        args.height as u32,
    )));

    let processor = DecoderProcessor::new(
        decode_context,
        parser_context,
        scaler,
        png_writer.clone(),
        args.width as i32,
        args.height as i32,
    );

    let mut pipeline = Pipeline::<FrameData>::new()
        .tag("decoder")
        .feedable()
        .link(
            Component::new()
                .append(processor)
                .tag("decoder"),
        );

    let feeder = pipeline.get_feeder();
    let handles = pipeline.run();

    let mut file = std::fs::File::open(&args.input).expect("Unable to open H264 input file");
    let mut chunk = vec![0u8; READ_CHUNK_SIZE];

    loop {
        let n = match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                log::error!("Read error: {:?}", e);
                break;
            }
        };

        let mut fd = FrameData::default();
        let mut buf = BytesMut::with_capacity(n);
        buf.extend_from_slice(&chunk[..n]);
        fd.push(BufferType::EncodedPacket, buf);

        feeder.feed(fd);
    }

    let mut eof_fd = FrameData::default();
    eof_fd.set(Stat::Eof, 1);
    feeder.feed(eof_fd);

    drop(feeder);

    for handle in handles {
        handle.await.unwrap();
    }

    let frame_count = png_writer.lock().unwrap().frame_count();
    log::info!("Decoding complete: {} frames written", frame_count);
}

struct DecoderProcessor {
    decode_context: Arc<Mutex<AVCodecContext>>,
    parser_context: AVCodecParserContext,
    scaler: SwsContext,
    png_writer: Arc<std::sync::Mutex<PNGWriter>>,
    output_width: i32,
    output_height: i32,
}

impl DecoderProcessor {
    fn new(
        decode_context: Arc<Mutex<AVCodecContext>>,
        parser_context: AVCodecParserContext,
        scaler: SwsContext,
        png_writer: Arc<std::sync::Mutex<PNGWriter>>,
        output_width: i32,
        output_height: i32,
    ) -> Self {
        Self {
            decode_context,
            parser_context,
            scaler,
            png_writer,
            output_width,
            output_height,
        }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for DecoderProcessor {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let is_eof = frame_data.get(&Stat::Eof) == Some(1);
        let mut decode_context = self.decode_context.lock().await;

        if is_eof {
            log::info!("DecoderProcessor: EOF, flushing parser and decoder");

            let mut packet = rsmpeg::avcodec::AVPacket::new();
            loop {
                let (packet_ready, consumed) = match self
                    .parser_context
                    .parse_packet(&mut decode_context, &mut packet, &[])
                {
                    Ok(result) => result,
                    Err(e) => {
                        log::warn!("Parser flush error: {:?}", e);
                        break;
                    }
                };

                if consumed == 0 && !packet_ready {
                    break;
                }

                if packet_ready {
                    let mut sent = false;
                    while !sent {
                        match decode_context.send_packet(Some(&packet)) {
                            Ok(()) => {
                                sent = true;
                            }
                            Err(RsmpegError::DecoderFullError) => {
                                drain_and_write_frames(
                                    &mut decode_context,
                                    &mut self.scaler,
                                    &mut self.png_writer,
                                    self.output_width,
                                    self.output_height,
                                );
                            }
                            Err(RsmpegError::SendPacketError(-11)) => {
                                drain_and_write_frames(
                                    &mut decode_context,
                                    &mut self.scaler,
                                    &mut self.png_writer,
                                    self.output_width,
                                    self.output_height,
                                );
                            }
                            Err(e) => {
                                log::warn!("DecoderProcessor: EOF flush send_packet error: {:?}", e);
                                sent = true;
                            }
                        }
                    }

                    packet = rsmpeg::avcodec::AVPacket::new();
                }
            }

            match decode_context.send_packet(None) {
                Ok(()) => {}
                Err(RsmpegError::DecoderFullError) | Err(RsmpegError::SendPacketError(-11)) => {
                    drain_and_write_frames(
                        &mut decode_context,
                        &mut self.scaler,
                        &mut self.png_writer,
                        self.output_width,
                        self.output_height,
                    );
                    if let Err(e) = decode_context.send_packet(None) {
                        log::warn!("DecoderProcessor: EOF flush send_packet(None) error after drain: {:?}", e);
                    }
                }
                Err(e) => {
                    log::warn!("DecoderProcessor: EOF flush send_packet(None) error: {:?}", e);
                }
            }

            drain_and_write_frames(
                &mut decode_context,
                &mut self.scaler,
                &mut self.png_writer,
                self.output_width,
                self.output_height,
            );
            return Some(frame_data);
        }

        let packet_data = frame_data
            .get_ref(&BufferType::EncodedPacket)
            .map(|b| b.as_ref())
            .unwrap_or(&[]);

        let mut packet = rsmpeg::avcodec::AVPacket::new();
        let mut offset = 0usize;

        while offset < packet_data.len() {
            let (packet_ready, consumed) = match self
                .parser_context
                .parse_packet(&mut decode_context, &mut packet, &packet_data[offset..])
            {
                Ok(result) => result,
                Err(e) => {
                    log::warn!("Parser error: {:?}", e);
                    break;
                }
            };

            offset += consumed;

            if packet_ready {
                let mut sent = false;
                while !sent {
                    match decode_context.send_packet(Some(&packet)) {
                        Ok(()) => {
                            sent = true;
                        }
                        Err(RsmpegError::DecoderFullError) => {
                            drain_and_write_frames(
                                &mut decode_context,
                                &mut self.scaler,
                                &mut self.png_writer,
                                self.output_width,
                                self.output_height,
                            );
                        }
                        Err(RsmpegError::DecoderFlushedError) => {
                            sent = true;
                        }
                        Err(RsmpegError::SendPacketError(-11)) => {
                            drain_and_write_frames(
                                &mut decode_context,
                                &mut self.scaler,
                                &mut self.png_writer,
                                self.output_width,
                                self.output_height,
                            );
                        }
                        Err(e) => {
                            log::warn!("DecoderProcessor: send_packet error: {:?}", e);
                            sent = true;
                        }
                    }
                }

                packet = rsmpeg::avcodec::AVPacket::new();
            }
        }

        drain_and_write_frames(
            &mut decode_context,
            &mut self.scaler,
            &mut self.png_writer,
            self.output_width,
            self.output_height,
        );

        Some(frame_data)
    }
}

fn drain_and_write_frames(
    decode_context: &mut AVCodecContext,
    scaler: &mut SwsContext,
    png_writer: &mut Arc<std::sync::Mutex<PNGWriter>>,
    output_width: i32,
    output_height: i32,
) {
    use rsmpeg::avutil::AVFrame;

    let mut scaled_frame = {
        let mut f = AVFrame::new();
        f.set_width(output_width);
        f.set_height(output_height);
        f.set_format(ffi::AV_PIX_FMT_RGBA);
        f.alloc_buffer().expect("Failed to alloc scaled frame buffer");
        f
    };

    loop {
        match decode_context.receive_frame() {
            Ok(codec_frame) => {
                scaler
                    .scale_frame(&codec_frame, 0, codec_frame.height, &mut scaled_frame)
                    .expect("Scale failed");

                let linesize = scaled_frame.linesize[0] as usize;
                let height = scaled_frame.height as usize;
                let rgba_data =
                    unsafe { std::slice::from_raw_parts(scaled_frame.data[0], height * linesize) };

                let mut writer = png_writer.lock().unwrap();
                writer.write_frame(rgba_data);
            }
            Err(RsmpegError::DecoderDrainError) => break,
            Err(RsmpegError::DecoderFlushedError) => break,
            Err(RsmpegError::ReceiveFrameError(-11)) => break,
            Err(e) => {
                log::warn!("DecoderProcessor: drain receive_frame error: {:?}", e);
                break;
            }
        }
    }
}
