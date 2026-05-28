//! Encoded-video-to-PNG decoder using a self-contained remotia pipeline.
//!
//! Pipeline layout:
//!
//! ```text
//! [EncodedFileChunkReader → DecoderPusher] → [DecoderPuller → PNGWriter]
//!  Component "reader-pusher"                Component "puller-writer"
//! ```
//!
//! The first component is headless: it reads fixed-size chunks from the encoded
//! bitstream and pushes them into the decoder. The decoder puller drains decoded
//! RGBA frames, and the PNG writer persists each frame to disk.

use clap::Parser;
use remotia::pipeline::{component::Component, Pipeline};
use remotia_ffmpeg_codecs::decoders::DecoderBuilder;
use remotia_ffmpeg_codecs::ffi;
use remotia_ffmpeg_codecs::scaling::ScalerBuilder;

use y4m_codec::processors::encoded_file_chunk_reader::EncodedFileChunkReader;
use y4m_codec::processors::png_frame_writer::PNGWriter;
use y4m_codec::FrameData;

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

    #[arg(short, long, default_value = "h264")]
    codec: String,
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

    let mut pipeline = Pipeline::<FrameData>::new().tag("decoder");

    let pipeline_handle = pipeline.get_handle();

    let (pusher, puller) = DecoderBuilder::new()
        .codec_id(&args.codec)
        .scaler(scaler)
        .pipeline_handle(pipeline_handle.clone())
        .build();

    let encoded_file = std::fs::File::open(&args.input).expect("Unable to open encoded input file");

    let png_writer = PNGWriter::new(
        args.output_dir.into(),
        args.width as u32,
        args.height as u32,
    );

    let pipeline = pipeline
        .link(
            Component::new()
                .append(EncodedFileChunkReader::new(encoded_file, pipeline_handle))
                .append(pusher)
                .tag("reader-pusher"),
        )
        .link(
            Component::new()
                .append(puller)
                .append(png_writer)
                .tag("puller-writer"),
        );

    let handles = pipeline.run();

    for handle in handles {
        handle.await.unwrap();
    }

    log::info!("Decoding complete");
}
