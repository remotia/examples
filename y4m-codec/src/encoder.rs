//! Y4M-to-encoded-video encoder using a self-contained remotia pipeline.
//!
//! Pipeline layout:
//!
//! ```text
//! [Y4MRGBAFrameCapturer → EncoderPusher] → [EncoderPuller → PacketWriter]
//!  Component "capturer-pusher"            Component "puller-writer"
//! ```
//!
//! The first component is headless (no external feeder): it generates a
//! [`FrameData`] on each tick, which the capturer fills with RGBA pixels read
//! from the Y4M source and converts from YUV420p. The encoder pusher/puller
//! pair handles FFmpeg encoding, and the packet writer persists the output.

use std::sync::Arc;

use clap::Parser;
use remotia::capture::y4m::Y4MRGBAFrameCapturer;
use remotia::pipeline::{component::Component, Pipeline};
use remotia_ffmpeg_codecs::encoders::EncoderBuilder;
use remotia_ffmpeg_codecs::encoders::fillers::rgba::RGBAFrameFiller;
use remotia_ffmpeg_codecs::scaling::ScalerBuilder;
use remotia_ffmpeg_codecs::options::Options;
use remotia_ffmpeg_codecs::ffi;

use y4m_codec::processors::packet_writer::PacketWriter;
use y4m_codec::{BufferType, FrameData, Stat};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    output: String,

    #[arg(short, long, default_value = "libx264")]
    codec: String,

    #[arg(short = 'O', long = "option", value_name = "KEY=VALUE")]
    codec_options: Vec<String>,

    #[arg(short = 'n', long = "frames")]
    max_frames: Option<u64>,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = Args::parse();
    log::info!("Encoding {} -> {}", args.input, args.output);

    let y4m_file = std::fs::File::open(&args.input).expect("Unable to open Y4M file");
    let y4m_reader = y4m::decode(y4m_file).expect("Unable to parse Y4M file");
    let width = y4m_reader.get_width();
    let height = y4m_reader.get_height();
    log::info!("Video dimensions: {}x{}", width, height);

    let scaler = ScalerBuilder::new()
        .input_width(width as i32)
        .input_height(height as i32)
        .input_pixel_format(ffi::AV_PIX_FMT_RGBA)
        .output_width(width as i32)
        .output_height(height as i32)
        .output_pixel_format(ffi::AV_PIX_FMT_YUV420P)
        .build();

    let mut options = Options::new();
    for opt in &args.codec_options {
        let (key, value) = opt.split_once('=').unwrap_or_else(|| {
            panic!("Invalid codec option '{}': expected KEY=VALUE", opt)
        });
        options = options.set(key.trim(), value.trim());
    }

    let (encoder_pusher, encoder_puller) = EncoderBuilder::new()
        .codec_id(&args.codec)
        .filler(RGBAFrameFiller::new(BufferType::RgbaFrame))
        .scaler(scaler)
        .options(options)
        .build();

    let output_file = Arc::new(std::sync::Mutex::new(
        std::fs::File::create(&args.output).expect("Unable to create output file"),
    ));

    let mut pipeline = Pipeline::<FrameData>::new().tag("encoder");

    let pipeline_handle = pipeline.get_handle();

    let y4m_capturer = Y4MRGBAFrameCapturer::from_decoder(
        y4m_reader,
        BufferType::RgbaFrame,
        Stat::Eof,
        Stat::FrameId,
        args.max_frames,
        pipeline_handle,
    );

    let pipeline = pipeline
        .link(
            Component::new()
                .append(y4m_capturer)
                .append(encoder_pusher)
                .tag("capturer-pusher"),
        )
        .link(
            Component::new()
                .append(encoder_puller)
                .append(PacketWriter::new(output_file.clone()))
                .tag("puller-writer"),
        );

    let handles = pipeline.run();

    for handle in handles {
        handle.await.unwrap();
    }

    let file_size = output_file.lock().unwrap().metadata().unwrap().len();
    log::info!("Encoding complete: {} bytes written", file_size);
}
