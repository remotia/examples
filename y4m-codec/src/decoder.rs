use std::io::Read;

use clap::Parser;
use remotia::buffers::BytesMut;
use remotia::pipeline::{component::Component, Pipeline};
use remotia::traits::{FrameProperties, PullableFrameProperties};
use remotia_ffmpeg_codecs::decoders::DecoderBuilder;
use remotia_ffmpeg_codecs::ffi;
use remotia_ffmpeg_codecs::scaling::ScalerBuilder;

use y4m_codec::processors::png_frame_writer::PNGWriter;
use y4m_codec::{BufferType, FrameData, Stat};

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

    let scaler = ScalerBuilder::new()
        .input_width(args.width as i32)
        .input_height(args.height as i32)
        .input_pixel_format(ffi::AV_PIX_FMT_YUV420P)
        .output_width(args.width as i32)
        .output_height(args.height as i32)
        .output_pixel_format(ffi::AV_PIX_FMT_RGBA)
        .build();

    let mut pipeline = Pipeline::<FrameData>::new()
        .tag("decoder")
        .feedable();

    let pipeline_handle = pipeline.get_handle();

    let (pusher, puller) = DecoderBuilder::new()
        .codec_id("h264")
        .scaler(scaler)
        .pipeline_handle(pipeline_handle)
        .build();

    let png_writer = PNGWriter::new(
        args.output_dir.into(),
        args.width as u32,
        args.height as u32,
    );

    let mut pipeline = pipeline
        .link(
            Component::new()
                .append(pusher)
                .tag("pusher"),
        )
        .link(
            Component::new()
                .append(puller)
                .append(png_writer)
                .tag("puller-writer"),
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

    log::info!("Decoding complete");
}
