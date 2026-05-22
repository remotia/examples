use std::io::Read;
use std::sync::Arc;

use clap::Parser;
use cstr::cstr;
use remotia::buffers::BytesMut;
use remotia::pipeline::{component::Component, Pipeline};
use remotia::traits::{FrameProperties, PullableFrameProperties};
use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVCodecParserContext};
use rsmpeg::avutil::AVFrame;
use rsmpeg::ffi;
use rsmpeg::swscale::SwsContext;
use tokio::sync::Mutex;

use y4m_codec::processors::{
    decoder_processor::DecoderProcessor,
    png_frame_writer::PNGWriter,
};
use y4m_codec::{BufferType, FrameData, Stat};

const READ_CHUNK_SIZE: usize = 65536;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    output_dir: String,

    #[arg(short, long)]
    width: usize,

    #[arg(short, long)]
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

    let scaled_avframe = {
        let mut f = AVFrame::new();
        f.set_width(args.width as i32);
        f.set_height(args.height as i32);
        f.set_format(ffi::AV_PIX_FMT_RGBA);
        f.alloc_buffer().expect("Failed to alloc scaled AVFrame buffer");
        f
    };

    let png_writer = Arc::new(std::sync::Mutex::new(PNGWriter::new(
        args.output_dir.into(),
        args.width as u32,
        args.height as u32,
    )));

    let processor = DecoderProcessor::new(
        decode_context,
        parser_context,
        scaler,
        scaled_avframe,
        png_writer.clone(),
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
