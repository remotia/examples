use std::time::Duration;

use clap::Parser;
use image::buffer;
use remotia::profilation::loggers::console::ConsoleAverageStatsLogger;
use remotia::profilation::time::diff::TimestampDiffCalculator;
use remotia::{
    buffers::pool_registry::PoolRegistry,
    capture::scrap::ScrapFrameCapturer,
    pipeline::{component::Component, registry::PipelineRegistry, Pipeline},
    processors::{error_switch::OnErrorSwitch, functional::Function, ticker::Ticker},
    profilation::time::add::TimestampAdder,
};
use remotia_ffmpeg_codecs::encoders::fillers::rgba::RGBAFrameFiller;
use remotia_ffmpeg_codecs::options::Options;
use remotia_ffmpeg_codecs::{encoders::EncoderBuilder, ffi, scaling::ScalerBuilder};
use remotia_srt::{options::ByteCount, sender::SRTFrameSender, SrtSocket};
use screen_stream::capturers::scap::ScapFrameCapturer;
use screen_stream::types::{BufferType::*, FrameData, Stat::*};

use remotia::register;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 60)]
    framerate: u32,

    #[arg(long, default_value_t=String::from(":9000"))]
    listen_address: String,

    #[arg(long, default_value_t=String::from("libx264"))]
    codec_id: String,

    #[arg(long)]
    stream_width: Option<u32>,

    #[arg(long)]
    stream_height: Option<u32>,

    #[arg(long)]
    force_capture_width: Option<u32>,

    #[arg(long)]
    force_capture_height: Option<u32>,

    #[arg(id = "codec-option", long)]
    codec_options: Vec<String>,
}

#[derive(PartialEq, Eq, Hash)]
enum Pipelines {
    Main,
    Error,
}

const POOLS_SIZE: usize = 1;

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Hello World!");

    let args = Args::parse();
    let mut capturer = ScapFrameCapturer::new_from_primary(args.framerate, CapturedRGBAFrameBuffer);

    log::info!("{:?}", capturer.capturer().get_output_frame_size());

    let capturer_resolution = capturer.resolution();

    let (width, height) = (
        args.force_capture_width.unwrap_or(capturer_resolution.0),
        args.force_capture_height.unwrap_or(capturer_resolution.1),
    );

    log::info!("Streaming at {}x{}", width, height);

    let stream_width = args.stream_width.unwrap_or(width);
    let stream_height = args.stream_height.unwrap_or(height);

    let mut pools = PoolRegistry::new();
    let pixels_count = (width * height) as usize;
    pools
        .register(CapturedRGBAFrameBuffer, POOLS_SIZE, pixels_count * 4)
        .await;
    pools
        .register(EncodedPacketBuffer, POOLS_SIZE, pixels_count * 4)
        .await;

    log::info!("{:?}", args.codec_options);
    let mut options = Options::new();
    for option in args.codec_options {
        let mut fragments = option.split(" ");
        let (key, value) = (fragments.next().unwrap(), fragments.next().unwrap());
        options = options.set(key, value);
    }
    let (encoder_pusher, encoder_puller) = EncoderBuilder::new()
        .codec_id(&args.codec_id)
        .filler(RGBAFrameFiller::new(CapturedRGBAFrameBuffer))
        .scaler(
            ScalerBuilder::new()
                .input_width(width as i32)
                .input_height(height as i32)
                .output_width(stream_width as i32)
                .output_height(stream_height as i32)
                .input_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_RGBA)
                .output_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_YUV420P)
                .build(),
        )
        .options(options)
        .build();

    let mut pipelines = PipelineRegistry::<FrameData, Pipelines>::new();

    register!(
        pipelines,
        Pipelines::Error,
        Pipeline::<FrameData>::singleton(
            Component::new()
                .append(Function::new(|fd| {
                    log::warn!("Dropped frame");
                    Some(fd)
                }))
                .append(pools.get(CapturedRGBAFrameBuffer).redeemer().soft())
                .append(pools.get(EncodedPacketBuffer).redeemer().soft()),
        )
        .feedable()
    );

    log::info!("Waiting for connection...");
    let socket = SrtSocket::builder()
        .latency(Duration::from_millis(50))
        .set(|options| options.sender.buffer_size = ByteCount(1024 * 1024))
        .listen_on(args.listen_address.as_str())
        .await
        .unwrap();

    register!(
        pipelines,
        Pipelines::Main,
        Pipeline::<FrameData>::new()
            .link(
                Component::new()
                    .append(Ticker::new(1000 / args.framerate as u64))
                    .append(pools.get(CapturedRGBAFrameBuffer).borrower())
                    .append(TimestampAdder::new(CaptureTime))
                    .append(capturer)
                    .append(TimestampAdder::new(EncodePushTime))
                    .append(encoder_pusher),
            )
            .link(
                Component::new()
                    .append(pools.get(CapturedRGBAFrameBuffer).redeemer())
                    .append(pools.get(EncodedPacketBuffer).borrower())
                    .append(encoder_puller)
                    .append(TimestampDiffCalculator::new(EncodePushTime, EncodeTime))
                    .append(OnErrorSwitch::new(pipelines.get_mut(&Pipelines::Error))),
            )
            .link(
                Component::new()
                    .append(TimestampAdder::new(TransmissionStartTime))
                    .append(SRTFrameSender::from_socket(socket))
                    .append(pools.get(EncodedPacketBuffer).redeemer())
                    .append(TimestampDiffCalculator::new(
                        TransmissionStartTime,
                        TransmissionTime,
                    ))
            )
            .link(
                Component::new().append(
                    ConsoleAverageStatsLogger::new()
                        .header("Statistics")
                        .log(EncodeTime)
                        .log(TransmissionTime),
                ),
            )
    );

    pipelines.run().await;
}
