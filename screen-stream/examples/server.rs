use std::time::Duration;

use clap::Parser;
use remotia::profilation::loggers::console::ConsoleAverageStatsLogger;
use remotia::profilation::time::diff::TimestampDiffCalculator;
use remotia::serialization::bincode::BincodeSerializer;
use remotia::{
    buffers::pool_registry::PoolRegistry,
    capture::scrap::ScrapFrameCapturer,
    pipeline::{component::Component, Pipeline},
    processors::{error_switch::OnErrorSwitch, functional::Function, ticker::Ticker},
    profilation::time::add::TimestampAdder,
};
use remotia_ffmpeg_codecs::{
    encoders::EncoderBuilder, ffi, options::Options, scaling::ScalerBuilder,
};
use remotia_srt::{
    sender::SRTFrameSender,
    srt_tokio::{options::ByteCount, SrtSocket},
};
use screen_stream::types::{BufferType::*, FrameData, Stat::*};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 60)]
    framerate: u64,

    #[arg(long, default_value_t=String::from(":9000"))]
    listen_address: String,

    #[arg(long, default_value_t=String::from("libx264"))]
    codec_id: String,

    #[arg(long)]
    stream_width: Option<u32>,

    #[arg(long)]
    stream_height: Option<u32>,

    #[arg(id = "codec-option", long)]
    codec_options: Vec<String>,
}

const POOLS_SIZE: usize = 1;

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Hello World!");

    let args = Args::parse();

    let capturer = ScrapFrameCapturer::new_from_primary(CapturedRGBAFrameBuffer);

    log::info!("Streaming at {}x{}", capturer.width(), capturer.height());

    let width = capturer.width() as u32;
    let height = capturer.height() as u32;

    let stream_width = args.stream_width.unwrap_or(width);
    let stream_height = args.stream_height.unwrap_or(height);

    let mut registry = PoolRegistry::new();
    let pixels_count = (width * height) as usize;
    registry
        .register(CapturedRGBAFrameBuffer, POOLS_SIZE, pixels_count * 4)
        .await;
    registry
        .register(EncodedFrameBuffer, POOLS_SIZE, pixels_count * 4)
        .await;
    registry
        .register(SerializedFrameData, POOLS_SIZE, pixels_count * 4)
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
        .rgba_buffer_key(CapturedRGBAFrameBuffer)
        .encoded_buffer_key(EncodedFrameBuffer)
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

    let mut error_pipeline = Pipeline::<FrameData>::singleton(
        Component::new()
            .append(Function::new(|fd| {
                log::warn!("Dropped frame");
                Some(fd)
            }))
            .append(registry.get(CapturedRGBAFrameBuffer).redeemer().soft())
            .append(registry.get(EncodedFrameBuffer).redeemer().soft()),
    )
    .feedable();

    log::info!("Waiting for connection...");
    let socket = SrtSocket::builder()
        .latency(Duration::from_millis(50))
        .set(|options| options.sender.buffer_size = ByteCount(1024 * 1024))
        .listen_on(args.listen_address.as_str())
        .await
        .unwrap();

    let pipeline = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(Ticker::new(1000 / args.framerate))
                .append(registry.get(CapturedRGBAFrameBuffer).borrower())
                .append(TimestampAdder::new(CaptureTime))
                .append(capturer)
                .append(TimestampAdder::new(EncodePushTime))
                .append(encoder_pusher),
        )
        .link(
            Component::new()
                .append(registry.get(CapturedRGBAFrameBuffer).redeemer())
                .append(registry.get(EncodedFrameBuffer).borrower())
                .append(encoder_puller)
                .append(TimestampDiffCalculator::new(EncodePushTime, EncodeTime))
                .append(OnErrorSwitch::new(&mut error_pipeline)),
        )
        .link(
            Component::new()
                .append(TimestampAdder::new(TransmissionStartTime))
                .append(registry.get(SerializedFrameData).borrower())
                .append(BincodeSerializer::new(SerializedFrameData))
                .append(registry.get(EncodedFrameBuffer).redeemer())
                .append(SRTFrameSender::new(SerializedFrameData, socket))
                .append(TimestampDiffCalculator::new(
                    TransmissionStartTime,
                    TransmissionTime,
                ))
                .append(registry.get(SerializedFrameData).redeemer()),
        )
        .link(
            Component::new().append(
                ConsoleAverageStatsLogger::new()
                    .header("Statistics")
                    .log(EncodeTime)
                    .log(TransmissionTime),
            ),
        );

    let mut handles = Vec::new();
    handles.extend(error_pipeline.run());
    handles.extend(pipeline.run());

    for handle in handles {
        handle.await.unwrap();
    }
}
